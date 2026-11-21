//! Point estimates and confidence intervals for query results.

use crate::scenario::Scenario;
use crate::sim::Outcome;

/// Normal quantile for a two-sided 95 % interval.
const Z_95: f64 = 1.959_964;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

/// 95 % Wilson score interval.
pub fn wilson(hits: u64, n: u64) -> Interval {
    if n == 0 {
        return Interval { lo: 0.0, hi: 1.0 };
    }
    let n = n as f64;
    let p = hits as f64 / n;
    let z2 = Z_95 * Z_95;
    let denom = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denom;
    let half = Z_95 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denom;
    Interval {
        lo: (center - half).max(0.0),
        hi: (center + half).min(1.0),
    }
}

pub fn ratio(hits: u64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { hits as f64 / n as f64 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    pub hits: u64,
    pub p: f64,
    pub ci95: Interval,
}

impl Estimate {
    pub fn new(hits: u64, n: u64) -> Estimate {
        Estimate {
            hits,
            p: ratio(hits, n),
            ci95: wilson(hits, n),
        }
    }

    /// Larger distance from `p` to an interval bound; Wilson is asymmetric.
    pub fn margin(&self) -> f64 {
        (self.p - self.ci95.lo).max(self.ci95.hi - self.p)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryResult {
    pub name: String,
    pub estimate: Estimate,
}

/// One result per query, in config order.
pub fn results(scn: &Scenario, outcome: &Outcome) -> Vec<QueryResult> {
    scn.queries
        .iter()
        .zip(&outcome.tallies)
        .map(|(q, tally)| QueryResult {
            name: q.name.clone(),
            estimate: Estimate::new(tally.hits, outcome.accepted),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_contains_p() {
        let ci = wilson(165, 1000);
        assert!(ci.lo < 0.165 && 0.165 < ci.hi);
        assert!(ci.hi - ci.lo < 0.05);
    }

    #[test]
    fn empty_sample_is_uninformative() {
        let e = Estimate::new(0, 0);
        assert_eq!(e.p, 0.0);
        assert_eq!(e.ci95, Interval { lo: 0.0, hi: 1.0 });
    }
}
