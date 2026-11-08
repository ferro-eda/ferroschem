# ADR 0003 — Project Renamed to FerroSchem under FerroEDA

- **Status:** Accepted (supersedes name decision in ADR 0001)
- **Date:** 2026-11-08

## Context & Problem Statement
The original name **SchemCraft** collided with an established Minecraft schematic platform and toolset (`schemcraft.com`, `.schem` world formats), creating discoverability and SEO confusion. Furthermore, the name did not immediately convey its implementation language or place within a broader open-source semiconductor toolchain.

## Decision
1. **Name:** Renamed from **SchemCraft** to **FerroSchem**.
2. **Organization:** Hosted under the **FerroEDA** umbrella organization on GitHub: [`https://github.com/ferro-eda/ferroschem`](https://github.com/ferro-eda/ferroschem).
3. **Rust Heritage & Identity:** The prefix *Ferro-* (Latin *ferrum* / iron) is a recognized motif in the Rust ecosystem (symbolizing memory safety, reliability, and extreme performance), while *Schem* directly signals schematic capture and xschem compatibility.
4. **Crate Namespace:**
   - `ferroschem-core` (L0)
   - `ferroschem-format` (L1)
   - `ferroschem-netlist` (L1)
   - `ferroschem-engine` (L2)
   - `ferroschem-ui-egui` (L3)
   - `ferroschem-cli` (Apps)
5. **Suite Synergy:** FerroSchem sits alongside **FerroLayout** (a companion pure-Rust physical layout editor combining Magic's real-time DRC with KLayout's high-capacity GDSII/OASIS performance) sharing cross-probing and the same command engine architecture.
6. **Native Format Extension:** The native S-expression format (ADR 0002) uses `.fsch` (schematic) and `.fsym` (symbol).
