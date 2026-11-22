//! Runs simulations on a background thread and restarts them whenever the model changes.
//! The worker steps the core `Run` in rounds, so results sharpen live and a stop takes effect
//! within one round.

use crate::model::{Model, Spec};
use eframe::egui;
use schafsim_core::scenario::{Scenario, ScenarioError};
use schafsim_core::sim::{Outcome, Run, RunState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

/// Example deals kept per query.
pub const EXAMPLES: usize = 3;
/// Chunks per thread and round: small enough for snappy updates and stops.
const CHUNKS_PER_THREAD: u64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunSettings {
    pub iterations: u64,
    pub seed: u64,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub outcome: Outcome,
    pub fraction: f64,
    pub elapsed: Duration,
}

enum Update {
    Progress(Snapshot),
    Finished(Snapshot),
    Failed(String),
}

#[derive(Debug)]
pub enum Status {
    Running,
    Finished,
    Stopped,
    Failed(String),
    Invalid(ScenarioError),
}

struct Job {
    cancel: Arc<AtomicBool>,
    updates: Receiver<Update>,
    scenario: Arc<Scenario>,
}

/// Results together with the scenario that produced them.
pub struct Shown {
    pub scenario: Arc<Scenario>,
    pub snapshot: Snapshot,
    pub settings: RunSettings,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RunKey {
    spec: Spec,
    settings: RunSettings,
}

pub struct Runner {
    key: Option<RunKey>,
    job: Option<Job>,
    pub status: Status,
    /// Latest results; kept while a new run starts so the view does not flicker.
    pub shown: Option<Shown>,
}

impl Runner {
    pub fn new() -> Runner {
        Runner {
            key: None,
            job: None,
            status: Status::Stopped,
            shown: None,
        }
    }

    /// Applies worker updates and restarts if the model or settings changed.
    pub fn sync(&mut self, model: &Model, settings: RunSettings, ctx: &egui::Context) {
        self.poll();
        let key = RunKey {
            spec: model.spec(),
            settings,
        };
        if self.key.as_ref() != Some(&key) {
            self.start(&key, ctx);
            self.key = Some(key);
        }
    }

    pub fn stop(&mut self) {
        if self.job.take().is_some() {
            self.status = Status::Stopped;
        }
    }

    /// Forces a fresh run on the next `sync`.
    pub fn restart(&mut self) {
        self.key = None;
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, Status::Running)
    }

    /// Shown results belong to an older model than the one being edited.
    pub fn is_stale(&self) -> bool {
        match (&self.job, &self.shown) {
            (Some(job), Some(shown)) => !Arc::ptr_eq(&job.scenario, &shown.scenario),
            (None, _) => matches!(self.status, Status::Invalid(_)),
            (Some(_), None) => false,
        }
    }

    pub fn settings(&self) -> Option<RunSettings> {
        self.key.as_ref().map(|k| k.settings)
    }

    fn poll(&mut self) {
        let Some(job) = &self.job else { return };
        let updates: Vec<Update> = job.updates.try_iter().collect();
        let scenario = Arc::clone(&job.scenario);
        let Some(settings) = self.settings() else {
            return;
        };
        for update in updates {
            let show = |snapshot| Shown {
                scenario: Arc::clone(&scenario),
                snapshot,
                settings,
            };
            match update {
                Update::Progress(snap) => self.shown = Some(show(snap)),
                Update::Finished(snap) => {
                    self.shown = Some(show(snap));
                    self.status = Status::Finished;
                    self.job = None;
                }
                Update::Failed(msg) => {
                    self.status = Status::Failed(msg);
                    self.job = None;
                }
            }
        }
    }

    fn start(&mut self, key: &RunKey, ctx: &egui::Context) {
        self.job = None;
        match Scenario::from_config(key.spec.to_config()) {
            Ok(scn) => {
                self.status = match spawn(Arc::new(scn), key.settings, ctx.clone()) {
                    Ok(job) => {
                        self.job = Some(job);
                        Status::Running
                    }
                    Err(e) => Status::Failed(format!("Thread nicht gestartet: {e}")),
                };
            }
            Err(e) => self.status = Status::Invalid(e),
        }
    }
}

fn spawn(scn: Arc<Scenario>, settings: RunSettings, ctx: egui::Context) -> std::io::Result<Job> {
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = channel();
    let flag = Arc::clone(&cancel);
    let worker_scn = Arc::clone(&scn);
    thread::Builder::new()
        .name("simulation".into())
        .spawn(move || work(&worker_scn, settings, &flag, &tx, &ctx))?;
    Ok(Job {
        cancel,
        updates: rx,
        scenario: scn,
    })
}

fn chunks_per_round() -> u64 {
    let threads = thread::available_parallelism().map_or(1, |n| n.get() as u64);
    threads * CHUNKS_PER_THREAD
}

fn work(
    scn: &Scenario,
    settings: RunSettings,
    cancel: &AtomicBool,
    tx: &Sender<Update>,
    ctx: &egui::Context,
) {
    let start = Instant::now();
    let mut run = Run::new(scn, settings.iterations, settings.seed, EXAMPLES);
    let round = chunks_per_round();
    while !cancel.load(Ordering::Relaxed) {
        let update = match run.step(round) {
            Ok(state) => {
                let snap = Snapshot {
                    outcome: run.outcome().clone(),
                    fraction: run.fraction_done(),
                    elapsed: start.elapsed(),
                };
                match state {
                    RunState::Running => Update::Progress(snap),
                    RunState::Finished => Update::Finished(snap),
                }
            }
            Err(e) => Update::Failed(e.to_string()),
        };
        let done = !matches!(update, Update::Progress(_));
        // A closed channel means the UI dropped this job; nothing left to do.
        if tx.send(update).is_err() || done {
            ctx.request_repaint();
            return;
        }
        ctx.request_repaint();
    }
}
