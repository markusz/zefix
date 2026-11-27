//! The front-end surface as JSON strings. Fallible calls return `{"ok": value}` or
//! `{"err": value}`, so TypeScript sees one shape and never an exception.

use crate::compile::{self, Problem};
use crate::model::{GameSetup, Model};
use crate::sim::Simulation;
use crate::{info, share};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Outcome<T, E> {
    Ok(T),
    Err(E),
}

pub fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn outcome<T: Serialize, E: Serialize>(result: Result<T, E>) -> String {
    json(&match result {
        Ok(value) => Outcome::Ok(value),
        Err(error) => Outcome::Err(error),
    })
}

fn parse_model(model: &str) -> Result<Model, String> {
    serde_json::from_str(model).map_err(|e| format!("Modell ungültig: {e}"))
}

pub fn default_model() -> String {
    json(&Model::default())
}

/// `setup` is a `GameSetup`; ok is `GameInfo`.
pub fn game_info(setup: &str) -> String {
    outcome(
        serde_json::from_str::<GameSetup>(setup)
            .map_err(|e| e.to_string())
            .and_then(|setup| setup.game().ok_or_else(|| "Ungültiges Spiel.".to_string()))
            .map(|game| info::game_info(&game)),
    )
}

/// ok is `Compiled`.
pub fn compile(model: &str) -> String {
    outcome(parse_model(model).map(|m| compile::compile(&m)))
}

/// ok is the URL fragment (without `#`) that restores the model.
pub fn link_fragment(model: &str) -> String {
    outcome(parse_model(model).map(|m| share::fragment(&m)))
}

/// The model in a link or fragment: `null` if there is none, else ok/err.
pub fn from_link(text: &str) -> String {
    match share::from_link(text) {
        None => "null".into(),
        Some(result) => outcome(result),
    }
}

pub fn to_toml(model: &str) -> String {
    outcome(parse_model(model).and_then(|m| {
        let game = m.game.game().ok_or("Ungültiges Spiel.")?;
        Ok(share::to_toml(&m, &game))
    }))
}

/// A share link or TOML scenario; ok is `{model, dropped}`.
pub fn import_text(text: &str) -> String {
    outcome(share::import(text))
}

/// Err is a `Problem` as JSON.
pub fn start(model: &str, iterations: u64) -> Result<Simulation, String> {
    let model = parse_model(model).map_err(|message| json(&Problem::Other { message }))?;
    Simulation::new(&model, iterations).map_err(|problem| json(&problem))
}

/// `Report` as JSON.
pub fn report(simulation: &Simulation) -> String {
    json(&simulation.report())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_are_tagged() {
        assert!(compile(&default_model()).starts_with(r#"{"ok":{"#));
        assert!(compile("{}").starts_with(r#"{"err":"Modell ungültig"#));
        assert_eq!(from_link("nichts"), "null");
        assert!(start("{}", 1).is_err());
    }

    /// The JSON the TypeScript types in `web/src/model.ts` mirror.
    #[test]
    fn model_json_shape() {
        let json = default_model();
        assert!(
            json.contains(r#""game":{"choice":"Sauspiel","suit":"gras"}"#),
            "{json}"
        );
        assert!(json.contains(r#""my_position":"First""#), "{json}");
        assert!(
            json.contains(
                r#""scope":"AnyOpponent","tiles":[{"Count":{"set":"Trumpf","op":{"AtLeast":4}}}]"#
            ),
            "{json}"
        );
    }
}
