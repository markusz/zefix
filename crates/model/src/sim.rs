//! A simulation the browser advances a few chunks at a time, so a worker can report partial
//! results and take a new model between steps.

use crate::compile::Problem;
use crate::model::Model;
use serde::Serialize;
use zefix_core::game::Seat;
use zefix_core::lang::eval::Deal;
use zefix_core::scenario::Scenario;
use zefix_core::sim::{Run, RunState};
use zefix_core::stats;

/// Example deals kept per query.
const EXAMPLES: usize = 3;
/// Not user-facing. Fixed so the same model always gives the same numbers: variants differ
/// only by what changed, not by sampling noise, and undoing an edit restores the old result.
pub const SEED: u64 = 42;

pub struct Simulation {
    run: Run<Scenario>,
    finished: bool,
}

#[derive(Serialize)]
pub struct Report {
    pub finished: bool,
    pub fraction: f64,
    pub accepted: u64,
    pub attempts: u64,
    pub results: Vec<ResultReport>,
}

#[derive(Serialize)]
pub struct ResultReport {
    pub name: String,
    pub p: f64,
    /// Half width of the 95 % interval.
    pub margin: f64,
    pub hits: u64,
    pub examples: Vec<DealReport>,
}

#[derive(Serialize)]
pub struct DealReport {
    /// Indexed like `Seat::ALL`, each hand in game order.
    pub hands: Vec<Vec<String>>,
    pub partner: Option<Seat>,
}

impl Simulation {
    pub fn new(model: &Model, iterations: u64) -> Result<Simulation, Problem> {
        let game = model.game.game().ok_or_else(|| Problem::InvalidGame {
            message: "Ungültiges Spiel.".into(),
        })?;
        let scenario =
            Scenario::from_config(model.spec(&game).to_config()).map_err(|e| Problem::of(&e))?;
        Ok(Simulation {
            run: Run::new(scenario, iterations, SEED, EXAMPLES),
            finished: false,
        })
    }

    /// Simulates up to `chunks` more chunks; `Err` if no deal fits the scenario.
    pub fn advance(&mut self, chunks: u64) -> Result<bool, String> {
        if !self.finished {
            let state = self.run.step(chunks).map_err(|e| e.to_string())?;
            self.finished = state == RunState::Finished;
        }
        Ok(self.finished)
    }

    pub fn report(&self) -> Report {
        let outcome = self.run.outcome();
        let scenario = self.run.scenario();
        let results = stats::results(scenario, outcome)
            .into_iter()
            .zip(&outcome.tallies)
            .map(|(result, tally)| ResultReport {
                name: result.name,
                p: result.estimate.p,
                margin: result.estimate.margin(),
                hits: result.estimate.hits,
                examples: tally
                    .examples
                    .iter()
                    .map(|deal| deal_report(scenario, deal))
                    .collect(),
            })
            .collect();
        Report {
            finished: self.finished,
            fraction: self.run.fraction_done(),
            accepted: outcome.accepted,
            attempts: outcome.attempts,
            results,
        }
    }
}

fn deal_report(scenario: &Scenario, deal: &Deal) -> DealReport {
    DealReport {
        hands: deal
            .hands
            .iter()
            .map(|&hand| {
                let sorted = scenario.game.sorted(hand);
                sorted.iter().map(|c| c.to_string()).collect()
            })
            .collect(),
        partner: deal.partner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stepping in the browser must give the numbers the CLI gives for the same seed.
    #[test]
    fn stepped_run_matches_core() {
        let model = Model::default();
        let iterations = 50_000;
        let mut sim = Simulation::new(&model, iterations).unwrap_or_else(|e| panic!("{e:?}"));
        while !sim.advance(1).unwrap_or_else(|e| panic!("{e}")) {}
        let report = sim.report();
        let game = model.game.game().unwrap_or_else(|| panic!("game"));
        let scenario =
            Scenario::from_config(model.spec(&game).to_config()).unwrap_or_else(|e| panic!("{e}"));
        let outcome = zefix_core::sim::run(&scenario, iterations, SEED, EXAMPLES)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.finished);
        assert_eq!(report.accepted, outcome.accepted);
        assert_eq!(report.results[0].hits, outcome.tallies[0].hits);
        assert_eq!(report.results[0].examples.len(), EXAMPLES);
        assert_eq!(report.results[0].examples[0].hands[0].len(), 8);
    }
}
