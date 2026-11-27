//! Simulated probabilities against exact hypergeometric results.

use zefix_core::scenario::Scenario;
use zefix_core::sim;

const ITERATIONS: u64 = 400_000;
const SEED: u64 = 7;
/// Roughly four standard errors at this sample size.
const TOLERANCE: f64 = 0.004;

fn binomial(n: u64, k: u64) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

fn simulate(toml: &str) -> Vec<f64> {
    let scn = Scenario::from_toml(toml).unwrap_or_else(|e| panic!("{e:#}"));
    let out = sim::run(&scn, ITERATIONS, SEED, 0).unwrap_or_else(|e| panic!("{e:#}"));
    assert_eq!(out.accepted, ITERATIONS);
    out.tallies
        .iter()
        .map(|t| t.hits as f64 / out.accepted as f64)
        .collect()
}

fn assert_close(got: f64, want: f64) {
    assert!(
        (got - want).abs() < TOLERANCE,
        "simuliert {got:.5}, exakt {want:.5}"
    );
}

#[test]
fn both_missing_ober_plus_two_trumps_at_one_opponent() {
    // Herz-Solo has the same trumps as a Sauspiel but no Rufregel.
    let p = simulate(
        r#"
        [game]
        type = "solo"
        trump = "herz"

        [me]
        cards = ["EO", "GO"]
        slots = [
          { from = "unter", count = 2 },
          { from = "herz", count = 2 },
          { from = "farbe - sau", count = 2 },
        ]

        [[query]]
        name = "q"
        where = "any(p in others, p.has(HO, SO) and p.count(trumpf - ober) >= 2)"
        "#,
    );
    // Both Ober at one player: 3 · 8/24 · 7/23. Then ≥2 of the 6 outstanding
    // trumps among that player's other 6 cards out of 22.
    let both = 3.0 * 8.0 / 24.0 * 7.0 / 23.0;
    let total = binomial(22, 6);
    let at_most_one = (binomial(16, 6) + 6.0 * binomial(16, 5)) / total;
    assert_close(p[0], both * (1.0 - at_most_one));
}

#[test]
fn single_card_location_and_voids() {
    let p = simulate(
        r#"
        [game]
        type = "wenz"

        [me]
        cards = ["EU", "GU", "EA", "E10", "GA", "G10", "HA", "SA"]

        [[query]]
        name = "left hat HU"
        where = "left.has(HU)"

        [[query]]
        name = "irgendwer frei in Eichel"
        where = "any(p in others, p.frei(eichel))"

        [[query]]
        name = "num == 3 gleich all"
        where = "num(p in others, p.count(alle) == 8) == 3"
        "#,
    );
    assert_close(p[0], 8.0 / 24.0);
    // 5 Eichel outstanding (EO EK E9 E8 E7), 19 other cards. Inclusion–exclusion;
    // three players cannot all be void.
    let p_void_left = binomial(19, 8) / binomial(24, 8);
    let p_two_void = p_void_left * binomial(11, 8) / binomial(16, 8);
    assert_close(p[1], 3.0 * p_void_left - 3.0 * p_two_void);
    assert_close(p[2], 1.0);
}

#[test]
fn rufregel_is_enforced() {
    let p = simulate(
        r#"
        [game]
        type = "sauspiel"
        called = "gras"

        [[query]]
        name = "ich habe Gras"
        where = "me.has_any(gras) and not me.has(GA)"

        [[query]]
        name = "partner hat GA"
        where = "partner.has(GA)"
        "#,
    );
    assert_close(p[0], 1.0);
    assert_close(p[1], 1.0);
}

#[test]
fn deterministic_for_seed() {
    let toml = r#"
        [game]
        type = "geier"
        [[query]]
        name = "q"
        where = "me.count(trumpf) >= 2"
    "#;
    assert_eq!(simulate(toml), simulate(toml));
}
