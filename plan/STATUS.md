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
- **Architecture (`docs/03-architecture/01-command-engine-and-ai.md`):** Command-first engine (`Session::execute`), borrowing (`&mut self`, `&str`, `&Value`).

## Active Focus
- **Current Milestone:** **M1 — Canonical Core Model & Pluggable Format Codecs (`xschem` + `native` S-expressions)** (`crates/core`, `crates/format`).
- **Completed Milestones:**
  - M0 — Workspace scaffold, `xtask layers` check, `AGENTS.md` rules, and `docs/` learning handbook foundation.
  - ADR 0003 — Project renamed to **FerroSchem** under GitHub org `ferro-eda` (`https://github.com/ferro-eda/ferroschem`).

## Last Completed Slice
- **M1 — Teaching Phase 2 + Pure Format-Agnostic `core` Refactor (`crates/core`, `crates/format/src/xschem/`):**
  - Made `L0 ferroschem-core` 100% format-agnostic: replaced `BTreeMap` + `raw: Option<String>` with insertion-ordered `Properties` (`Vec<Property>` with in-place `set`), replaced `XschemHeaders` with format-agnostic `comment: String` and `netlist_blocks: Properties` on `Symbol`/`Schematic`, and moved `layer_from_xschem` / `layer_to_xschem` into [`crates/format/src/xschem/mod.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/mod.rs).
  - Created [`crates/format/src/xschem/tokens.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/tokens.rs) and [`crates/format/src/xschem/mod.rs`](file:///Users/loki/Workspace/GitHub_Repos/ferroeda/ferroschem/crates/format/src/xschem/mod.rs), implementing `read_braced_string`, `write_braced_string`, `parse_properties`, and `format_properties` with unit tests.
  - Updated `ADR 0002`, `docs/`, and `ATTRIBUTION.md`.

## Immediate Next Slice
- **M1 — Teaching Phase 3 (Xschem `.sym` & `.sch` Record Parser & Writer in `crates/format/src/xschem/`):**
  1. Implement the xschem record parser and writer (`crates/format/src/xschem/parser.rs` and `writer.rs`) translating `v`, `G`, `K`, `V`, `S`, `F`, `E`, `L`, `B` (including `B 5` ↔ first-class `core::Pin`), `A`, `P`, `T`, `N`, `C`, and `[...]` embedded symbols into `core::Symbol`, `core::Schematic`, and `core::Document`.
  2. Round-trip test against real `xschem_library` symbol and schematic files (e.g. `res.sym`, `capa.sym`, and sample `.sch` files).
  3. Update `docs/` and `ATTRIBUTION.md`.

## Key Context & Rules to Remember
- **No Auto-Commit:** Never run `git commit` or `git push` without explicit user approval in the current turn.
- **Reference repos (`reference/xschem` and `reference/solidcraft`):** Gitignored locally for study/porting; record any ported logic in `ATTRIBUTION.md`.
- **Off-Repo Assets (`memory_bank/`):** Gitignored local directory tracking off-repo admin info (domain `ferroeda.org`, private repos, registries).
- **Checks:** `cargo fmt --check && cargo clippy --workspace --all-targets && cargo test --workspace && cargo xtask layers`.
