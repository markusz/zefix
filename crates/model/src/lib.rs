//! The scenario as the UI edits it, independent of any front end. The web UI (in `web/`)
//! edits a `Model` as JSON; everything with meaning — compiling tiles to query-language text,
//! share links, TOML, the simulation — happens here, so the UI and the TOML files share one
//! semantics. `api` is that surface as JSON strings, wrapped by `zefix-wasm` for the browser
//! and by the desktop app.

pub mod api;
pub mod compile;
pub mod info;
pub mod model;
pub mod share;
pub mod sim;
