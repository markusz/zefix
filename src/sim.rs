//! Parallel Monte Carlo loop. Work is split into fixed-size chunks with their
//! own seeded RNG, so results only depend on the seed, not on thread count.

use crate::lang::eval::{Deal, Evaluator};
use crate::sampler::Sampler;
use crate::scenario::Scenario;
use anyhow::{Result, bail};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::SeedableRng;
use rayon::prelude::*;

const CHUNK: u64 = 1 << 14;
/// Golden-ratio constant; spreads chunk indices across the seed space.
const CHUNK_SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;
/// Give up if a chunk sees this many rejections before its first accepted deal.
const MAX_FUTILE_ATTEMPTS: u64 = 10_000_000;

#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub hits: u64,
    /// First matching deals, for `--examples`.
    pub examples: Vec<Deal>,
}

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

    fn merge(mut self, other: Outcome, max_examples: usize) -> Outcome {
        self.accepted += other.accepted;
        self.attempts += other.attempts;
        for (a, b) in self.tallies.iter_mut().zip(other.tallies) {
            a.hits += b.hits;
            let room = max_examples.saturating_sub(a.examples.len());
            a.examples.extend(b.examples.into_iter().take(room));
        }
        self
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

pub fn run(scn: &Scenario, iterations: u64, seed: u64, max_examples: usize) -> Result<Outcome> {
    let sampler = Sampler::new(scn);
    let chunks: Vec<Result<Outcome>> = (0..iterations.div_ceil(CHUNK))
        .into_par_iter()
        .map(|c| {
            let n = CHUNK.min(iterations - c * CHUNK);
            let chunk_seed = seed ^ c.wrapping_mul(CHUNK_SEED_MIX);
            run_chunk(scn, &sampler, n, chunk_seed, max_examples)
        })
        .collect();
    chunks
        .into_iter()
        .try_fold(Outcome::new(scn.queries.len()), |acc, chunk| {
            Ok(acc.merge(chunk?, max_examples))
        })
}

fn run_chunk(
    scn: &Scenario,
    sampler: &Sampler,
    n: u64,
    seed: u64,
    max_examples: usize,
) -> Result<Outcome> {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(seed);
    let mut out = Outcome::new(scn.queries.len());
    while out.accepted < n {
        out.attempts += 1;
        if out.accepted == 0 && out.attempts > MAX_FUTILE_ATTEMPTS {
            bail!(
                "nach {MAX_FUTILE_ATTEMPTS} Versuchen kein passendes Blatt: Rufregel bzw. `[me] where` praktisch unerfüllbar"
            );
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
