// SPDX-License-Identifier: GPL-3.0-or-later
//! Xschem `.sym` and `.sch` ASCII record parser.
//!
//! Clean-room Rust port of the record grammar in xschem's `src/save.c`
//! (`read_xschem_file`, `load_sym_def`, `load_line`, `load_box`, `load_arc`,
//! `load_polygon`, `load_text`, `load_wire`, `load_inst`) by Stefan Frederik
//! Schippers (GPL-2.0-or-later).

use ferroschem_core::{
    Arc, Document, Graphics, Instance, LayerRole, Line, Pin, PinDirection, Point, Polygon,
    Properties, Rect, Rotation, Schematic, Symbol, Text, Wire,
};

use crate::xschem::{layer_from_xschem, parse_properties, read_braced_string};
use crate::{FormatError, ParseLimits};

/// Zero-copy cursor over an input `&str` slice.
///
/// The lifetime `'a` ties this cursor to the borrowed input string: advancing the
/// cursor only moves the slice pointer forward without copying the underlying text.
struct Cursor<'a> {
    rest: &'a str,
    records_read: usize,
}

impl<'a> Cursor<'a> {
    const fn new(input: &'a str) -> Self {
        Self {
            rest: input,
            records_read: 0,
        }
    }

    /// Skip spaces, tabs, carriage returns, and newlines.
    fn skip_whitespace(&mut self) {
        self.rest = self.rest.trim_start_matches([' ', '\t', '\r', '\n']);
    }

    /// Discard any remaining characters on the current line up to (and including) `'\n'`.
    fn skip_to_end_of_line(&mut self) {
        if let Some(newline_pos) = self.rest.find('\n') {
            self.rest = &self.rest[newline_pos + 1..];
        } else {
            self.rest = "";
        }
    }

    /// Read the next single-character record tag (`v`, `K`, `L`, `B`, `T`, `N`, `C`, etc.),
    /// returning `None` at end-of-input.
    fn next_tag(&mut self) -> Option<char> {
        self.skip_whitespace();
        let mut chars = self.rest.chars();
        let ch = chars.next()?;
        self.rest = chars.as_str();
        Some(ch)
    }

    /// Read the next whitespace-delimited token (used for integer and float fields).
    fn read_word(&mut self) -> Result<&'a str, FormatError> {
        self.rest = self.rest.trim_start_matches([' ', '\t', '\r']);
        let end = self
            .rest
            .find([' ', '\t', '\r', '\n', '{'])
            .unwrap_or(self.rest.len());
        if end == 0 {
            return Err(FormatError::Syntax(
                "expected numeric field, found end of line or '{'".into(),
            ));
        }
        let word = &self.rest[..end];
        self.rest = &self.rest[end..];
        Ok(word)
    }

    /// Parse a finite `f64` coordinate or scale factor.
    fn read_f64(&mut self) -> Result<f64, FormatError> {
        let word = self.read_word()?;
        let val = word
            .parse::<f64>()
            .map_err(|_| FormatError::Syntax(format!("invalid floating-point number '{word}'")))?;
        if !val.is_finite() {
            return Err(FormatError::Syntax(format!(
                "non-finite coordinate '{word}' is not allowed"
            )));
        }
        // Normalize -0.0 to 0.0 so parsed coordinates compare and format cleanly.
        Ok(if val == 0.0 { 0.0 } else { val })
    }

    /// Parse an unsigned 32-bit integer (e.g. CAD layer index).
    fn read_u32(&mut self) -> Result<u32, FormatError> {
        let word = self.read_word()?;
        word.parse::<u32>()
            .map_err(|_| FormatError::Syntax(format!("invalid unsigned integer '{word}'")))
    }

    /// Parse a `usize` count (e.g. polygon vertex count).
    fn read_usize(&mut self) -> Result<usize, FormatError> {
        let word = self.read_word()?;
        word.parse::<usize>()
            .map_err(|_| FormatError::Syntax(format!("invalid count '{word}'")))
    }

    /// Parse an xschem quarter-turn rotation code (`0`, `1`, `2`, `3`).
    fn read_rotation(&mut self) -> Result<Rotation, FormatError> {
        let word = self.read_word()?;
        let turns = word
            .parse::<u16>()
            .map_err(|_| FormatError::Syntax(format!("invalid rotation '{word}'")))?;
        Rotation::from_quarter_turns(turns).ok_or_else(|| {
            FormatError::Syntax(format!("rotation must be 0, 1, 2, or 3 (got {turns})"))
        })
    }

    /// Parse an xschem horizontal flip flag (`0` -> `false`, `1` -> `true`).
    fn read_flip(&mut self) -> Result<bool, FormatError> {
        let word = self.read_word()?;
        match word {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(FormatError::Syntax(format!(
                "flip flag must be 0 or 1 (got '{word}')"
            ))),
        }
    }

    /// Read a `{...}` braced string and advance the cursor past the closing `}`.
    fn read_braced(&mut self) -> Result<String, FormatError> {
        let (text, rest) = read_braced_string(self.rest)?;
        self.rest = rest;
        Ok(text)
    }

    /// Read a `{...}` braced string and parse its inner `key=value` pairs into [`Properties`].
    fn read_props(&mut self) -> Result<Properties, FormatError> {
        let raw = self.read_braced()?;
        parse_properties(&raw)
    }
}

