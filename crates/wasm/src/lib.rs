//! `zefix_model::api` for the browser. Each export forwards to it; see there for the shapes.

use wasm_bindgen::prelude::wasm_bindgen;
use zefix_model::api;

#[wasm_bindgen(js_name = defaultModel)]
pub fn default_model() -> String {
    api::default_model()
}

#[wasm_bindgen(js_name = gameInfo)]
pub fn game_info(setup: &str) -> String {
    api::game_info(setup)
}

#[wasm_bindgen]
pub fn compile(model: &str) -> String {
    api::compile(model)
}

#[wasm_bindgen(js_name = linkFragment)]
pub fn link_fragment(model: &str) -> String {
    api::link_fragment(model)
}

#[wasm_bindgen(js_name = fromLink)]
pub fn from_link(text: &str) -> String {
    api::from_link(text)
}

#[wasm_bindgen(js_name = toToml)]
pub fn to_toml(model: &str) -> String {
    api::to_toml(model)
}

#[wasm_bindgen(js_name = importText)]
pub fn import_text(text: &str) -> String {
    api::import_text(text)
}

#[wasm_bindgen]
pub struct Simulation(zefix_model::sim::Simulation);

#[wasm_bindgen]
impl Simulation {
    /// Throws a `Problem` as JSON if the model cannot run.
    pub fn start(model: &str, iterations: u32) -> Result<Simulation, String> {
        api::start(model, u64::from(iterations)).map(Simulation)
    }

    /// Simulates up to `chunks` more chunks; true once finished. Throws if no deal fits.
    pub fn advance(&mut self, chunks: u32) -> Result<bool, String> {
        self.0.advance(u64::from(chunks))
    }

    /// `Report` as JSON.
    pub fn report(&self) -> String {
        api::report(&self.0)
    }
}
