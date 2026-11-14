# Status & Session Context

> Single source of truth for session continuity. Every agent must read this at session start and
> update it at the end of every Teaching Phase.

## Developer Profile & Teaching Mode
- **Rust experience:** Learning Rust alongside building FerroSchem.
- **EDA / Schematic Capture experience:** Learning schematic capture and xschem internals alongside building FerroSchem.
- **Mode:** **Teaching Phases (Strict)** —
  1. **Concept & Mental Model:** Explain the EDA/xschem concept + the Rust pattern first.
  2. **Small, Idiomatic Code Slice:** Write one small, self-contained, testable slice (no black-box dumps).
  3. **Walkthrough & Check-in:** Walk through key lines, update `docs/`, update `plan/STATUS.md`, and pause for review/questions.
- *(Note for agents: If a new contributor starts a session without a profile here, ask their Rust and EDA comfort level first.)*

## Concepts Already Covered in `docs/`
- **EDA (`docs/01-eda-and-xschem/`):**
  - `01-what-is-schematic-capture.md`: Cosmetic graphics vs. electrical objects, symbols (`.sym`) vs. instances (`.sch`), pins, wires vs. nets, hierarchy, netlisting overview.
  - `02-xschem-file-format.md`: Canonical Domain Model vs. Pluggable Codecs (ADR 0002), how xschem's ASCII record format works (`v`, `G`, `K`, `V`, `S`, `F`, `E`, `L`, `B`, `A`, `P`, `T`, `N`, `C`, `[...]`), how `B 5 ...` boxes map to first-class `core::Pin`s, and a preview of FerroSchem's native S-expression format (`.fsch` / `.fsym`).
- **Rust (`docs/02-rust-for-ferroschem/`):**
  - `01-workspace-and-crates.md`: Cargo workspaces, library vs. binary crates, multi-file modules (`pub mod`, `crate::`, and `pub use` Facade re-exports), and strict `L0`–`L3` layering.
  - `02-structs-enums-and-errors.md`: `struct`, `#[derive]`, `Option<T>`, `#[must_use]`, `Result<T, E>`, `?`, and banning `unwrap()`.
  - `03-enums-and-collections.md`: Algebraic Data Types (`enum` variants carrying data: `LayerRole::Custom(u32)`, `PinDirection::Custom(String)`, `Document`), struct composition (`Graphics`), insertion-ordered `Properties` (`Vec<Property>`) vs. `HashMap`/`BTreeMap`, and `impl Into<String>`.
  - `04-strings-and-lexing.md`: Returning `(String, &str)` for zero-copy cursor slicing, UTF-8 safety with `.char_indices()` and `.len_utf8()`, `let Some(...) = ... else`, `matches!`, and `std::mem::take` in state machines.
  - `05-lifetimes-and-recursive-descent.md`: Struct lifetime annotations (`struct Cursor<'a>`), generic parsing with turbofish syntax (`.parse::<f64>()`), `.map_err(...)`, finite float checks, Rust 2024 `let`-chains, and recursive descent for `[...]` embedded symbols.
- **Architecture (`docs/03-architecture/01-command-engine-and-ai.md`):** Command-first engine (`Session::execute`), borrowing (`&mut self`, `&str`, `&Value`).

## Active Focus
- **Current Milestone:** **M1 — Canonical Core Model & Pluggable Format Codecs (`xschem` + `native` S-expressions)** (`crates/core`, `crates/format`).
- **Completed Milestones:**
  - M0 — Workspace scaffold, `xtask layers` check, `AGENTS.md` rules, and `docs/` learning handbook foundation.
  - ADR 0003 — Project renamed to **FerroSchem** under GitHub org `ferro-eda` (`https://github.com/ferro-eda/ferroschem`).

## Last Completed Slice
- **M1 — Teaching Phase 3A (Xschem `.sym` & `.sch` Record Parser in `crates/format/src/xschem/parser.rs`):**
  - Created [`crates/format/src/xschem/parser.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/parser.rs) implementing `parse_symbol`, `parse_schematic`, and `parse_document`.
  - Translates xschem records (`v`, `G`, `K`, `V`, `S`, `F`, `E`, `L`, `B`, `A`, `P`, `T`, `N`, `C`, and `[...]` inline embedded symbols) into canonical `ferroschem-core` types:
    - Converts `B 5 ...` boxes into first-class `core::Pin`s centered at `pos` with `size`, `name`, `dir`, and remaining `props`.
    - Extracts `layer=<n>` from `T` text properties into semantic `Text::layer` (`LayerRole::PinAnnotation`, `NetAnnotation`, `SimAnnotation`, `Text`).
    - Strips the `xschem version=... file_version=...` prefix line from `v {...}` into `comment`.
    - Enforces `ParseLimits` (`max_records` across the full `Cursor`, `max_nesting_depth` for `[...]` blocks, `max_polygon_points`, `max_string_bytes`) and finite `f64` checks on all hostile inputs.
  - Added [`docs/02-rust-for-ferroschem/05-lifetimes-and-recursive-descent.md`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/docs/02-rust-for-ferroschem/05-lifetimes-and-recursive-descent.md) and updated `ATTRIBUTION.md`.

## Immediate Next Slice
- **M1 — Teaching Phase 3B (Xschem `.sym` & `.sch` Record Writer & Library Round-Trip Tests in `crates/format/src/xschem/writer.rs`):**
  1. Implement `write_symbol`, `write_schematic`, and `write_document` in `crates/format/src/xschem/writer.rs` translating canonical `core::Symbol` (including `core::Pin` -> `B 5` via `Pin::bounds()`), `core::Schematic`, and `core::Document` back into xschem `.sym`/`.sch` text.
  2. Add round-trip integration tests against real `reference/xschem/xschem_library` symbol and schematic files (`res.sym`, `capa.sym`, `npn.sym`, `vsource.sym`, `Q1.sch`).
  3. Update `docs/` and `ATTRIBUTION.md`.

## Key Context & Rules to Remember
- **No Auto-Commit:** Never run `git commit` or `git push` without explicit user approval in the current turn.
- **Reference repos (`reference/xschem` and `reference/solidcraft`):** Gitignored locally for study/porting; record any ported logic in `ATTRIBUTION.md`.
- **Off-Repo Assets (`memory_bank/`):** Gitignored local directory tracking off-repo admin info (domain `ferroeda.org`, private repos, registries).
- **Checks:** `cargo fmt --check && cargo clippy --workspace --all-targets && cargo test --workspace && cargo xtask layers`.
