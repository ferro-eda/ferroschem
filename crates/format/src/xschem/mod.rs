// SPDX-License-Identifier: GPL-3.0-or-later
//! Xschem `.sch` and `.sym` ASCII format codec.

pub mod tokens;

use ferroschem_core::LayerRole;

pub use tokens::{format_properties, parse_properties, read_braced_string, write_braced_string};

/// Convert an xschem integer CAD layer (`0..N`) into a semantic [`LayerRole`].
#[must_use]
pub const fn layer_from_xschem(layer: u32) -> LayerRole {
    match layer {
        1 => LayerRole::Wire,
        3 => LayerRole::Text,
        4 => LayerRole::SymbolBody,
        5 => LayerRole::Pin,
        13 => LayerRole::PinAnnotation,
        15 => LayerRole::NetAnnotation,
        17 => LayerRole::SimAnnotation,
        other => LayerRole::Custom(other),
    }
}

/// Convert a [`LayerRole`] back into its xschem integer CAD layer number.
#[must_use]
pub const fn layer_to_xschem(role: LayerRole) -> u32 {
    match role {
        LayerRole::Wire => 1,
        LayerRole::Text => 3,
        LayerRole::SymbolBody => 4,
        LayerRole::Pin => 5,
        LayerRole::PinAnnotation => 13,
        LayerRole::NetAnnotation => 15,
        LayerRole::SimAnnotation => 17,
        LayerRole::Custom(n) => n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xschem_layer_numbers_round_trip_through_layer_role() {
        for n in 0..30 {
            let role = layer_from_xschem(n);
            assert_eq!(layer_to_xschem(role), n);
        }
        assert_eq!(layer_from_xschem(4), LayerRole::SymbolBody);
        assert_eq!(layer_from_xschem(5), LayerRole::Pin);
        assert_eq!(layer_from_xschem(8), LayerRole::Custom(8));
    }
}
