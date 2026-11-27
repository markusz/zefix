//! Browser backend: WebAssembly has no threads, so each frame steps the run for a fixed time
//! budget and asks for the next frame until the run is done.

use super::{EXAMPLES, RunSettings, SEED, Update, update_after};
use eframe::egui;
use std::sync::Arc;
use std::time::Duration;
use web_time::Instant;
use zefix_core::scenario::Scenario;
use zefix_core::sim::Run;

/// Simulation time per frame; leaves room for drawing within a 60 Hz frame.
const FRAME_BUDGET: Duration = Duration::from_millis(10);
/// One chunk at a time, so the budget is checked often.
const CHUNKS_PER_STEP: u64 = 1;

pub struct Job {
    run: Run<Arc<Scenario>>,
    scenario: Arc<Scenario>,
    start: Instant,
    ctx: egui::Context,
}

impl Job {
    pub fn start(
        scenario: Arc<Scenario>,
        settings: RunSettings,
        ctx: &egui::Context,
    ) -> Result<Job, String> {
        let run = Run::new(Arc::clone(&scenario), settings.iterations, SEED, EXAMPLES);
        ctx.request_repaint();
        Ok(Job {
            run,
            scenario,
            start: Instant::now(),
            ctx: ctx.clone(),
        })
    }

    pub fn scenario(&self) -> &Arc<Scenario> {
        &self.scenario
    }

    pub fn updates(&mut self) -> Vec<Update> {
        let deadline = Instant::now() + FRAME_BUDGET;
        loop {
            let step = self.run.step(CHUNKS_PER_STEP);
            let update = update_after(&self.run, step, self.start);
            if update.is_final() {
                return vec![update];
            }
            if Instant::now() >= deadline {
                self.ctx.request_repaint();
                return vec![update];
            }
        }
    }
}
