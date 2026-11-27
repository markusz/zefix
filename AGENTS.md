# Ground rules

## Workspace
- `crates/core` (`zefix-core`): game rules, query language, simulation, statistics. No I/O, no output formatting.
- `crates/cli` (`zefix-cli`, binary `zefix`): argument parsing and rendering only; all logic lives in core.
- `crates/app` (`zefix-app`): egui UI for desktop and WebAssembly (`trunk serve` in `crates/app`). The visual model compiles to query-language text, so UI and TOML share one semantics. The browser has no threads: core is built without its `parallel` feature there and the runner steps the simulation inside frames; check both targets with `cargo clippy -p zefix-app --target wasm32-unknown-unknown -- -D warnings`. Layouts: `layout/mod.rs` picks `Layout::Desktop` or `Layout::Phone` by width, the only place that looks at it; `layout/desktop.rs` and `layout/phone.rs` compose the shared views, `layout/controls.rs` holds the controls both use. Views just wrap, and spacing comes from `theme::Density` via the inherited `Style`; query-language text (previews, text mode) shows only where `Density::shows_code`. Every text field (`TextEdit`, `DragValue`) passes its response to `keyboard::text_field`, or iPhones open no keyboard for it. Share links (`share.rs`) serialize the UI model as base64 JSON under a versioned key (`s1=`); bump it when a model change breaks old links. `cargo test -p zefix-app -- --ignored` renders screenshots to `target/ui-snapshots/` (needs a GPU).
- `crates/mcp` (`zefix-mcp`): MCP server over stdio; tool logic in `tools.rs` (tested without the protocol), protocol surface in `server.rs`. `reference` serves the README from `### Config` up to `## Entwicklung`, so keep those headings; it appends `docs/beispiele.md`, whose results must match `zefix run -n 1e6 --seed 42`.

## Rust
- Idiomatic Rust. No code that can panic in production paths: no `unwrap`, `expect`, `panic!`. Indexing only on fixed-size arrays with indices derived from enums or bounded by construction.
- Errors: `thiserror` for all core errors (`LangError`, `GameError`, `ScenarioError`, `SimError`) so front ends can match on them; `anyhow` only in front ends.
- Structs and enums instead of tuples; enums instead of bools.
- No magic values; name constants (`DECK_SIZE`, `HAND_SIZE`, `Z_95`, ...).
- Functions > 30 lines are an amber flag, > 50 a red flag. Flat lookup tables may exceed 30 lines.
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` before any change is done.

## Language
- Precise wording, no filler. Comments say why, not what; most fit on one line.
- No doc comment per field/variable unless the name is not self-explanatory.
- CLI output and user-facing errors in German, code and comments in English.

## Testing
- Simulation results are checked against closed-form hypergeometric results in `crates/core/tests/closed_form.rs`. New query features get such a test where an exact value is derivable.
