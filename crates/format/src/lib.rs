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

use ferroschem_core::Schematic;

/// Safety limits for format parsers, configurable by user settings or `Session`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    /// Maximum size in bytes for a single string or property block (default: 16 MiB).
    pub max_string_bytes: usize,
}

impl ParseLimits {
    pub const DEFAULT: Self = Self {
        max_string_bytes: 16 * 1024 * 1024,
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

/// Parse xschem schematic text.
pub fn parse_schematic(_text: &str) -> Result<Schematic, FormatError> {
    Err(FormatError::Unsupported("schematic parsing".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_is_not_implemented_yet_but_never_panics() {
        assert!(parse_schematic("v {xschem version=3.4.0}").is_err());
    }
}
