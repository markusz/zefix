//! TOML config → validated, compiled scenario.

use crate::cards::{Card, Cards};
use crate::game::{Game, GameKind, Seat};
use crate::lang::compile::Compiler;
use crate::lang::ir::BoolExpr;
use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

pub const HAND_SIZE: u32 = 8;
pub const DEFAULT_ITERATIONS: u64 = 10_000_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub iterations: Option<u64>,
    pub seed: Option<u64>,
    pub game: GameKind,
    pub table: Option<TableConfig>,
    #[serde(default)]
    pub me: MeConfig,
    #[serde(default, rename = "query")]
    pub queries: Vec<QueryConfig>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableConfig {
    pub vorhand: Seat,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct MeConfig {
    #[serde(default)]
    pub cards: Vec<String>,
    #[serde(default)]
    pub slots: Vec<SlotConfig>,
    #[serde(rename = "where")]
    pub filter: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotConfig {
    pub from: String,
    pub count: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryConfig {
    pub name: String,
    #[serde(rename = "where")]
    pub expr: String,
}

/// Draw exactly `count` cards from `cards`.
#[derive(Clone, Debug)]
pub struct Slot {
    pub label: String,
    pub cards: Cards,
    pub count: u32,
}

pub struct Query {
    pub name: String,
    pub expr: BoolExpr,
}

pub struct Scenario {
    pub game: Game,
    pub fixed: Cards,
    pub slots: Vec<Slot>,
    pub filter: Option<BoolExpr>,
    pub queries: Vec<Query>,
    pub iterations: u64,
    pub seed: Option<u64>,
}

impl Scenario {
    pub fn from_toml(text: &str) -> Result<Scenario> {
        let cfg: Config = toml::from_str(text).context("Config ungültig")?;
        Scenario::from_config(cfg)
    }

    pub fn from_config(cfg: Config) -> Result<Scenario> {
        let game = Game::new(cfg.game).context("[game]")?;
        let mut compiler = Compiler::new(&game, cfg.table.map(|t| t.vorhand));
        let fixed = parse_fixed(&cfg.me.cards, &game)?;
        let slots = compile_slots(&mut compiler, &cfg.me.slots, fixed)?;
        let filter = cfg
            .me
            .filter
            .as_deref()
            .map(|src| compile_expr(&mut compiler, src, "[me] where"))
            .transpose()?;
        let queries = compile_queries(&mut compiler, &cfg.queries)?;
        Ok(Scenario {
            game,
            fixed,
            slots,
            filter,
            queries,
            iterations: cfg.iterations.unwrap_or(DEFAULT_ITERATIONS),
            seed: cfg.seed,
        })
    }

    /// Number of distinct hands the slots can produce, before any filter.
    pub fn possible_hands(&self) -> u128 {
        self.slots
            .iter()
            .map(|s| binomial(s.cards.len() as u128, s.count as u128))
            .product()
    }

    pub fn describe_hand(&self) -> String {
        let mut parts: Vec<String> = self
            .game
            .sorted(self.fixed)
            .iter()
            .map(|c| c.to_string())
            .collect();
        parts.extend(
            self.slots
                .iter()
                .map(|s| format!("{}× [{}]", s.count, s.label)),
        );
        parts.join(" ")
    }
}

fn parse_fixed(names: &[String], game: &Game) -> Result<Cards> {
    let mut fixed = Cards::EMPTY;
    for name in names {
        let card =
            Card::parse(name).ok_or_else(|| anyhow!("[me] cards: unbekannte Karte `{name}`"))?;
        if fixed.contains(card) {
            bail!("[me] cards: `{card}` doppelt");
        }
        fixed |= card.mask();
    }
    if let Some(sau) = game.called_sau()
        && fixed.contains(sau)
    {
        bail!("[me] cards: die gerufene Sau `{sau}` kann nicht auf meiner Hand sein");
    }
    Ok(fixed)
}

/// Slots must be disjoint; missing cards are filled by an implicit `rest` slot.
fn compile_slots(
    compiler: &mut Compiler,
    configs: &[SlotConfig],
    fixed: Cards,
) -> Result<Vec<Slot>> {
    let mut slots: Vec<Slot> = Vec::with_capacity(configs.len() + 1);
    for (i, cfg) in configs.iter().enumerate() {
        let slot =
            compile_slot(compiler, cfg, fixed).with_context(|| format!("[me] slots[{i}]"))?;
        check_disjoint(&slots, &slot)?;
        slots.push(slot);
    }
    let total = slots.iter().map(|s| s.count).sum::<u32>() + fixed.len();
    if total > HAND_SIZE {
        bail!("[me]: feste Karten + Slots ergeben {total} Karten, erlaubt sind {HAND_SIZE}");
    }
    if total < HAND_SIZE {
        let used = slots.iter().fold(fixed, |acc, s| acc | s.cards);
        slots.push(Slot {
            label: "rest".into(),
            cards: Cards::ALL - used,
            count: HAND_SIZE - total,
        });
    }
    Ok(slots)
}

fn compile_slot(compiler: &mut Compiler, cfg: &SlotConfig, fixed: Cards) -> Result<Slot> {
    let cards = compiler
        .compile_const_set(&cfg.from)
        .map_err(|e| anyhow!("from: {}", e.render(&cfg.from)))?
        - fixed;
    if cfg.count > cards.len() {
        bail!(
            "`{}` hat ohne feste Karten nur {} Karten, count = {}",
            cfg.from,
            cards.len(),
            cfg.count
        );
    }
    Ok(Slot {
        label: cfg.from.clone(),
        cards,
        count: cfg.count,
    })
}

fn check_disjoint(existing: &[Slot], slot: &Slot) -> Result<()> {
    let Some(other) = existing.iter().find(|s| s.cards.intersects(slot.cards)) else {
        return Ok(());
    };
    let shared: Vec<String> = (other.cards & slot.cards)
        .iter()
        .map(|c| c.to_string())
        .collect();
    bail!(
        "[me] Slot `{}` überschneidet sich mit `{}` ({}); Slots müssen disjunkt sein, Rest über `[me] where`",
        slot.label,
        other.label,
        shared.join(" ")
    )
}

fn compile_expr(compiler: &mut Compiler, src: &str, context: &str) -> Result<BoolExpr> {
    compiler
        .compile_bool(src)
        .map_err(|e| anyhow!("{context}: {}", e.render(src)))
}

fn compile_queries(compiler: &mut Compiler, configs: &[QueryConfig]) -> Result<Vec<Query>> {
    if configs.is_empty() {
        bail!("keine [[query]] definiert");
    }
    configs
        .iter()
        .map(|q| {
            Ok(Query {
                name: q.name.clone(),
                expr: compile_expr(compiler, &q.expr, &format!("query \"{}\"", q.name))?,
            })
        })
        .collect()
}

fn binomial(n: u128, k: u128) -> u128 {
    (0..k).fold(1, |acc, i| acc * (n - i) / (i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"
        [game]
        type = "sauspiel"
        called = "schellen"

        [[query]]
        name = "x"
        where = "me.has(EO)"
    "#;

    fn with_me(me: &str) -> Result<Scenario> {
        Scenario::from_toml(&format!("{BASE}\n[me]\n{me}"))
    }

    #[test]
    fn slots_and_rest() {
        let s = with_me(
            r#"cards = ["EO", "GO"]
            slots = [{ from = "unter", count = 2 }]"#,
        )
        .unwrap_or_else(|e| panic!("{e:#}"));
        let counts: Vec<(u32, u32)> = s.slots.iter().map(|s| (s.count, s.cards.len())).collect();
        assert_eq!(counts, [(2, 4), (4, 26)]);
        assert_eq!(s.possible_hands(), 6 * binomial(26, 4));
    }

    #[test]
    fn rejects_bad_hands() {
        let e = |me: &str| {
            with_me(me)
                .err()
                .map(|e| format!("{e:#}"))
                .unwrap_or_default()
        };
        assert!(e(r#"cards = ["EO", "EO"]"#).contains("doppelt"));
        assert!(e(r#"cards = ["SA"]"#).contains("gerufene Sau"));
        assert!(
            e(r#"slots = [{ from = "trumpf", count = 2 }, { from = "ober", count = 1 }]"#)
                .contains("überschneidet")
        );
        assert!(e(r#"slots = [{ from = "ober", count = 5 }]"#).contains("nur 4 Karten"));
        assert!(e(r#"slots = [{ from = "trumpf", count = 9 }]"#).contains("9 Karten"));
    }

    #[test]
    fn empty_me_is_random_hand() {
        let s = with_me("").unwrap_or_else(|e| panic!("{e:#}"));
        assert_eq!(s.possible_hands(), binomial(32, 8));
    }
}
