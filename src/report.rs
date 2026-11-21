use crate::game::Seat;
use crate::lang::eval::Deal;
use crate::scenario::Scenario;
use crate::sim::Outcome;
use comfy_table::{CellAlignment, Table, presets};
use serde_json::json;
use std::time::Duration;

/// Normal quantile for a two-sided 95 % interval.
const Z_95: f64 = 1.959_964;
const PERCENT: f64 = 100.0;

pub struct RunInfo {
    pub seed: u64,
    pub elapsed: Duration,
}

#[derive(Clone, Copy, Debug)]
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

fn ratio(hits: u64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { hits as f64 / n as f64 }
}

fn de_decimal(x: f64, digits: usize) -> String {
    format!("{x:.digits$}").replace('.', ",")
}

fn de_int(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

pub fn text(scn: &Scenario, o: &Outcome, info: &RunInfo) -> String {
    let mut out = header(scn, o, info);
    out.push('\n');
    let rows: Vec<Vec<String>> = scn
        .queries
        .iter()
        .zip(&o.tallies)
        .map(|(q, tally)| query_row(&q.name, tally.hits, o.accepted))
        .collect();
    out += &table(rows);
    for (q, tally) in scn.queries.iter().zip(&o.tallies) {
        for deal in &tally.examples {
            out += &format!("\n{}\n{}", q.name, format_deal(scn, deal));
        }
    }
    out
}

fn header(scn: &Scenario, o: &Outcome, info: &RunInfo) -> String {
    let mut out = format!(
        "{} · {} Deals · Seed {} · {} s\n",
        scn.game.describe(),
        de_int(o.accepted),
        info.seed,
        de_decimal(info.elapsed.as_secs_f64(), 2)
    );
    if o.attempts != o.accepted {
        out += &format!(
            "Akzeptanz {} % ({} Versuche)\n",
            de_decimal(PERCENT * ratio(o.accepted, o.attempts), 2),
            de_int(o.attempts)
        );
    }
    out
}

const HEADERS: [&str; 4] = ["Abfrage", "P", "±95 %", "Treffer"];
const NUMERIC_COLUMNS: std::ops::Range<usize> = 1..4;

fn query_row(name: &str, hits: u64, n: u64) -> Vec<String> {
    let p = ratio(hits, n);
    let ci = wilson(hits, n);
    let half = (p - ci.lo).max(ci.hi - p);
    vec![
        name.to_string(),
        format!("{} %", de_decimal(PERCENT * p, 2)),
        de_decimal(PERCENT * half, 2),
        de_int(hits),
    ]
}

fn table(rows: Vec<Vec<String>>) -> String {
    let mut table = Table::new();
    table
        .load_style(presets::UTF8_FULL_CONDENSED)
        .set_header(HEADERS)
        .add_rows(rows);
    for i in NUMERIC_COLUMNS {
        if let Some(col) = table.column_mut(i) {
            col.set_cell_alignment(CellAlignment::Right);
        }
    }
    format!("{table}\n")
}

pub fn format_deal(scn: &Scenario, deal: &Deal) -> String {
    Seat::ALL
        .iter()
        .map(|&seat| {
            let cards: Vec<String> = scn
                .game
                .sorted(deal.hands[seat.index()])
                .iter()
                .map(|c| c.to_string())
                .collect();
            let tag = if deal.partner == Some(seat) { "*" } else { " " };
            format!("  {:<7}{tag} {}\n", seat.name(), cards.join(" "))
        })
        .collect()
}

pub fn json(scn: &Scenario, o: &Outcome, info: &RunInfo) -> serde_json::Value {
    let queries: Vec<_> = scn
        .queries
        .iter()
        .zip(&o.tallies)
        .map(|(q, tally)| {
            let hits = tally.hits;
            let ci = wilson(hits, o.accepted);
            json!({
                "name": q.name,
                "hits": hits,
                "p": ratio(hits, o.accepted),
                "ci95": [ci.lo, ci.hi],
            })
        })
        .collect();
    json!({
        "game": scn.game.describe(),
        "seed": info.seed,
        "deals": o.accepted,
        "attempts": o.attempts,
        "seconds": info.elapsed.as_secs_f64(),
        "queries": queries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(de_int(1_000_000), "1.000.000");
        assert_eq!(de_int(999), "999");
        assert_eq!(de_decimal(16.4789, 2), "16,48");
    }

    #[test]
    fn wilson_contains_p() {
        let ci = wilson(165, 1000);
        assert!(ci.lo < 0.165 && 0.165 < ci.hi);
        assert!(ci.hi - ci.lo < 0.05);
    }
}
