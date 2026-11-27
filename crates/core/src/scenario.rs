//! TOML config → validated, compiled scenario.

use crate::cards::{Card, Cards};
use crate::game::{Game, GameError, GameKind, Seat};
use crate::lang::LangError;
use crate::lang::compile::Compiler;
use crate::lang::ir::BoolExpr;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const HAND_SIZE: u32 = 8;
pub const DEFAULT_ITERATIONS: u64 = 10_000_000;

/// Where a query-language expression sits in the config.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprSite {
    SlotFrom { index: usize },
    MeWhere,
    Query { name: String },
}

impl fmt::Display for ExprSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprSite::SlotFrom { index } => write!(f, "[me] slots[{index}] from"),
            ExprSite::MeWhere => f.write_str("[me] where"),
            ExprSite::Query { name } => write!(f, "query \"{name}\""),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("Config ungültig")]
    Toml(#[from] toml::de::Error),
    #[error("[game]")]
    Game(#[from] GameError),
    #[error("[me] cards: unbekannte Karte `{0}`")]
    UnknownCard(String),
    #[error("[me] cards: `{0}` doppelt")]
    DuplicateCard(Card),
    #[error("[me] cards: die gerufene Sau `{0}` kann nicht auf meiner Hand sein")]
    CalledSauInHand(Card),
    #[error(
        "[me] slots[{index}]: `{from}` hat ohne feste Karten nur {available} Karten, count = {count}"
    )]
    SlotTooSmall {
        index: usize,
        from: String,
        available: u32,
        count: u32,
    },
    #[error(
        "[me] Slot `{slot}` überschneidet sich mit `{other}` ({shared}); Slots müssen disjunkt sein, Rest über `[me] where`"
    )]
    SlotOverlap {
        slot: String,
        other: String,
        shared: Cards,
    },
    #[error("[me]: feste Karten + Slots ergeben {total} Karten, erlaubt sind {HAND_SIZE}")]
    TooManyCards { total: u32 },
    #[error("keine [[query]] definiert")]
    NoQueries,
    /// `error.span` indexes into `src`, so front ends can highlight it.
    #[error("{site}: {}", error.render(src))]
    Expr {
        site: ExprSite,
        src: String,
        error: LangError,
    },
}

type Result<T> = std::result::Result<T, ScenarioError>;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    pub game: GameKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<TableConfig>,
    #[serde(default)]
    pub me: MeConfig,
    #[serde(default, rename = "query")]
    pub queries: Vec<QueryConfig>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TableConfig {
    pub vorhand: Seat,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct MeConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<SlotConfig>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SlotConfig {
    pub from: String,
    pub count: u32,
}

#[derive(Deserialize, Serialize)]
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
        let cfg: Config = toml::from_str(text)?;
        Scenario::from_config(cfg)
    }

    pub fn from_config(cfg: Config) -> Result<Scenario> {
        let game = Game::new(cfg.game)?;
        let mut compiler = Compiler::new(&game, cfg.table.map(|t| t.vorhand));
        let fixed = parse_fixed(&cfg.me.cards, &game)?;
        let slots = compile_slots(&mut compiler, &cfg.me.slots, fixed)?;
        let filter = cfg
            .me
            .filter
            .as_deref()
            .map(|src| compile_expr(&mut compiler, src, ExprSite::MeWhere))
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
        let card = Card::parse(name).ok_or_else(|| ScenarioError::UnknownCard(name.clone()))?;
        if fixed.contains(card) {
            return Err(ScenarioError::DuplicateCard(card));
        }
        fixed |= card.mask();
    }
    if let Some(sau) = game.called_sau()
        && fixed.contains(sau)
    {
        return Err(ScenarioError::CalledSauInHand(sau));
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
        let slot = compile_slot(compiler, cfg, i, fixed)?;
        check_disjoint(&slots, &slot)?;
        slots.push(slot);
    }
    let total = slots.iter().map(|s| s.count).sum::<u32>() + fixed.len();
    if total > HAND_SIZE {
        return Err(ScenarioError::TooManyCards { total });
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

fn compile_slot(
    compiler: &mut Compiler,
    cfg: &SlotConfig,
    index: usize,
    fixed: Cards,
) -> Result<Slot> {
    let cards = compiler
        .compile_const_set(&cfg.from)
        .map_err(|error| ScenarioError::Expr {
            site: ExprSite::SlotFrom { index },
            src: cfg.from.clone(),
            error,
        })?
        - fixed;
    if cfg.count > cards.len() {
        return Err(ScenarioError::SlotTooSmall {
            index,
            from: cfg.from.clone(),
            available: cards.len(),
            count: cfg.count,
        });
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
    Err(ScenarioError::SlotOverlap {
        slot: slot.label.clone(),
        other: other.label.clone(),
        shared: other.cards & slot.cards,
    })
}

fn compile_expr(compiler: &mut Compiler, src: &str, site: ExprSite) -> Result<BoolExpr> {
    compiler
        .compile_bool(src)
        .map_err(|error| ScenarioError::Expr {
            site,
            src: src.to_string(),
            error,
        })
}

fn compile_queries(compiler: &mut Compiler, configs: &[QueryConfig]) -> Result<Vec<Query>> {
    if configs.is_empty() {
        return Err(ScenarioError::NoQueries);
    }
    configs
        .iter()
        .map(|q| {
            Ok(Query {
                name: q.name.clone(),
                expr: compile_expr(
                    compiler,
                    &q.expr,
                    ExprSite::Query {
                        name: q.name.clone(),
                    },
                )?,
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
    fn query_errors_carry_site_and_span() {
        let toml = BASE.replace("me.has(EO)", "me.count(trumf) > 1");
        match Scenario::from_toml(&toml) {
            Err(ScenarioError::Expr { site, src, error }) => {
                assert_eq!(site, ExprSite::Query { name: "x".into() });
                assert_eq!(src.get(error.span), Some("trumf"));
            }
            Err(other) => panic!("falscher Fehler: {other}"),
            Ok(_) => panic!("Fehler erwartet"),
        }
    }

    #[test]
    fn empty_me_is_random_hand() {
        let s = with_me("").unwrap_or_else(|e| panic!("{e:#}"));
        assert_eq!(s.possible_hands(), binomial(32, 8));
    }
}
