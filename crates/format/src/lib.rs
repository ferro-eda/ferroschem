// SPDX-License-Identifier: GPL-3.0-or-later
//! Pluggable schematic and symbol format codecs (`L1 format`).
//!
//! Operates on `&str`, never the filesystem, so it compiles cleanly to
//! `wasm32-unknown-unknown`. Input is treated as hostile: every failure is a
//! typed [`FormatError`], never a panic.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

pub mod xschem;

use ferroschem_core::{Document, Schematic, Symbol};

/// Safety limits for format parsers, configurable by user settings or `Session`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    /// Maximum size in bytes for a single string or property block (default: 16 MiB).
    pub max_string_bytes: usize,
    /// Maximum number of records allowed in a single document (default: 1,000,000).
    pub max_records: usize,
    /// Maximum number of vertices allowed in a single polygon (default: 65,536).
    pub max_polygon_points: usize,
    /// Maximum nesting depth for inline `[...]` embedded symbol blocks (default: 32).
    pub max_nesting_depth: usize,
}

impl ParseLimits {
    pub const DEFAULT: Self = Self {
        max_string_bytes: 16 * 1024 * 1024,
        max_records: 1_000_000,
        max_polygon_points: 65_536,
        max_nesting_depth: 32,
    };
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Errors returned when parsing or serializing schematic/symbol formats.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    #[error("syntax error: {0}")]
    Syntax(String),
    #[error("input size ({size} bytes) exceeds maximum allowed ({max} bytes)")]
    InputTooLarge { size: usize, max: usize },
    #[error("not supported yet: {0}")]
    Unsupported(String),
}

/// Parse xschem `.sch` schematic text into a canonical [`Schematic`].
pub fn parse_schematic(text: &str) -> Result<Schematic, FormatError> {
    xschem::parse_schematic(text)
}

/// Parse xschem `.sym` symbol text into a canonical [`Symbol`].
pub fn parse_symbol(text: &str) -> Result<Symbol, FormatError> {
    xschem::parse_symbol(text)
}

/// Parse xschem `.sch` or `.sym` text into a [`Document`].
pub fn parse_document(text: &str) -> Result<Document, FormatError> {
    xschem::parse_document(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_schematic_and_symbol_headers() -> Result<(), FormatError> {
        let sch = parse_schematic("v {xschem version=3.4.0}\nN 0 0 100 0 {lab=GND}\n")?;
        assert_eq!(sch.wires.len(), 1);

        let sym = parse_symbol(
            "v {xschem version=3.4.0}\nK {type=resistor}\nB 5 -2.5 -2.5 2.5 2.5 {name=P dir=inout}\n",
        )?;
        assert_eq!(sym.pins.len(), 1);
        Ok(())
    }
}
