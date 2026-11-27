//! Monte Carlo loop. Work is split into fixed-size chunks with their own seeded RNG, so
//! results only depend on the seed, not on thread count. With the `parallel` feature chunks
//! run on rayon; without it (WebAssembly) they run one after another with the same result.

use crate::lang::eval::{Deal, Evaluator};
use crate::sampler::Sampler;
use crate::scenario::Scenario;
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::SeedableRng;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::borrow::Borrow;

const CHUNK: u64 = 1 << 14;
/// Golden-ratio constant; spreads chunk indices across the seed space.
const CHUNK_SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;
/// Give up if a chunk sees this many rejections before its first accepted deal.
const MAX_FUTILE_ATTEMPTS: u64 = 10_000_000;

#[derive(Debug, thiserror::Error)]
pub enum SimError {
    #[error(
        "nach {MAX_FUTILE_ATTEMPTS} Versuchen kein passendes Blatt: Rufregel bzw. `[me] where` praktisch unerfüllbar"
    )]
    Unsatisfiable,
}

#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub hits: u64,
    /// First matching deals, for `--examples`.
    pub examples: Vec<Deal>,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    /// Deals that passed the Rufregel and `[me] where`.
    pub accepted: u64,
    pub attempts: u64,
    /// One per query, in config order.
    pub tallies: Vec<Tally>,
}

impl Outcome {
    fn new(queries: usize) -> Outcome {
        Outcome {
            accepted: 0,
            attempts: 0,
            tallies: vec![Tally::default(); queries],
        }
    }

    fn merge(&mut self, other: Outcome, max_examples: usize) {
        self.accepted += other.accepted;
        self.attempts += other.attempts;
        for (a, b) in self.tallies.iter_mut().zip(other.tallies) {
            a.hits += b.hits;
            let room = max_examples.saturating_sub(a.examples.len());
            a.examples.extend(b.examples.into_iter().take(room));
        }
    }

    fn record(&mut self, scn: &Scenario, deal: &Deal, max_examples: usize) {
        self.accepted += 1;
        let mut ev = Evaluator::new(&scn.game, deal);
        for (tally, query) in self.tallies.iter_mut().zip(&scn.queries) {
            if ev.eval(&query.expr) {
                tally.hits += 1;
                if tally.examples.len() < max_examples {
                    tally.examples.push(*deal);
                }
            }
        }
    }
}

pub fn run(
    scn: &Scenario,
    iterations: u64,
    seed: u64,
    max_examples: usize,
) -> Result<Outcome, SimError> {
    let mut run = Run::new(scn, iterations, seed, max_examples);
    run.step(u64::MAX)?;
    Ok(run.into_outcome())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    Finished,
}

/// A simulation advanced in steps, so front ends can show partial results and stop early.
/// Stepping visits the same chunks with the same seeds as [`run`], so results are identical.
/// `S` is `&Scenario` for a borrowed scenario or e.g. `Arc<Scenario>` for one the run owns.
pub struct Run<S: Borrow<Scenario>> {
    scn: S,
    sampler: Sampler,
    iterations: u64,
    seed: u64,
    max_examples: usize,
    next_chunk: u64,
    outcome: Outcome,
}

impl<S: Borrow<Scenario>> Run<S> {
    pub fn new(scn: S, iterations: u64, seed: u64, max_examples: usize) -> Run<S> {
        let sampler = Sampler::new(scn.borrow());
        let outcome = Outcome::new(scn.borrow().queries.len());
        Run {
            scn,
            sampler,
            iterations,
            seed,
            max_examples,
            next_chunk: 0,
            outcome,
        }
    }

    fn total_chunks(&self) -> u64 {
        self.iterations.div_ceil(CHUNK)
    }

    /// Simulates up to `max_chunks` further chunks, in parallel with the `parallel` feature.
    pub fn step(&mut self, max_chunks: u64) -> Result<RunState, SimError> {
        let total = self.total_chunks();
        let end = self.next_chunk.saturating_add(max_chunks).min(total);
        let (scn, sampler) = (self.scn.borrow(), &self.sampler);
        let (iterations, seed, max_examples) = (self.iterations, self.seed, self.max_examples);
        let simulate = |c: u64| {
            let n = CHUNK.min(iterations - c * CHUNK);
            let chunk_seed = seed ^ c.wrapping_mul(CHUNK_SEED_MIX);
            run_chunk(scn, sampler, n, chunk_seed, max_examples)
        };
        #[cfg(feature = "parallel")]
        let chunks: Vec<Result<Outcome, SimError>> = (self.next_chunk..end)
            .into_par_iter()
            .map(simulate)
            .collect();
        #[cfg(not(feature = "parallel"))]
        let chunks: Vec<Result<Outcome, SimError>> = (self.next_chunk..end).map(simulate).collect();
        for chunk in chunks {
            self.outcome.merge(chunk?, self.max_examples);
        }
        self.next_chunk = end;
        Ok(if end == total {
            RunState::Finished
        } else {
            RunState::Running
        })
    }

    pub fn fraction_done(&self) -> f64 {
        match self.total_chunks() {
            0 => 1.0,
            total => self.next_chunk as f64 / total as f64,
        }
    }

    pub fn outcome(&self) -> &Outcome {
        &self.outcome
    }

    pub fn scenario(&self) -> &Scenario {
        self.scn.borrow()
    }

    pub fn into_outcome(self) -> Outcome {
        self.outcome
    }
}

fn run_chunk(
    scn: &Scenario,
    sampler: &Sampler,
    n: u64,
    seed: u64,
    max_examples: usize,
) -> Result<Outcome, SimError> {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
    let mut out = Outcome::new(scn.queries.len());
    while out.accepted < n {
        out.attempts += 1;
        if out.accepted == 0 && out.attempts > MAX_FUTILE_ATTEMPTS {
            return Err(SimError::Unsatisfiable);
        }
        if let Some(deal) = draw(scn, sampler, &mut rng) {
            out.record(scn, &deal, max_examples);
        }
    }
    Ok(out)
}

/// One deal satisfying Rufregel and `[me] where`; `None` if rejected.
fn draw(scn: &Scenario, sampler: &Sampler, rng: &mut Xoshiro256PlusPlus) -> Option<Deal> {
    let me = sampler.my_hand(rng);
    if !sampler.may_call(me) {
        return None;
    }
    let deal = sampler.deal(rng, me);
    let accepted = scn
        .filter
        .as_ref()
        .is_none_or(|filter| Evaluator::new(&scn.game, &deal).eval(filter));
    accepted.then_some(deal)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIO: &str = r#"
        [game]
        type = "sauspiel"
        called = "gras"

        [[query]]
        name = "x"
        where = "any(p in others, p.count(trumpf) >= 4)"
    "#;

    #[test]
    fn stepping_matches_single_run() {
        let scn = Scenario::from_toml(SCENARIO).unwrap_or_else(|e| panic!("{e}"));
        let (iterations, seed, examples) = (10 * CHUNK + 7, 3, 2);
        let whole = run(&scn, iterations, seed, examples).unwrap_or_else(|e| panic!("{e}"));
        let mut stepped = Run::new(&scn, iterations, seed, examples);
        while stepped.step(3).unwrap_or_else(|e| panic!("{e}")) == RunState::Running {}
        let stepped = stepped.into_outcome();
        assert_eq!(stepped.accepted, iterations);
        assert_eq!(stepped.attempts, whole.attempts);
        assert_eq!(stepped.tallies[0].hits, whole.tallies[0].hits);
        assert_eq!(stepped.tallies[0].examples, whole.tallies[0].examples);
    }
}
