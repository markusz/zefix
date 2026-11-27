//! Desktop backend: a background thread steps the run and sends updates over a channel.

use super::{EXAMPLES, RunSettings, SEED, Update, update_after};
use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use web_time::Instant;
use zefix_core::scenario::Scenario;
use zefix_core::sim::Run;

/// Chunks per thread and round: small enough for snappy updates and stops.
const CHUNKS_PER_THREAD: u64 = 4;

pub struct Job {
    cancel: Arc<AtomicBool>,
    updates: Receiver<Update>,
    scenario: Arc<Scenario>,
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Job {
    pub fn start(
        scenario: Arc<Scenario>,
        settings: RunSettings,
        ctx: &egui::Context,
    ) -> Result<Job, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let flag = Arc::clone(&cancel);
        let worker_scn = Arc::clone(&scenario);
        let ctx = ctx.clone();
        thread::Builder::new()
            .name("simulation".into())
            .spawn(move || work(&worker_scn, settings, &flag, &tx, &ctx))
            .map_err(|e| format!("Thread nicht gestartet: {e}"))?;
        Ok(Job {
            cancel,
            updates: rx,
            scenario,
        })
    }

    pub fn scenario(&self) -> &Arc<Scenario> {
        &self.scenario
    }

    pub fn updates(&mut self) -> Vec<Update> {
        self.updates.try_iter().collect()
    }
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
    let mut run = Run::new(scn, settings.iterations, SEED, EXAMPLES);
    let round = chunks_per_round();
    while !cancel.load(Ordering::Relaxed) {
        let step = run.step(round);
        let update = update_after(&run, step, start);
        let done = update.is_final();
        // A closed channel means the UI dropped this job; nothing left to do.
        let sent = tx.send(update).is_ok();
        ctx.request_repaint();
        if !sent || done {
            return;
        }
    }
}