/// Extract the user/license comment from an xschem `v {...}` header block.
///
/// Xschem writes `v {xschem version=3.4.8RC file_version=1.2\n<comment>}`.
/// We strip the leading `xschem version=...` line so `comment` holds only the
/// document's actual header comment.
fn extract_version_comment(raw: &str) -> String {
    let trimmed = raw.trim_start_matches([' ', '\t']);
    if trimmed.starts_with("xschem") {
        if let Some(nl) = trimmed.find('\n') {
            return trimmed[nl + 1..].to_owned();
        }
        return String::new();
    }
    raw.to_owned()
}

/// Intermediate accumulator capable of collecting records for either a [`Symbol`]
/// or a [`Schematic`].
#[derive(Default)]
struct RawRecordSet {
    comment: String,
    props: Properties,
    netlist_blocks: Properties,
    pins: Vec<Pin>,
    pin_rects: Vec<Rect>,
    graphics: Graphics,
    wires: Vec<Wire>,
    instances: Vec<Instance>,
}

impl RawRecordSet {
    fn into_symbol(mut self) -> Symbol {
        // Legacy pre-1.2 xschem symbols stored symbol attributes in `G {...}` when `K` was empty.
        if self.props.is_empty()
            && let Some(vhdl) = self.netlist_blocks.get("vhdl")
            && vhdl.contains("type=")
            && let Ok(legacy_props) = parse_properties(vhdl)
        {
            self.props = legacy_props;
            self.netlist_blocks.remove("vhdl");
        }

        // In a Symbol, any `N` wire records (from LCC schematics-as-symbols) become lines on Wire layer.
        for wire in self.wires {
            self.graphics.lines.push(Line {
                layer: LayerRole::Wire,
                a: wire.a,
                b: wire.b,
                props: wire.props,
            });
        }

        Symbol {
            pins: self.pins,
            graphics: self.graphics,
            props: self.props,
            comment: self.comment,
            netlist_blocks: self.netlist_blocks,
        }
    }

    fn into_schematic(mut self) -> Schematic {
        // In a Schematic, any `B 5` boxes are kept as graphic rectangles on `LayerRole::Pin`
        // (since schematic electrical ports are `C {ipin.sym}` / `opin.sym` / `iopin.sym` instances).
        self.graphics.rects.extend(self.pin_rects);

        Schematic {
            wires: self.wires,
            instances: self.instances,
            graphics: self.graphics,
            props: self.props,
            comment: self.comment,
            netlist_blocks: self.netlist_blocks,
        }
    }
}

