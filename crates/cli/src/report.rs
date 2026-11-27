//! Text and JSON rendering of a simulation run.

use comfy_table::{CellAlignment, Table, presets};
use serde_json::json;
use std::time::Duration;
use zefix_core::game::Seat;
use zefix_core::lang::eval::Deal;
use zefix_core::scenario::Scenario;
use zefix_core::sim::Outcome;
use zefix_core::stats::{self, QueryResult, ratio};

const PERCENT: f64 = 100.0;
const DECIMALS: usize = 2;
const THOUSANDS_GROUP: usize = 3;

pub struct RunInfo {
    pub seed: u64,
    pub elapsed: Duration,
}

fn de_decimal(x: f64) -> String {
    format!("{x:.DECIMALS$}").replace('.', ",")
}

fn de_int(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / THOUSANDS_GROUP);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(THOUSANDS_GROUP) {
            out.push('.');
        }
        out.push(ch);
    }
    out
}

pub fn text(scn: &Scenario, o: &Outcome, info: &RunInfo) -> String {
    let mut out = header(scn, o, info);
    out.push('\n');
    out += &table(&stats::results(scn, o));
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
        de_decimal(info.elapsed.as_secs_f64())
    );
    if o.attempts != o.accepted {
        out += &format!(
            "Akzeptanz {} % ({} Versuche)\n",
            de_decimal(PERCENT * ratio(o.accepted, o.attempts)),
            de_int(o.attempts)
        );
    }
    out
}

const HEADERS: [&str; 4] = ["Abfrage", "P", "±95 %", "Treffer"];
const NUMERIC_COLUMNS: std::ops::Range<usize> = 1..4;

fn query_row(r: &QueryResult) -> Vec<String> {
    let e = &r.estimate;
    vec![
        r.name.clone(),
        format!("{} %", de_decimal(PERCENT * e.p)),
        de_decimal(PERCENT * e.margin()),
        de_int(e.hits),
    ]
}

fn table(results: &[QueryResult]) -> String {
    let mut table = Table::new();
    table
        .load_style(presets::UTF8_FULL_CONDENSED)
        .set_header(HEADERS)
        .add_rows(results.iter().map(query_row));
    for i in NUMERIC_COLUMNS {
        if let Some(col) = table.column_mut(i) {
            col.set_cell_alignment(CellAlignment::Right);
        }
    }
    format!("{table}\n")
}

fn format_deal(scn: &Scenario, deal: &Deal) -> String {
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
    let queries: Vec<_> = stats::results(scn, o)
        .iter()
        .map(|r| {
            let e = &r.estimate;
            json!({
                "name": r.name,
                "hits": e.hits,
                "p": e.p,
                "ci95": [e.ci95.lo, e.ci95.hi],
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
        assert_eq!(de_decimal(16.4789), "16,48");
    }
}
