//! Runs simulations and restarts them whenever the model changes. The core `Run` is stepped
//! in small rounds, so results sharpen live and a stop takes effect within one round. On the
//! desktop a background thread steps it; in the browser, which has no threads, each frame does.

#[cfg(target_arch = "wasm32")]
mod frame;
#[cfg(not(target_arch = "wasm32"))]
mod thread;

#[cfg(target_arch = "wasm32")]
use frame::Job;
#[cfg(not(target_arch = "wasm32"))]
use thread::Job;

use crate::model::Spec;
use eframe::egui;
use schafsim_core::scenario::{Scenario, ScenarioError};
use schafsim_core::sim::{Outcome, Run, RunState};
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;

/// Example deals kept per query.
pub const EXAMPLES: usize = 3;

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

/// Results together with the scenario that produced them.
pub struct Shown {
    pub scenario: Arc<Scenario>,
    pub snapshot: Snapshot,
    pub settings: RunSettings,
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
    pub fn sync(&mut self, spec: Spec, settings: RunSettings, ctx: &egui::Context) {
        self.poll();
        let key = RunKey { spec, settings };
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
            (Some(job), Some(shown)) => !Arc::ptr_eq(job.scenario(), &shown.scenario),
            (None, _) => matches!(self.status, Status::Invalid(_)),
            (Some(_), None) => false,
        }
    }

    pub fn settings(&self) -> Option<RunSettings> {
        self.key.as_ref().map(|k| k.settings)
    }

    fn poll(&mut self) {
        let Some(job) = &mut self.job else { return };
        let updates = job.updates();
        let scenario = Arc::clone(job.scenario());
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
                self.status = match Job::start(Arc::new(scn), key.settings, ctx) {
                    Ok(job) => {
                        self.job = Some(job);
                        Status::Running
                    }
                    Err(e) => Status::Failed(e),
                };
            }
            Err(e) => self.status = Status::Invalid(e),
        }
    }
}

/// The run as far as it got; `start` is when it began.
fn snapshot<S: std::borrow::Borrow<Scenario>>(run: &Run<S>, start: Instant) -> Snapshot {
    Snapshot {
        outcome: run.outcome().clone(),
        fraction: run.fraction_done(),
        elapsed: start.elapsed(),
    }
}

/// Turns the result of one step into the update the UI sees.
fn update_after<S: std::borrow::Borrow<Scenario>>(
    run: &Run<S>,
    step: Result<RunState, schafsim_core::sim::SimError>,
    start: Instant,
) -> Update {
    match step {
        Ok(RunState::Running) => Update::Progress(snapshot(run, start)),
        Ok(RunState::Finished) => Update::Finished(snapshot(run, start)),
        Err(e) => Update::Failed(e.to_string()),
    }
}

impl Update {
    fn is_final(&self) -> bool {
        !matches!(self, Update::Progress(_))
    }
}
