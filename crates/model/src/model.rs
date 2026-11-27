//! The scenario as the UI edits it: tiles per seat and per query. Compiles to a core
//! `Config` via query-language text, so the UI and the TOML files share one semantics.

use serde::{Deserialize, Serialize};
use zefix_core::cards::{Card, Cards, Suit};
use zefix_core::game::{Game, GameKind, PLAYER_COUNT, Seat};
use zefix_core::scenario::{Config, MeConfig, QueryConfig, TableConfig};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameChoice {
    Sauspiel,
    Solo,
    Wenz,
    Farbwenz,
    Geier,
    Farbgeier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSetup {
    pub choice: GameChoice,
    pub suit: Suit,
}

impl GameSetup {
    pub fn kind(self) -> GameKind {
        let suit = self.suit;
        match self.choice {
            GameChoice::Sauspiel => GameKind::Sauspiel { called: suit },
            GameChoice::Solo => GameKind::Solo { trump: suit },
            GameChoice::Wenz => GameKind::Wenz,
            GameChoice::Farbwenz => GameKind::Farbwenz { trump: suit },
            GameChoice::Geier => GameKind::Geier,
            GameChoice::Farbgeier => GameKind::Farbgeier { trump: suit },
        }
    }

    /// Keeps the suit valid after the game type changed (e.g. no Herz-Sau to call).
    pub fn normalize(&mut self) {
        const CALLABLE: [Suit; 3] = [Suit::Eichel, Suit::Gras, Suit::Schellen];
        if self.choice == GameChoice::Sauspiel && !CALLABLE.contains(&self.suit) {
            self.suit = Suit::Eichel;
        }
    }

    pub fn game(self) -> Option<Game> {
        Game::new(self.kind()).ok()
    }
}

/// Position in play order; the first position is Vorhand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    First,
    Second,
    Third,
    Fourth,
}

impl Position {
    pub const ALL: [Position; PLAYER_COUNT] = [
        Position::First,
        Position::Second,
        Position::Third,
        Position::Fourth,
    ];

    /// Play runs clockwise, so the player after me sits on my left.
    pub fn seat(self, me: Position) -> Seat {
        Seat::ALL[(self as usize + PLAYER_COUNT - me as usize) % PLAYER_COUNT]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardSet {
    Trumpf,
    Farbe,
    Ober,
    Unter,
    Sau,
    Zehn,
    Koenig,
    Suit(Suit),
}

impl CardSet {
    pub const ALL: [CardSet; 11] = [
        CardSet::Trumpf,
        CardSet::Farbe,
        CardSet::Ober,
        CardSet::Unter,
        CardSet::Sau,
        CardSet::Zehn,
        CardSet::Koenig,
        CardSet::Suit(Suit::Eichel),
        CardSet::Suit(Suit::Gras),
        CardSet::Suit(Suit::Herz),
        CardSet::Suit(Suit::Schellen),
    ];

    /// Name in the query language.
    pub fn ident(self) -> &'static str {
        match self {
            CardSet::Trumpf => "trumpf",
            CardSet::Farbe => "farbe",
            CardSet::Ober => "ober",
            CardSet::Unter => "unter",
            CardSet::Sau => "sau",
            CardSet::Zehn => "zehn",
            CardSet::Koenig => "koenig",
            CardSet::Suit(s) => s.name(),
        }
    }
}

pub fn set_cards(set: CardSet, game: &Game) -> Cards {
    game.named_set(set.ident()).unwrap_or(Cards::ALL)
}

/// Fixed cards of the same hand that belong to `set`. Rules count on top of these, so
/// "EO, GO, 2–5 Trumpf" means two to five trumps besides the two Ober.
pub fn fixed_in(set: CardSet, fixed: Cards, game: &Game) -> u8 {
    (set_cards(set, game) & fixed).len() as u8
}

/// The comparison a count tile makes; the editor's dropdown picks the kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountOp {
    AtLeast(u8),
    AtMost(u8),
    Exactly(u8),
    Between { min: u8, max: u8 },
    Zero,
}

impl CountOp {
    /// The same comparison on the total count, given `fixed` cards of the set already held.
    fn on_top_of(self, fixed: u8) -> CountOp {
        match self {
            CountOp::AtLeast(0) => CountOp::AtLeast(0),
            CountOp::AtLeast(n) => CountOp::AtLeast(n + fixed),
            CountOp::AtMost(n) => CountOp::AtMost(n + fixed),
            CountOp::Exactly(n) => CountOp::Exactly(n + fixed),
            CountOp::Between { min, max } => CountOp::Between {
                min: min + fixed,
                max: max + fixed,
            },
            CountOp::Zero => CountOp::Exactly(fixed),
        }
    }

