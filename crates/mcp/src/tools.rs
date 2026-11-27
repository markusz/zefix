//! What the MCP tools do, independent of the protocol so it can be tested directly.

use serde::Serialize;
use zefix_core::scenario::{Config, Scenario};
use zefix_core::sim;
use zefix_core::stats::{self, ratio};

/// Fast enough for an assistant to compare several variants in one answer.
pub const DEFAULT_ITERATIONS: u64 = 1_000_000;
/// Keeps a single call under a few seconds.
pub const MAX_ITERATIONS: u64 = 10_000_000;
/// Same scenario, same numbers: variants an assistant compares differ only by what changed.
const DEFAULT_SEED: u64 = 42;

/// The README part that documents the scenario format and the query language.
const README: &str = include_str!("../../../README.md");
const REFERENCE_START: &str = "### Config";
const REFERENCE_END: &str = "## Entwicklung";

#[derive(Serialize, Debug)]
pub struct Report {
    pub game: String,
    pub hand: String,
    pub iterations: u64,
    pub seed: u64,
    /// Deals that matched the scenario; every `p` is relative to these.
    pub matching_deals: u64,
    pub attempts: u64,
    pub acceptance: f64,
    pub queries: Vec<QueryReport>,
}

#[derive(Serialize, Debug)]
pub struct QueryReport {
    pub name: String,
    pub p: f64,
    pub ci95: [f64; 2],
    pub hits: u64,
}

struct Loaded {
    scenario: Scenario,
    iterations: Option<u64>,
    seed: Option<u64>,
}

fn load(toml_text: &str) -> Result<Loaded, String> {
    // Parsed here rather than via `Scenario::from_toml` to keep the parser's line and column.
    let cfg: Config = toml::from_str(toml_text).map_err(|e| format!("Config ungültig: {e}"))?;
    let iterations = cfg.iterations;
    let seed = cfg.seed;
    let scenario = Scenario::from_config(cfg).map_err(|e| e.to_string())?;
    Ok(Loaded {
        scenario,
        iterations,
        seed,
    })
}

pub fn reference() -> &'static str {
    let start = README.find(REFERENCE_START).unwrap_or(0);
    let end = README.find(REFERENCE_END).unwrap_or(README.len());
    README.get(start..end).unwrap_or(README)
}

pub fn check(toml_text: &str) -> Result<String, String> {
    let scn = load(toml_text)?.scenario;
    Ok(format!(
        "{}\nHand: {}\nMögliche Hände: {}\nAbfragen: {}",
        scn.game.describe(),
        scn.describe_hand(),
        scn.possible_hands(),
        scn.queries.len()
    ))
}

pub fn simulate(toml_text: &str, iterations: Option<u64>) -> Result<Report, String> {
    let loaded = load(toml_text)?;
    let iterations = iterations
        .or(loaded.iterations)
        .unwrap_or(DEFAULT_ITERATIONS);
    if !(1..=MAX_ITERATIONS).contains(&iterations) {
        return Err(format!(
            "iterations muss zwischen 1 und {MAX_ITERATIONS} liegen, war {iterations}"
        ));
    }
    let seed = loaded.seed.unwrap_or(DEFAULT_SEED);
    let scn = &loaded.scenario;
    let outcome = sim::run(scn, iterations, seed, 0).map_err(|e| e.to_string())?;
    let queries = stats::results(scn, &outcome)
        .into_iter()
        .map(|r| QueryReport {
            name: r.name,
            p: r.estimate.p,
            ci95: [r.estimate.ci95.lo, r.estimate.ci95.hi],
            hits: r.estimate.hits,
        })
        .collect();
    Ok(Report {
        game: scn.game.describe(),
        hand: scn.describe_hand(),
        iterations,
        seed,
        matching_deals: outcome.accepted,
        attempts: outcome.attempts,
        acceptance: ratio(outcome.accepted, outcome.attempts),
        queries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBER: &str = include_str!("../../../scenarios/ober.toml");
    const SMALL: u64 = 100_000;

    #[test]
    fn reference_covers_config_and_language() {
        let r = reference();
        assert!(r.starts_with(REFERENCE_START));
        assert!(r.contains("## Abfragesprache"));
        assert!(!r.contains(REFERENCE_END));
    }

    #[test]
    fn simulates_a_scenario() {
        let report = simulate(OBER, Some(SMALL)).map_err(|e| e.to_string());
        let Ok(report) = report else {
            panic!("{report:?}")
        };
        assert_eq!(report.iterations, SMALL);
        assert_eq!(report.matching_deals, SMALL);
        assert_eq!(report.queries.len(), 3);
        // CLI over 10M deals: 52,57 %.
        let partner_ober = report.queries[2].p;
        assert!((0.50..0.55).contains(&partner_ober), "{partner_ober}");
    }

    #[test]
    fn same_scenario_same_numbers() {
        let a = simulate(OBER, Some(SMALL)).map(|r| r.queries[0].hits);
        let b = simulate(OBER, Some(SMALL)).map(|r| r.queries[0].hits);
        assert_eq!(a, b);
    }

    #[test]
    fn errors_point_at_the_problem() {
        let typo = OBER.replace("partner.count(trumpf)", "partner.count(trumf)");
        let err = simulate(&typo, Some(SMALL)).err().unwrap_or_default();
        assert!(err.contains("trumf"), "{err}");
        let broken = check("[game]\ntype = ").err().unwrap_or_default();
        assert!(broken.contains("line 2"), "{broken}");
    }

    #[test]
    fn rejects_too_many_iterations() {
        assert!(simulate(OBER, Some(MAX_ITERATIONS + 1)).is_err());
    }
}