/// Parse xschem records from `cursor` at nesting `depth` (`0` = top-level document,
/// `> 0` = inside a `[...]` inline embedded symbol block).
fn parse_records(cursor: &mut Cursor<'_>, depth: usize) -> Result<RawRecordSet, FormatError> {
    let mut out = RawRecordSet::default();

    while let Some(tag) = cursor.next_tag() {
        cursor.records_read += 1;
        if cursor.records_read > ParseLimits::DEFAULT.max_records {
            return Err(FormatError::InputTooLarge {
                size: cursor.records_read,
                max: ParseLimits::DEFAULT.max_records,
            });
        }

        match tag {
            ']' if depth > 0 => {
                cursor.skip_to_end_of_line();
                return Ok(out);
            }
            '#' => {
                cursor.skip_to_end_of_line();
                continue;
            }
            'v' => {
                let raw = cursor.read_braced()?;
                out.comment = extract_version_comment(&raw);
            }
            'K' => {
                out.props = cursor.read_props()?;
            }
            'G' => {
                let text = cursor.read_braced()?;
                if !text.is_empty() {
                    out.netlist_blocks.set("vhdl", text);
                }
            }
            'V' => {
                let text = cursor.read_braced()?;
                if !text.is_empty() {
                    out.netlist_blocks.set("verilog", text);
                }
            }
            'S' => {
                let text = cursor.read_braced()?;
                if !text.is_empty() {
                    out.netlist_blocks.set("spice", text);
                }
            }
            'F' => {
                let text = cursor.read_braced()?;
                if !text.is_empty() {
                    out.netlist_blocks.set("spectre", text);
                }
            }
            'E' => {
                let text = cursor.read_braced()?;
                if !text.is_empty() {
                    out.netlist_blocks.set("tedax", text);
                }
            }
            'L' => {
                let layer = layer_from_xschem(cursor.read_u32()?);
                let x1 = cursor.read_f64()?;
                let y1 = cursor.read_f64()?;
                let x2 = cursor.read_f64()?;
                let y2 = cursor.read_f64()?;
                let props = cursor.read_props()?;
                out.graphics.lines.push(Line {
                    layer,
                    a: Point::new(x1, y1),
                    b: Point::new(x2, y2),
                    props,
                });
            }
            'B' => {
                let raw_layer = cursor.read_u32()?;
                let x1 = cursor.read_f64()?;
                let y1 = cursor.read_f64()?;
                let x2 = cursor.read_f64()?;
                let y2 = cursor.read_f64()?;
                let mut props = cursor.read_props()?;

                if raw_layer == 5 {
                    // Save a copy of the raw rect in case this file is parsed as a Schematic,
                    // and also construct a first-class `core::Pin` for Symbol parsing.
                    out.pin_rects.push(Rect {
                        layer: LayerRole::Pin,
                        a: Point::new(x1, y1),
                        b: Point::new(x2, y2),
                        props: props.clone(),
                    });

                    let name = props.remove("name").unwrap_or_default();
                    let dir = props
                        .remove("dir")
                        .map(|d| PinDirection::from_str_lossless(&d))
                        .unwrap_or_default();
                    let pos = Point::new((x1 + x2) * 0.5, (y1 + y2) * 0.5);
                    let size = Point::new((x2 - x1).abs(), (y2 - y1).abs());
                    out.pins.push(Pin {
                        name,
                        dir,
                        pos,
                        size,
                        props,
                    });
                } else {
                    out.graphics.rects.push(Rect {
                        layer: layer_from_xschem(raw_layer),
                        a: Point::new(x1, y1),
                        b: Point::new(x2, y2),
                        props,
                    });
                }
            }
            'A' => {
                let layer = layer_from_xschem(cursor.read_u32()?);
                let x = cursor.read_f64()?;
                let y = cursor.read_f64()?;
                let radius = cursor.read_f64()?;
                let start_angle = cursor.read_f64()?;
                let arc_angle = cursor.read_f64()?;
                let props = cursor.read_props()?;
                out.graphics.arcs.push(Arc {
                    layer,
                    center: Point::new(x, y),
                    radius,
                    start_angle,
                    arc_angle,
                    props,
                });
            }
            'P' => {
                let layer = layer_from_xschem(cursor.read_u32()?);
                let npoints = cursor.read_usize()?;
                if npoints > ParseLimits::DEFAULT.max_polygon_points {
                    return Err(FormatError::InputTooLarge {
                        size: npoints,
                        max: ParseLimits::DEFAULT.max_polygon_points,
                    });
                }
                let mut points = Vec::with_capacity(npoints);
                for _ in 0..npoints {
                    let x = cursor.read_f64()?;
                    let y = cursor.read_f64()?;
                    points.push(Point::new(x, y));
                }
                let props = cursor.read_props()?;
                out.graphics.polygons.push(Polygon {
                    layer,
                    points,
                    props,
                });
            }
            'T' => {
                let text = cursor.read_braced()?;
                let x = cursor.read_f64()?;
                let y = cursor.read_f64()?;
                let rot = cursor.read_rotation()?;
                let flip = cursor.read_flip()?;
                let xscale = cursor.read_f64()?;
                let yscale = cursor.read_f64()?;
                let mut props = cursor.read_props()?;

                let layer = match props.remove("layer") {
                    Some(raw_layer) => {
                        let num = raw_layer.parse::<u32>().map_err(|_| {
                            FormatError::Syntax(format!("invalid text layer '{raw_layer}'"))
                        })?;
                        layer_from_xschem(num)
                    }
                    None => LayerRole::Text,
                };

                out.graphics.texts.push(Text {
                    text,
                    pos: Point::new(x, y),
                    rot,
                    flip,
                    xscale,
                    yscale,
                    layer,
                    props,
                });
            }
            'N' => {
                let x1 = cursor.read_f64()?;
                let y1 = cursor.read_f64()?;
                let x2 = cursor.read_f64()?;
                let y2 = cursor.read_f64()?;
                let props = cursor.read_props()?;
                out.wires.push(Wire {
                    a: Point::new(x1, y1),
                    b: Point::new(x2, y2),
                    props,
                });
            }
            'C' => {
                let symbol = cursor.read_braced()?;
                let x = cursor.read_f64()?;
                let y = cursor.read_f64()?;
                let rot = cursor.read_rotation()?;
                let flip = cursor.read_flip()?;
                let props = cursor.read_props()?;
                out.instances.push(Instance {
                    symbol,
                    pos: Point::new(x, y),
                    rot,
                    flip,
                    props,
                    embedded_symbol: None,
                });
            }
            '[' => {
                let next_depth = depth + 1;
                if next_depth > ParseLimits::DEFAULT.max_nesting_depth {
                    return Err(FormatError::InputTooLarge {
                        size: next_depth,
                        max: ParseLimits::DEFAULT.max_nesting_depth,
                    });
                }
                let Some(last_inst) = out.instances.last_mut() else {
                    return Err(FormatError::Syntax(
                        "encountered '[' embedded symbol block without a preceding 'C' instance"
                            .into(),
                    ));
                };
                cursor.skip_to_end_of_line();
                let embedded_records = parse_records(cursor, next_depth)?;
                last_inst.embedded_symbol = Some(embedded_records.into_symbol());
                continue;
            }
            other => {
                return Err(FormatError::Syntax(format!(
                    "unknown xschem record tag '{other}'"
                )));
            }
        }

        cursor.skip_to_end_of_line();
    }

    if depth > 0 {
        return Err(FormatError::Syntax(
            "unterminated '[' embedded symbol block (missing closing ']')".into(),
        ));
    }

    Ok(out)
}