    /// `None` if the comparison does not restrict anything.
    fn condition(self, subject: &str, set: &str) -> Option<String> {
        let count = format!("{subject}.count({set})");
        match self {
            CountOp::AtLeast(0) => None,
            CountOp::AtLeast(n) => Some(format!("{count} >= {n}")),
            CountOp::AtMost(n) => Some(format!("{count} <= {n}")),
            CountOp::Exactly(0) | CountOp::Zero => Some(format!("{subject}.frei({set})")),
            CountOp::Exactly(n) => Some(format!("{count} == {n}")),
            CountOp::Between { min, max } if min == max => {
                CountOp::Exactly(min).condition(subject, set)
            }
            CountOp::Between { min: 0, max } => CountOp::AtMost(max).condition(subject, set),
            CountOp::Between { min, max } => Some(format!("{count} >= {min} and {count} <= {max}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tile {
    Card(#[serde(with = "card_text")] Card),
    Count { set: CardSet, op: CountOp },
}

pub fn cards_of(tiles: &[Tile]) -> Cards {
    tiles
        .iter()
        .filter_map(|t| match t {
            Tile::Card(c) => Some(*c),
            Tile::Count { .. } => None,
        })
        .collect()
}

/// How a hand's fixed cards enter its condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedCards {
    /// Checked with `has(..)`.
    Required,
    /// My cards are dealt directly, so only the rules need checking.
    Dealt,
}

/// All tiles of `subject` joined with `and`; `None` if nothing is constrained.
pub fn condition(subject: &str, tiles: &[Tile], game: &Game, mode: FixedCards) -> Option<String> {
    let mut parts = Vec::new();
    let cards = cards_of(tiles);
    if !cards.is_empty() && mode == FixedCards::Required {
        let names: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
        parts.push(format!("{subject}.has({})", names.join(", ")));
    }
    parts.extend(tiles.iter().filter_map(|t| {
        match t {
            Tile::Count { set, op } => op
                .on_top_of(fixed_in(*set, cards, game))
                .condition(subject, set.ident()),
            Tile::Card(_) => None,
        }
    }));
    (!parts.is_empty()).then(|| parts.join(" and "))
}

/// Who a visual query talks about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Player(Seat),
    /// Sauspiel only: whoever holds the called Sau.
    Partner,
    AnyOther,
    AllOthers,
    AtLeast(u8),
    /// Sauspiel only: the two players who are not my partner.
    AnyOpponent,
    AllOpponents,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryMode {
    Visual,
    Text,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueryModel {
    pub name: String,
    pub mode: QueryMode,
    pub scope: Scope,
    pub tiles: Vec<Tile>,
    pub text: String,
}

/// Bound variable in generated quantifiers.
const VAR: &str = "p";
const PARTNER: &str = "partner";

impl QueryModel {
    pub fn visual(name: &str, scope: Scope, tiles: Vec<Tile>) -> QueryModel {
        QueryModel {
            name: name.into(),
            mode: QueryMode::Visual,
            scope,
            tiles,
            text: String::new(),
        }
    }

    /// Query-language source; `None` while the query is empty.
    pub fn expr(&self, game: &Game) -> Option<String> {
        match self.mode {
            QueryMode::Text => {
                let text = self.text.trim();
                (!text.is_empty()).then(|| text.to_string())
            }
            QueryMode::Visual => self.visual_expr(game),
        }
    }

    pub fn visual_expr(&self, game: &Game) -> Option<String> {
        let subject = match self.scope {
            Scope::Player(seat) => seat.name(),
            Scope::Partner => PARTNER,
            Scope::AnyOther
            | Scope::AllOthers
            | Scope::AtLeast(_)
            | Scope::AnyOpponent
            | Scope::AllOpponents => VAR,
        };
        let body = condition(subject, &self.tiles, game, FixedCards::Required)?;
        Some(match self.scope {
            Scope::Player(_) | Scope::Partner => body,
            Scope::AnyOther => format!("any({VAR} in others, {body})"),
            Scope::AllOthers => format!("all({VAR} in others, {body})"),
            Scope::AnyOpponent => format!("any({VAR} in opponents, {body})"),
            Scope::AllOpponents => format!("all({VAR} in opponents, {body})"),
            Scope::AtLeast(k) => format!("num({VAR} in others, {body}) >= {k}"),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SeatModel {
    pub tiles: Vec<Tile>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub game: GameSetup,
    pub my_position: Position,
    /// Indexed by `Seat::index`, so my hand stays mine when I change position.
    pub seats: [SeatModel; PLAYER_COUNT],
    pub queries: Vec<QueryModel>,
    /// Further condition on the deal in query-language text, e.g. from an imported TOML.
    #[serde(default)]
    pub filter_text: String,
}

impl Default for Model {
    fn default() -> Model {
        Model {
            game: GameSetup {
                choice: GameChoice::Sauspiel,
                suit: Suit::Gras,
            },
            my_position: Position::First,
            seats: Default::default(),
            queries: vec![QueryModel::visual(
                "Ein Gegner hat ≥ 4 Trumpf",
                Scope::AnyOpponent,
                vec![Tile::Count {
                    set: CardSet::Trumpf,
                    op: CountOp::AtLeast(4),
                }],
            )],
            filter_text: String::new(),
        }
    }
}

/// Everything a simulation run depends on; a change means the results are stale.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub game: GameKind,
    pub vorhand: Seat,
    pub my_cards: Cards,
    pub filter: Option<String>,
    pub queries: Vec<QuerySpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuerySpec {
    pub name: String,
    pub expr: String,
}

impl Model {
    pub fn seat(&self, seat: Seat) -> &SeatModel {
        &self.seats[seat.index()]
    }

    pub fn seat_mut(&mut self, seat: Seat) -> &mut SeatModel {
        &mut self.seats[seat.index()]
    }

    pub fn spec(&self, game: &Game) -> Spec {
        Spec {
            game: self.game.kind(),
            vorhand: Position::First.seat(self.my_position),
            my_cards: cards_of(&self.seat(Seat::Me).tiles),
            filter: self.filter(game),
            queries: self
                .queries
                .iter()
                .filter_map(|q| {
                    Some(QuerySpec {
                        name: q.name.clone(),
                        expr: q.expr(game)?,
                    })
                })
                .collect(),
        }
    }

    /// Constraints on all seats except my fixed cards, which are dealt directly, plus the
    /// text condition.
    pub fn filter(&self, game: &Game) -> Option<String> {
        let mut parts: Vec<String> = Seat::ALL
            .iter()
            .filter_map(|&seat| {
                let mode = match seat {
                    Seat::Me => FixedCards::Dealt,
                    _ => FixedCards::Required,
                };
                condition(seat.name(), &self.seat(seat).tiles, game, mode)
            })
            .collect();
        let text = self.filter_text.trim();
        if !text.is_empty() {
            // Seat conditions are plain conjunctions; only free text can hold an `or`.
            parts.push(if parts.is_empty() {
                text.to_string()
            } else {
                format!("({text})")
            });
        }
        (!parts.is_empty()).then(|| parts.join(" and "))
    }
}

impl Spec {
    pub fn to_config(&self) -> Config {
        Config {
            iterations: None,
            seed: None,
            game: self.game,
            table: Some(TableConfig {
                vorhand: self.vorhand,
            }),
            me: MeConfig {
                cards: self.my_cards.iter().map(|c| c.to_string()).collect(),
                slots: Vec::new(),
                filter: self.filter.clone(),
            },
            queries: self
                .queries
                .iter()
                .map(|q| QueryConfig {
                    name: q.name.clone(),
                    expr: q.expr.clone(),
                })
                .collect(),
        }
    }
}

/// Cards as their short names (`EO`, `S10`) in serialized models.
mod card_text {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};
    use zefix_core::cards::Card;

    pub fn serialize<S: Serializer>(card: &Card, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(card)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Card, D::Error> {
        let text = String::deserialize(d)?;
        Card::parse(&text).ok_or_else(|| D::Error::custom(format!("unbekannte Karte {text}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zefix_core::scenario::Scenario;

    fn card(s: &str) -> Tile {
        Tile::Card(Card::parse(s).unwrap_or_else(|| panic!("{s}")))
    }

    fn count(set: CardSet, op: CountOp) -> Tile {
        Tile::Count { set, op }
    }

    fn sauspiel() -> Game {
        let setup = GameSetup {
            choice: GameChoice::Sauspiel,
            suit: Suit::Gras,
        };
        setup.game().unwrap_or_else(|| panic!("Sauspiel"))
    }

    fn cond(subject: &str, tiles: &[Tile]) -> Option<String> {
        condition(subject, tiles, &sauspiel(), FixedCards::Required)
    }

    #[test]
    fn positions_map_to_seats_clockwise() {
        let me = Position::Second;
        assert_eq!(Position::First.seat(me), Seat::Right);
        assert_eq!(Position::Third.seat(me), Seat::Left);
        assert_eq!(Position::Fourth.seat(me), Seat::Across);
    }

    #[test]
    fn tiles_become_conditions() {
        let tiles = [
            card("GA"),
            count(CardSet::Suit(Suit::Eichel), CountOp::AtMost(4)),
            count(CardSet::Suit(Suit::Herz), CountOp::Zero),
            count(CardSet::Ober, CountOp::AtLeast(0)),
        ];
        assert_eq!(
            cond("left", &tiles).as_deref(),
            Some("left.has(GA) and left.count(eichel) <= 4 and left.frei(herz)")
        );
        assert_eq!(
            cond("left", &[count(CardSet::Ober, CountOp::AtLeast(0))]),
            None
        );
    }

    #[test]
    fn rules_count_on_top_of_fixed_cards() {
        // Sauspiel: GO and SU are trumps, GA is not.
        let tiles = [
            card("GO"),
            card("SU"),
            card("GA"),
            count(CardSet::Trumpf, CountOp::Between { min: 2, max: 4 }),
            count(CardSet::Ober, CountOp::Zero),
            count(CardSet::Suit(Suit::Gras), CountOp::AtLeast(1)),
        ];
        assert_eq!(
            cond("left", &tiles).as_deref(),
            Some(
                "left.has(GO, GA, SU) and left.count(trumpf) >= 4 and left.count(trumpf) <= 6 \
                 and left.count(ober) == 1 and left.count(gras) >= 2"
            )
        );
        assert_eq!(fixed_in(CardSet::Trumpf, cards_of(&tiles), &sauspiel()), 2);
    }

    #[test]
    fn scopes_wrap_the_condition() {
        let tiles = vec![count(CardSet::Trumpf, CountOp::AtLeast(4))];
        let game = sauspiel();
        let expr = |scope| QueryModel::visual("x", scope, tiles.clone()).expr(&game);
        assert_eq!(
            expr(Scope::AnyOther).as_deref(),
            Some("any(p in others, p.count(trumpf) >= 4)")
        );
        assert_eq!(
            expr(Scope::AtLeast(2)).as_deref(),
            Some("num(p in others, p.count(trumpf) >= 4) >= 2")
        );
        assert_eq!(
            expr(Scope::Player(Seat::Across)).as_deref(),
            Some("across.count(trumpf) >= 4")
        );
        assert_eq!(
            expr(Scope::Partner).as_deref(),
            Some("partner.count(trumpf) >= 4")
        );
        assert_eq!(
            expr(Scope::AllOpponents).as_deref(),
            Some("all(p in opponents, p.count(trumpf) >= 4)")
        );
        assert_eq!(
            QueryModel::visual("x", Scope::AnyOther, vec![]).expr(&game),
            None
        );
    }

    #[test]
    fn spec_compiles_and_deals_my_cards_directly() {
        let mut model = Model {
            my_position: Position::Second,
            ..Model::default()
        };
        model.seat_mut(Seat::Me).tiles =
            vec![card("EO"), count(CardSet::Trumpf, CountOp::AtLeast(3))];
        model.seat_mut(Seat::Left).tiles = vec![count(CardSet::Suit(Suit::Herz), CountOp::Zero)];
        let trumps = vec![count(CardSet::Trumpf, CountOp::AtLeast(3))];
        for scope in [Scope::Partner, Scope::AllOpponents] {
            model
                .queries
                .push(QueryModel::visual("x", scope, trumps.clone()));
        }
        let spec = model.spec(&sauspiel());
        assert_eq!(spec.vorhand, Seat::Right);
        // EO is dealt directly but still counts towards the trump rule.
        assert_eq!(
            spec.filter.as_deref(),
            Some("me.count(trumpf) >= 4 and left.frei(herz)")
        );
        let scn = Scenario::from_config(spec.to_config()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(scn.fixed.len(), 1);
    }
}
