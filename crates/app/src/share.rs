//! Moving scenarios in and out of the app: share links that restore the exact UI state,
//! TOML for the CLI and for AI assistants, and importing either.

use crate::model::{GameChoice, GameSetup, Model, Position, QueryMode, QueryModel, Scope};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use zefix_core::cards::Card;
use zefix_core::game::{Game, GameKind, Seat};
use zefix_core::scenario::{Config, Scenario};

/// Where the web build is published; share links from the desktop point there too.
pub const APP_URL: &str = "https://zefix.markusziller.de/";
/// Marks the model in a link's fragment; the digit versions the encoding.
const LINK_KEY: &str = "s1=";

/// Fragment (without `#`) that restores `model`.
pub fn fragment(model: &Model) -> String {
    let json = serde_json::to_vec(model).unwrap_or_default();
    format!("{LINK_KEY}{}", URL_SAFE_NO_PAD.encode(json))
}

pub fn link(model: &Model) -> String {
    format!("{APP_URL}#{}", fragment(model))
}

/// The model in a link, a bare fragment, or anything containing one.
pub fn from_link(text: &str) -> Option<Result<Model, String>> {
    let start = text.find(LINK_KEY)? + LINK_KEY.len();
    let encoded: String = text
        .get(start..)?
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect();
    Some(
        URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| "Der Link ist unvollständig.".to_string())
            .and_then(|json| {
                serde_json::from_slice(&json)
                    .map_err(|_| "Der Link passt nicht zu dieser Version von zefix!.".to_string())
            }),
    )
}

pub fn to_toml(model: &Model, game: &Game) -> String {
    toml::to_string(&model.spec(game).to_config()).unwrap_or_default()
}

/// A model built from outside input, with what it had to leave out.
pub struct Imported {
    pub model: Model,
    pub dropped: Vec<String>,
}

/// Accepts a share link or a TOML scenario.
pub fn import(text: &str) -> Result<Imported, String> {
    if let Some(model) = from_link(text) {
        return model.map(|model| Imported {
            model,
            dropped: Vec::new(),
        });
    }
    // Compiling first reports every error with its position, including query-language ones.
    Scenario::from_toml(text).map_err(|e| render_error(&e))?;
    let cfg: Config = toml::from_str(text).map_err(|e| e.to_string())?;
    from_config(cfg)
}

fn render_error(e: &zefix_core::scenario::ScenarioError) -> String {
    match std::error::Error::source(e) {
        Some(source) if matches!(e, zefix_core::scenario::ScenarioError::Toml(_)) => {
            format!("{e}: {source}")
        }
        _ => e.to_string(),
    }
}

fn from_config(cfg: Config) -> Result<Imported, String> {
    let mut dropped = Vec::new();
    let mut model = Model {
        game: game_setup(cfg.game),
        queries: Vec::new(),
        ..Model::default()
    };
    if let Some(table) = cfg.table {
        model.my_position = my_position(table.vorhand);
    }
    for name in &cfg.me.cards {
        let card = Card::parse(name).ok_or_else(|| format!("unbekannte Karte {name}"))?;
        model
            .seat_mut(Seat::Me)
            .tiles
            .push(crate::model::Tile::Card(card));
    }
    if !cfg.me.slots.is_empty() {
        dropped.push("Slots (die Oberfläche kennt nur feste Karten und Regeln)".to_string());
    }
    if cfg.iterations.is_some() || cfg.seed.is_some() {
        dropped.push("iterations und seed (die Genauigkeit wählst du oben)".to_string());
    }
    model.filter_text = cfg.me.filter.unwrap_or_default();
    model.queries = cfg
        .queries
        .into_iter()
        .map(|q| QueryModel {
            mode: QueryMode::Text,
            text: q.expr,
            ..QueryModel::visual(&q.name, Scope::AnyOther, Vec::new())
        })
        .collect();
    Ok(Imported { model, dropped })
}