/// Parse an xschem `.sym` text buffer into a canonical [`Symbol`].
pub fn parse_symbol(input: &str) -> Result<Symbol, FormatError> {
    let mut cursor = Cursor::new(input);
    let records = parse_records(&mut cursor, 0)?;
    Ok(records.into_symbol())
}

/// Parse an xschem `.sch` text buffer into a canonical [`Schematic`].
pub fn parse_schematic(input: &str) -> Result<Schematic, FormatError> {
    let mut cursor = Cursor::new(input);
    let records = parse_records(&mut cursor, 0)?;
    Ok(records.into_schematic())
}

/// Parse an xschem buffer into a [`Document`], automatically classifying it as a
/// [`Document::Symbol`] or [`Document::Schematic`] based on its records.
pub fn parse_document(input: &str) -> Result<Document, FormatError> {
    let mut cursor = Cursor::new(input);
    let records = parse_records(&mut cursor, 0)?;
    let is_symbol = records.wires.is_empty()
        && records.instances.is_empty()
        && (!records.pins.is_empty() || records.props.get("type").is_some());
    if is_symbol {
        Ok(Document::Symbol(records.into_symbol()))
    } else {
        Ok(Document::Schematic(records.into_schematic()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_res_sym_into_first_class_pins_and_semantic_layers() -> Result<(), FormatError> {
        let res_sym = r#"v {xschem version=3.4.8RC file_version=1.2
* Copyright (C) 1998-2026 Stefan Frederik Schippers
}
G {}
K {type=resistor
format="@name @pinlist @value m=@m"
template="name=R1
value=1k
footprint=1206
device=resistor
m=1"
}
V {}
S {}
F {}
E {}
L 4 0 20 0 30 {}
L 4 0 20 7.5 17.5 {}
B 5 -2.5 -32.5 2.5 -27.5 {name=P dir=inout propag=1 pinnumber=1 goto=1}
B 5 -2.5 27.5 2.5 32.5 {name=M dir=inout propag=0 pinnumber=2 goto=0}
T {@#0:pinnumber} -10 -26.25 0 1 0.2 0.2 {layer=13}
T {@#0:net_name} 10 -28.75 0 0 0.15 0.15 {layer=15 hide=instance}
T {@name} 15 -18.75 0 0 0.2 0.2 {}
T {@spice_get_current} -10 5 0 1 0.2 0.2 {layer=17}
"#;

        let sym = parse_symbol(res_sym)?;
        assert_eq!(
            sym.comment,
            "* Copyright (C) 1998-2026 Stefan Frederik Schippers\n"
        );
        assert_eq!(sym.props.get("type"), Some("resistor"));
        assert_eq!(sym.graphics.lines.len(), 2);
        assert_eq!(sym.graphics.lines[0].layer, LayerRole::SymbolBody);
        assert!(sym.graphics.rects.is_empty());

        // Layer-5 boxes became first-class Pins!
        assert_eq!(sym.pins.len(), 2);
        assert_eq!(sym.pins[0].name, "P");
        assert_eq!(sym.pins[0].dir, PinDirection::InOut);
        assert_eq!(sym.pins[0].pos, Point::new(0.0, -30.0));
        assert_eq!(sym.pins[0].size, Point::new(5.0, 5.0));
        assert_eq!(sym.pins[0].props.get("pinnumber"), Some("1"));
        assert_eq!(sym.pins[0].props.get("propag"), Some("1"));
        assert_eq!(
            sym.pins[0].bounds(),
            (Point::new(-2.5, -32.5), Point::new(2.5, -27.5))
        );

        // Text layers were extracted from `layer=` properties into `LayerRole`
        assert_eq!(sym.graphics.texts.len(), 4);
        assert_eq!(sym.graphics.texts[0].layer, LayerRole::PinAnnotation);
        assert!(sym.graphics.texts[0].props.is_empty());
        assert_eq!(sym.graphics.texts[1].layer, LayerRole::NetAnnotation);
        assert_eq!(sym.graphics.texts[1].props.get("hide"), Some("instance"));
        assert_eq!(sym.graphics.texts[2].layer, LayerRole::Text);
        assert_eq!(sym.graphics.texts[3].layer, LayerRole::SimAnnotation);
        Ok(())
    }

    #[test]
    fn parses_polygons_arcs_wires_instances_and_embedded_symbols() -> Result<(), FormatError> {
        let sch_text = r#"v {xschem version=3.4.4 file_version=1.2}
G {}
K {}
V {}
S {.tran 1n 100n}
E {}
A 4 0 0 15 270 360 {}
P 4 4 20 30 13.75 13.75 3.75 23.75 20 30 {fill=true}
N 440 -160 440 -100 {lab=P1}
C {res.sym} 190 -380 3 1 {name=Rbx value=3.723}
[
v {xschem version=3.4.4 file_version=1.2}
K {type=resistor}
G {}
V {}
S {}
E {}
L 4 0 -30 0 30 {}
B 5 -2.5 -32.5 2.5 -27.5 {name=P dir=inout}
]
"#;

        let doc = parse_document(sch_text)?;
        let Some(sch) = doc.as_schematic() else {
            return Err(FormatError::Syntax("expected schematic".into()));
        };

        assert_eq!(sch.netlist_blocks.get("spice"), Some(".tran 1n 100n"));
        assert_eq!(sch.graphics.arcs.len(), 1);
        assert_eq!(sch.graphics.arcs[0].radius, 15.0);
        assert_eq!(sch.graphics.polygons.len(), 1);
        assert_eq!(sch.graphics.polygons[0].points.len(), 4);
        assert_eq!(sch.wires.len(), 1);
        assert_eq!(sch.wires[0].label(), Some("P1"));
        assert_eq!(sch.instances.len(), 1);
        assert_eq!(sch.instances[0].symbol, "res.sym");
        assert_eq!(sch.instances[0].rot, Rotation::Deg270);
        assert!(sch.instances[0].flip);
        assert_eq!(sch.instances[0].instance_name(), Some("Rbx"));

        let Some(embedded) = &sch.instances[0].embedded_symbol else {
            return Err(FormatError::Syntax("expected embedded symbol".into()));
        };
        assert_eq!(embedded.props.get("type"), Some("resistor"));
        assert_eq!(embedded.pins.len(), 1);
        assert_eq!(embedded.pins[0].name, "P");
        Ok(())
    }

    #[test]
    fn rejects_malformed_or_hostile_records_without_panicking() {
        assert!(parse_schematic("L 4 0 20 0 not_a_float {}").is_err());
        assert!(parse_schematic("L 4 0 20 0 NaN {}").is_err());
        assert!(parse_schematic("C {res.sym} 0 0 9 0 {}").is_err());
        assert!(parse_schematic("C {res.sym} 0 0 0 2 {}").is_err());
        assert!(parse_schematic("P 4 99999999 0 0 {}").is_err());
        assert!(parse_schematic("[\nv {}\n]").is_err());
        assert!(parse_schematic("C {res.sym} 0 0 0 0 {}\n[\nv {}").is_err());

        let mut deeply_nested = String::new();
        for _ in 0..=ParseLimits::DEFAULT.max_nesting_depth {
            deeply_nested.push_str("C {sub.sym} 0 0 0 0 {}\n[\n");
        }
        assert_eq!(
            parse_schematic(&deeply_nested),
            Err(FormatError::InputTooLarge {
                size: ParseLimits::DEFAULT.max_nesting_depth + 1,
                max: ParseLimits::DEFAULT.max_nesting_depth,
            })
        );
    }
}
