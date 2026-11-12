# ADR 0002 — Canonical Core Domain Model and Pluggable Format Codecs

## Context
Initially during M1, `crates/core` modeled xschem's `.sch`/`.sym` file records almost 1-to-1
(e.g. representing electrical pins as rectangles on magic layer number `5`, using raw integer
layer numbers, and using a single `Schematic` struct for both schematics and symbols).

While compatible with xschem, tightly coupling `L0 ferroschem-core` to xschem's 1990s file
format quirks would force `netlist`, `engine`, `ui-egui`, and AI commands to deal with format
quirks and would make supporting FerroSchem's own native format or other industry formats
(KiCad, LTspice, EDIF) difficult.

## Decision
1. **100% Format-Agnostic Canonical Domain Model in `L0 ferroschem-core`:**
   - **Zero Format Leakage in `core`:** `crates/core` contains no format-specific struct names,
     fields, or helper methods (no `XschemHeaders`, no `Properties::raw`, and no format-specific
     layer conversion functions).
   - **Insertion-Ordered `Properties` (`Vec<Property>`):** `Properties` preserves exact insertion
     order and updates existing keys in place, guaranteeing deterministic subcircuit parameter
     ordering in netlists (`M3`) and clean diffs across all file formats without caching raw
     format strings.
   - **Semantic Layers (`LayerRole`):** `core` uses semantic layer roles (`Wire`, `Text`,
     `SymbolBody`, `Pin`, `PinAnnotation`, `NetAnnotation`, `SimAnnotation`, plus `Custom(u32)`
     to preserve arbitrary numbered layers losslessly). Each codec in `crates/format` maps its own
     layer identifiers to/from `LayerRole`.
   - **First-Class `Pin` and `PinDirection`:** Electrical pins on a symbol are represented by a
     dedicated `Pin` struct (`name`, `dir`, `pos`, `size`, `props`), not as magic-layer boxes.
   - **Distinct `Schematic` and `Symbol` Structs + `enum Document`:**
     - `Symbol` holds `pins: Vec<Pin>`, `graphics: Graphics`, `props: Properties`,
       `comment: String`, and `netlist_blocks: Properties`.
     - `Schematic` holds `wires: Vec<Wire>`, `instances: Vec<Instance>`, `graphics: Graphics`,
       `props: Properties`, `comment: String`, and `netlist_blocks: Properties`.
     - `enum Document { Schematic(Schematic), Symbol(Symbol) }` wraps either document kind.
2. **Pluggable Format Codecs in `L1 ferroschem-format`:**
   - `format::xschem`: Parses and writes xschem `.sch` and `.sym` files and translates to/from
     `ferroschem_core::Document` (converting layer-5 boxes into `core::Pin`s, mapping xschem layer
     numbers via `format::xschem::{layer_from_xschem, layer_to_xschem}`, and mapping `v`/`K`/`G`/`V`/`S`/`F`/`E`
     headers into `comment`, `props`, and `netlist_blocks`).
   - `format::native`: FerroSchem's own native format (`.fsch` / `.fsym`) using **KiCad-style
     S-expressions** (one line per wire/pin/shape for clean `git diff`s, named fields for
     forward compatibility, zero external dependencies).
   - JSON serialization remains available out of the box via `serde` on all `core` types for
     MCP, CLI, and web interchange.
   - Future industry format parsers (KiCad, LTspice, EDIF) will live as peer codec modules in
     `crates/format` translating into `ferroschem_core::Document`.