fn game_setup(kind: GameKind) -> GameSetup {
    let (choice, suit) = match kind {
        GameKind::Sauspiel { called } => (GameChoice::Sauspiel, Some(called)),
        GameKind::Solo { trump } => (GameChoice::Solo, Some(trump)),
        GameKind::Wenz => (GameChoice::Wenz, None),
        GameKind::Farbwenz { trump } => (GameChoice::Farbwenz, Some(trump)),
        GameKind::Geier => (GameChoice::Geier, None),
        GameKind::Farbgeier { trump } => (GameChoice::Farbgeier, Some(trump)),
    };
    let mut setup = GameSetup {
        choice,
        suit: suit.unwrap_or(Model::default().game.suit),
    };
    setup.normalize();
    setup
}

/// The position whose Vorhand sits at `vorhand`, as seen from me.
fn my_position(vorhand: Seat) -> Position {
    Position::ALL
        .into_iter()
        .find(|&me| Position::First.seat(me) == vorhand)
        .unwrap_or(Position::First)
}

/// The browser's address bar mirrors the model, so reloading keeps it and copying the
/// address shares it.
#[cfg(target_arch = "wasm32")]
pub mod location {
    use super::*;

    pub fn read() -> Option<Result<Model, String>> {
        let hash = web_sys::window()?.location().hash().ok()?;
        from_link(&hash)
    }

    pub fn write(model: &Model) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let url = format!("#{}", fragment(model));
        if let Ok(history) = window.history() {
            // Replaced, not pushed: every edit would otherwise become a Back step.
            let _ = history.replace_state_with_url(
                &eframe::wasm_bindgen::JsValue::NULL,
                "",
                Some(&url),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CardSet, CountOp, Tile};
    use zefix_core::cards::Suit;

    fn sample() -> Model {
        let mut model = Model::default();
        model.game.choice = GameChoice::Solo;
        model.game.suit = Suit::Eichel;
        model.my_position = Position::Third;
        model.seat_mut(Seat::Me).tiles = ["EO", "GO", "S10"]
            .iter()
            .filter_map(|c| Card::parse(c).map(Tile::Card))
            .collect();
        model.seat_mut(Seat::Left).tiles = vec![Tile::Count {
            set: CardSet::Suit(Suit::Gras),
            op: CountOp::Zero,
        }];
        model.filter_text = "across.count(trumpf) >= 2 or right.has(SO)".into();
        model
    }

    fn game(model: &Model) -> Game {
        model.game.game().unwrap_or_else(|| panic!("invalid game"))
    }

    #[test]
    fn links_restore_the_model() {
        let model = sample();
        let restored = from_link(&link(&model));
        assert_eq!(restored, Some(Ok(model)));
    }

    #[test]
    fn broken_links_are_errors() {
        assert!(matches!(from_link("#s1=%%%"), Some(Err(_))));
        assert!(matches!(from_link("#s1=e30"), Some(Err(_))));
        assert_eq!(from_link("https://example.org/"), None);
    }

    #[test]
    fn toml_round_trip_keeps_the_scenario() {
        let model = sample();
        let game = game(&model);
        let text = to_toml(&model, &game);
        let imported = import(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert!(imported.dropped.is_empty(), "{:?}", imported.dropped);
        let back = imported.model;
        assert_eq!(back.game, model.game);
        assert_eq!(back.my_position, model.my_position);
        assert_eq!(back.spec(&game), model.spec(&game));
    }

    #[test]
    fn imports_files_and_names_what_it_drops() {
        let imported =
            import(include_str!("../../../scenarios/ober.toml")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(imported.dropped.len(), 2, "{:?}", imported.dropped);
        assert_eq!(imported.model.queries.len(), 3);
        assert_eq!(imported.model.game.suit, Suit::Schellen);
    }

    #[test]
    fn import_errors_point_at_the_problem() {
        let err = import("[game]\ntype = \"solo\"\ntrump = \"eichel\"\n[[query]]\nname = \"x\"\nwhere = \"me.count(trumf) > 1\"")
            .err()
            .unwrap_or_default();
        assert!(err.contains("trumf"), "{err}");
        let err = import("[game]\ntype = ").err().unwrap_or_default();
        assert!(err.contains("line 2"), "{err}");
    }
}
