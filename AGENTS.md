# Ground rules

## Workspace
- `crates/core` (`schafsim-core`): game rules, query language, simulation, statistics. No I/O, no output formatting.
- `crates/cli` (`schafsim-cli`, binary `schafsim`): argument parsing and rendering only; all logic lives in core.

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
