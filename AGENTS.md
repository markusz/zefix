# Ground rules

## Workspace
- `crates/core` (`zefix-core`): game rules, query language, simulation, statistics. No I/O, no output formatting.
- `crates/cli` (`zefix-cli`, binary `zefix`): argument parsing and rendering only; all logic lives in core.
- `crates/model` (`zefix-model`): the UI model (`model.rs`, serde JSON = share-link format), compiling it to query-language text, share links and TOML, game info for drawing, a stepped `Simulation`. `api.rs` is the whole front-end surface as JSON strings (`{"ok"}`/`{"err"}`); `zefix-wasm` and `zefix-desktop` only wrap it. Share links use a versioned key (`s1=`); bump it when a model change breaks old links.
- `crates/wasm` (`zefix-wasm`): `zefix-model` for the browser via wasm-bindgen (pinned exactly; `web/scripts/build-wasm.sh` fetches the matching CLI). Check with `cargo clippy -p zefix-wasm --target wasm32-unknown-unknown -- -D warnings`.
- `crates/desktop` (`zefix-desktop`): Tauri shell around `web/dist`. Only simulations run natively (all cores), streamed as the same messages the browser's worker sends.
- `web/`: React + TypeScript + Vite. `src/model.ts` mirrors the serde JSON of the Rust model exactly and holds only what drawing and editing need; meaning stays in Rust. `src/engine.ts` picks the Web Worker or Tauri. One layout for all widths: CSS turns the columns into tabs below 700 px and hides `.code-only` (query-language text) there. `npm run check` type-checks; dependencies are pinned exactly.
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
