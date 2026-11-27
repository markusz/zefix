//! Game types and the card sets that depend on them (trump, suits, ...).

use crate::cards::{Card, Cards, DECK_SIZE, Rank, Suit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum GameKind {
    Sauspiel { called: Suit },
    Solo { trump: Suit },
    Wenz,
    Farbwenz { trump: Suit },
    Geier,
    Farbgeier { trump: Suit },
}

pub const PLAYER_COUNT: usize = 4;

/// Seats relative to me.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Seat {
    Me,
    Left,
    Across,
    Right,
}

impl Seat {
    pub const ALL: [Seat; PLAYER_COUNT] = [Seat::Me, Seat::Left, Seat::Across, Seat::Right];
    pub const OTHERS: [Seat; PLAYER_COUNT - 1] = [Seat::Left, Seat::Across, Seat::Right];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Seat::Me => "me",
            Seat::Left => "left",
            Seat::Across => "across",
            Seat::Right => "right",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("die Herz-Sau kann nicht gerufen werden")]
    HerzSauCalled,
}

/// Which Ober/Unter ranks and which suit are trump.
struct TrumpRule {
    ranks: &'static [Rank],
    suit: Option<Suit>,
}

impl TrumpRule {
    fn for_kind(kind: GameKind) -> Result<TrumpRule, GameError> {
        const OBER_UNTER: &[Rank] = &[Rank::Ober, Rank::Unter];
        const UNTER: &[Rank] = &[Rank::Unter];
        const OBER: &[Rank] = &[Rank::Ober];
        let rule = |ranks, suit| Ok(TrumpRule { ranks, suit });
        match kind {
            GameKind::Sauspiel { called: Suit::Herz } => Err(GameError::HerzSauCalled),
            GameKind::Sauspiel { .. } => rule(OBER_UNTER, Some(Suit::Herz)),
            GameKind::Solo { trump } => rule(OBER_UNTER, Some(trump)),
            GameKind::Wenz => rule(UNTER, None),
            GameKind::Farbwenz { trump } => rule(UNTER, Some(trump)),
            GameKind::Geier => rule(OBER, None),
            GameKind::Farbgeier { trump } => rule(OBER, Some(trump)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub kind: GameKind,
    pub trumps: Cards,
    /// Per suit: the suit's cards minus Ober/Unter that are trump.
    suit_sets: [Cards; 4],
    /// Suits that are not trump.
    pub plain_suits: Vec<Suit>,
    /// Display order: strongest trump first, then suits.
    sort_rank: [u8; DECK_SIZE],
}

impl Game {
    pub fn new(kind: GameKind) -> Result<Game, GameError> {
        let rule = TrumpRule::for_kind(kind)?;
        let rank_trumps = rule
            .ranks
            .iter()
            .fold(Cards::EMPTY, |acc, &r| acc | Cards::of_rank(r));
        let suit_sets = Suit::ALL.map(|s| Cards::of_suit(s) - rank_trumps);
        let trumps = rank_trumps | rule.suit.map_or(Cards::EMPTY, |s| suit_sets[s.index()]);
        let plain_suits: Vec<Suit> = Suit::ALL
            .into_iter()
            .filter(|&s| Some(s) != rule.suit)
            .collect();
        let sort_rank = display_order(&rule, &suit_sets, &plain_suits);
        Ok(Game {
            kind,
            trumps,
            suit_sets,
            plain_suits,
            sort_rank,
        })
    }

    pub fn suit_set(&self, suit: Suit) -> Cards {
        self.suit_sets[suit.index()]
    }

    /// Suit whose plain cards are trump; `None` in Wenz and Geier.
    pub fn trump_suit(&self) -> Option<Suit> {
        match self.kind {
            GameKind::Sauspiel { .. } => Some(Suit::Herz),
            GameKind::Solo { trump }
            | GameKind::Farbwenz { trump }
            | GameKind::Farbgeier { trump } => Some(trump),
            GameKind::Wenz | GameKind::Geier => None,
        }
    }

    pub fn called_sau(&self) -> Option<Card> {
        match self.kind {
            GameKind::Sauspiel { called } => Some(Card::new(called, Rank::Sau)),
            _ => None,
        }
    }

    /// Card sets available by name in queries.
    pub fn named_set(&self, name: &str) -> Option<Cards> {
        let rank = |r| Some(Cards::of_rank(r));
        match name {
            "trumpf" => Some(self.trumps),
            "farbe" => Some(Cards::ALL - self.trumps),
            "alle" => Some(Cards::ALL),
            "ober" => rank(Rank::Ober),
            "unter" => rank(Rank::Unter),
            "sau" => rank(Rank::Sau),
            "zehn" => rank(Rank::Zehn),
            "koenig" => rank(Rank::Koenig),
            "neun" => rank(Rank::Neun),
            "acht" => rank(Rank::Acht),
            "sieben" => rank(Rank::Sieben),
            _ => Suit::ALL
                .into_iter()
                .find(|s| s.name() == name)
                .map(|s| self.suit_set(s)),
        }
    }

    pub fn sorted(&self, cards: Cards) -> Vec<Card> {
        let mut v: Vec<Card> = cards.iter().collect();
        v.sort_by_key(|c| self.sort_rank[c.index() as usize]);
        v
    }

    pub fn describe(&self) -> String {
        match self.kind {
            GameKind::Sauspiel { called } => {
                format!("Sauspiel auf die {}-Sau", called.display_name())
            }
            GameKind::Solo { trump } => format!("{}-Solo", trump.display_name()),
            GameKind::Wenz => "Wenz".into(),
            GameKind::Farbwenz { trump } => format!("{}-Wenz", trump.display_name()),
            GameKind::Geier => "Geier".into(),
            GameKind::Farbgeier { trump } => format!("{}-Geier", trump.display_name()),
        }
    }
}

fn display_order(
    rule: &TrumpRule,
    suit_sets: &[Cards; 4],
    plain_suits: &[Suit],
) -> [u8; DECK_SIZE] {
    let mut order: Vec<Card> = Vec::with_capacity(DECK_SIZE);
    for &rank in rule.ranks {
        order.extend(Suit::ALL.map(|s| Card::new(s, rank)));
    }
    for suit in rule.suit.into_iter().chain(plain_suits.iter().copied()) {
        let in_suit = suit_sets[suit.index()];
        order.extend(
            Rank::NATURAL
                .into_iter()
                .map(|r| Card::new(suit, r))
                .filter(|c| in_suit.contains(*c)),
        );
    }
    let mut sort_rank = [0u8; DECK_SIZE];
    for (pos, card) in order.iter().enumerate() {
        sort_rank[card.index() as usize] = pos as u8;
    }
    sort_rank
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards(s: &str) -> Cards {
        s.split_whitespace()
            .map(|c| Card::parse(c).unwrap())
            .collect()
    }

    #[test]
    fn sauspiel_sets() {
        let g = Game::new(GameKind::Sauspiel {
            called: Suit::Schellen,
        })
        .unwrap();
        assert_eq!(g.trumps.len(), 14);
        assert_eq!(g.named_set("herz"), Some(cards("HA H10 HK H9 H8 H7")));
        assert_eq!(g.named_set("eichel"), Some(cards("EA E10 EK E9 E8 E7")));
        assert_eq!(g.named_set("farbe").unwrap().len(), 18);
        assert_eq!(
            g.plain_suits,
            vec![Suit::Eichel, Suit::Gras, Suit::Schellen]
        );
        let order: Vec<String> = g
            .sorted(cards("H7 EO SU HA E10 EA"))
            .iter()
            .map(|c| c.to_string())
            .collect();
        assert_eq!(order, ["EO", "SU", "HA", "H7", "EA", "E10"]);
    }

    #[test]
    fn wenz_sets() {
        let g = Game::new(GameKind::Wenz).unwrap();
        assert_eq!(g.trumps, Cards::of_rank(Rank::Unter));
        assert_eq!(g.named_set("eichel").unwrap().len(), 7);
        assert!(
            g.named_set("eichel")
                .unwrap()
                .contains(Card::parse("EO").unwrap())
        );
        assert_eq!(g.plain_suits.len(), 4);
    }

    #[test]
    fn herz_sau_cannot_be_called() {
        assert!(Game::new(GameKind::Sauspiel { called: Suit::Herz }).is_err());
    }
}
