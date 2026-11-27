//! Cards of the long Schafkopf deck (32 cards) and card sets as bitmasks.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{BitAnd, BitOr, BitOrAssign, Sub};

pub const DECK_SIZE: usize = 32;
pub const CARDS_PER_SUIT: u8 = 8;
const SUIT_MASK: u32 = 0xFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Suit {
    Eichel,
    Gras,
    Herz,
    Schellen,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Eichel, Suit::Gras, Suit::Herz, Suit::Schellen];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn letter(self) -> char {
        match self {
            Suit::Eichel => 'E',
            Suit::Gras => 'G',
            Suit::Herz => 'H',
            Suit::Schellen => 'S',
        }
    }

    pub fn from_letter(c: char) -> Option<Suit> {
        Suit::ALL.into_iter().find(|s| s.letter() == c)
    }

    /// Lowercase name as used in configs and queries.
    pub fn name(self) -> &'static str {
        match self {
            Suit::Eichel => "eichel",
            Suit::Gras => "gras",
            Suit::Herz => "herz",
            Suit::Schellen => "schellen",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Suit::Eichel => "Eichel",
            Suit::Gras => "Gras",
            Suit::Herz => "Herz",
            Suit::Schellen => "Schellen",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rank {
    Ober,
    Unter,
    Sau,
    Zehn,
    Koenig,
    Neun,
    Acht,
    Sieben,
}

impl Rank {
    /// Order of the bit layout inside a suit.
    pub const ALL: [Rank; 8] = [
        Rank::Ober,
        Rank::Unter,
        Rank::Sau,
        Rank::Zehn,
        Rank::Koenig,
        Rank::Neun,
        Rank::Acht,
        Rank::Sieben,
    ];

    /// Strength order inside a suit when Ober/Unter are not trump.
    pub const NATURAL: [Rank; 8] = [
        Rank::Sau,
        Rank::Zehn,
        Rank::Koenig,
        Rank::Ober,
        Rank::Unter,
        Rank::Neun,
        Rank::Acht,
        Rank::Sieben,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            Rank::Ober => "O",
            Rank::Unter => "U",
            Rank::Sau => "A",
            Rank::Zehn => "10",
            Rank::Koenig => "K",
            Rank::Neun => "9",
            Rank::Acht => "8",
            Rank::Sieben => "7",
        }
    }

    pub fn from_symbol(s: &str) -> Option<Rank> {
        match s {
            "Z" => Some(Rank::Zehn),
            _ => Rank::ALL.into_iter().find(|r| r.symbol() == s),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Card(u8);

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Card {
        Card(suit as u8 * CARDS_PER_SUIT + rank as u8)
    }

    /// Indices wrap around the deck, so every `u8` maps to a valid card.
    pub fn from_index(i: u8) -> Card {
        Card(i % DECK_SIZE as u8)
    }

    pub fn index(self) -> u8 {
        self.0
    }

    pub fn suit(self) -> Suit {
        Suit::ALL[(self.0 / CARDS_PER_SUIT) as usize]
    }

    pub fn rank(self) -> Rank {
        Rank::ALL[(self.0 % CARDS_PER_SUIT) as usize]
    }

    pub fn mask(self) -> Cards {
        Cards(1 << self.0)
    }

    /// Parses `EO`, `GU`, `HA`, `S10` (or `SZ`), `EK`, `G9`, ...
    pub fn parse(s: &str) -> Option<Card> {
        let mut chars = s.chars();
        let suit = Suit::from_letter(chars.next()?)?;
        let rank = Rank::from_symbol(chars.as_str())?;
        Some(Card::new(suit, rank))
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.suit().letter(), self.rank().symbol())
    }
}

/// A set of cards, one bit per card; `u32` covers exactly the 32-card deck.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cards(pub u32);

impl Cards {
    pub const EMPTY: Cards = Cards(0);
    pub const ALL: Cards = Cards(u32::MAX);

    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, card: Card) -> bool {
        self.0 & (1 << card.0) != 0
    }

    pub fn is_subset_of(self, other: Cards) -> bool {
        self.0 & !other.0 == 0
    }

    pub fn intersects(self, other: Cards) -> bool {
        self.0 & other.0 != 0
    }

    /// All eight physical cards of a suit, including Ober and Unter.
    pub fn of_suit(suit: Suit) -> Cards {
        Cards(SUIT_MASK << (suit as u32 * CARDS_PER_SUIT as u32))
    }

    pub fn of_rank(rank: Rank) -> Cards {
        Suit::ALL
            .into_iter()
            .map(|s| Card::new(s, rank).mask())
            .fold(Cards::EMPTY, |acc, c| acc | c)
    }

    pub fn iter(self) -> impl Iterator<Item = Card> {
        let mut bits = self.0;
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let i = bits.trailing_zeros() as u8;
            bits &= bits - 1;
            Some(Card::from_index(i))
        })
    }
}

/// Space-separated in deck order, e.g. `EO GO HU`.
impl fmt::Display for Cards {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, card) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(" ")?;
            }
            write!(f, "{card}")?;
        }
        Ok(())
    }
}

impl FromIterator<Card> for Cards {
    fn from_iter<I: IntoIterator<Item = Card>>(iter: I) -> Self {
        iter.into_iter().fold(Cards::EMPTY, |acc, c| acc | c.mask())
    }
}

impl BitOr for Cards {
    type Output = Cards;
    fn bitor(self, rhs: Cards) -> Cards {
        Cards(self.0 | rhs.0)
    }
}

impl BitOrAssign for Cards {
    fn bitor_assign(&mut self, rhs: Cards) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Cards {
    type Output = Cards;
    fn bitand(self, rhs: Cards) -> Cards {
        Cards(self.0 & rhs.0)
    }
}

impl Sub for Cards {
    type Output = Cards;
    fn sub(self, rhs: Cards) -> Cards {
        Cards(self.0 & !rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        for i in 0..32 {
            let card = Card::from_index(i);
            assert_eq!(Card::parse(&card.to_string()), Some(card));
        }
        assert_eq!(Card::parse("SZ"), Card::parse("S10"));
        assert_eq!(Card::parse("XO"), None);
        assert_eq!(Card::parse("E1"), None);
        assert_eq!(Card::parse("eo"), None);
    }

    #[test]
    fn masks() {
        assert_eq!(Cards::of_rank(Rank::Ober).len(), 4);
        assert_eq!(Cards::of_suit(Suit::Herz).len(), 8);
        let all: Cards = (0..32).map(Card::from_index).collect();
        assert_eq!(all, Cards::ALL);
    }
}
