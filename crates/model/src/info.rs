//! What the UI needs to know about a game to draw it: card order, the picker's groups and
//! the cards behind each named set (for rule caps). The rules themselves stay in core.

use crate::model::CardSet;
use serde::Serialize;
use zefix_core::cards::{Card, Cards, Rank};
use zefix_core::game::Game;

#[derive(Serialize)]
pub struct GameInfo {
    /// All cards, highest trump first, then each plain suit; tiles and hands sort by it.
    pub order: Vec<String>,
    /// Trumps first, then the plain suits, as the card picker shows them.
    pub groups: Vec<Group>,
    /// Non-trump suits, for the "Farbe" rule face.
    pub plain_suits: Vec<String>,
    /// Cards behind each `CardSet`, keyed by its query-language name.
    pub sets: Vec<NamedSet>,
    /// Only a Sauspiel has a partner and two opponents.
    pub called_sau: Option<String>,
    /// E.g. "Ober, Unter, Herz" in a Sauspiel or "Unter" in a Wenz.
    pub trump_description: String,
}

#[derive(Serialize)]
pub struct Group {
    pub label: String,
    pub cards: Vec<String>,
}

#[derive(Serialize)]
pub struct NamedSet {
    pub ident: &'static str,
    pub cards: Vec<String>,
}

pub fn game_info(game: &Game) -> GameInfo {
    let names =
        |cards: Cards| -> Vec<String> { game.sorted(cards).iter().map(Card::to_string).collect() };
    let groups = std::iter::once(Group {
        label: "Trumpf".into(),
        cards: names(game.trumps),
    })
    .chain(game.plain_suits.iter().map(|&s| Group {
        label: s.display_name().into(),
        cards: names(game.suit_set(s)),
    }))
    .collect();
    GameInfo {
        order: names(Cards::ALL),
        groups,
        plain_suits: game.plain_suits.iter().map(|s| s.name().into()).collect(),
        sets: CardSet::ALL
            .iter()
            .map(|&set| NamedSet {
                ident: set.ident(),
                cards: names(crate::model::set_cards(set, game)),
            })
            .collect(),
        called_sau: game.called_sau().map(|c| c.to_string()),
        trump_description: trump_description(game),
    }
}

fn trump_description(game: &Game) -> String {
    let ranks = [(Rank::Ober, "Ober"), (Rank::Unter, "Unter")]
        .into_iter()
        .filter(|&(rank, _)| (Cards::of_rank(rank) - game.trumps).is_empty())
        .map(|(_, name)| name);
    let suit = game.trump_suit().map(|s| s.display_name());
    ranks.chain(suit).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use zefix_core::cards::Suit;
    use zefix_core::game::GameKind;

    #[test]
    fn sauspiel_groups_and_sets() {
        let game =
            Game::new(GameKind::Sauspiel { called: Suit::Gras }).unwrap_or_else(|e| panic!("{e}"));
        let info = game_info(&game);
        assert_eq!(info.order.len(), 32);
        assert_eq!(info.order.first().map(String::as_str), Some("EO"));
        assert_eq!(info.groups.len(), 4);
        assert_eq!(info.groups[0].cards.len(), 14);
        assert_eq!(info.called_sau.as_deref(), Some("GA"));
        assert_eq!(info.trump_description, "Ober, Unter, Herz");
        let ober = info.sets.iter().find(|s| s.ident == "ober");
        assert_eq!(ober.map(|s| s.cards.len()), Some(4));
    }
}
