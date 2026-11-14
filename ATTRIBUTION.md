# Attribution

FerroSchem is licensed GPL-3.0-or-later. This file records code, formats and ideas taken from
other projects. Add a row whenever ported or copied material lands in the repo.

| Area in FerroSchem | Source project | Source files | License | Notes |
|---|---|---|---|---|
| `.sch`/`.sym` data model (`crates/core`) & format spec (`docs/01-eda-and-xschem/02-xschem-file-format.md`) | xschem (Stefan Frederik Schippers) | `src/xschem.h`, `src/save.c`, `xschem_library/devices/res.sym` | GPL-2.0-or-later | Clean-room Rust data structures modeling xschem's ASCII record format and layer constants (`LAYER_WIRE`, `LAYER_PIN`, etc.). |
| Xschem `{...}` string & `key=value` property lexer/writer (`crates/format/src/xschem/tokens.rs`) | xschem (Stefan Frederik Schippers) | `src/save.c` (`load_ascii_string`, `save_ascii_string`), `src/token.c` (`get_tok_value`) | GPL-2.0-or-later | Clean-room Rust port of xschem's braced-string escaping (`\\`, `\{`, `\}`) and `TOK_*` attribute state machine. |
| Xschem `.sym`/`.sch` record parser (`crates/format/src/xschem/parser.rs`) | xschem (Stefan Frederik Schippers) | `src/save.c` (`read_xschem_file`, `load_sym_def`, `load_line`, `load_box`, `load_arc`, `load_polygon`, `load_text`, `load_wire`, `load_inst`) | GPL-2.0-or-later | Clean-room Rust parser translating xschem ASCII records (`v`, `G`, `K`, `V`, `S`, `F`, `E`, `L`, `B`, `A`, `P`, `T`, `N`, `C`, `[...]`) into canonical `ferroschem-core` types. |

## Inspiration & Planned Sources
- **xschem** ([Stefan Frederik Schippers](https://github.com/StefanSchippers/xschem), `GPL-2.0-or-later`):
  FerroSchem is inspired by xschem and aims for compatibility with its `.sch`/`.sym` file formats
  (`save.c`), netlisting (`netlist.c`, `spice_netlist.c`), and label/bus expansion (`expandlabel.y`).
- **SolidCraft** (`MIT OR Apache-2.0`): workspace conventions, command-engine architecture, and
  `xtask` patterns.

FerroSchem is an independent project and is not affiliated with or endorsed by xschem or its author.
