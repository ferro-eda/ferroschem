// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic, insertion-ordered key-value [`Properties`].

use serde::{Deserialize, Serialize};

/// A single key-value attribute entry inside [`Properties`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub key: String,
    pub value: String,
}

/// An insertion-ordered key-value property map attached to a document, pin, wire,
/// instance, or graphic shape.
///
/// Preserves the exact order in which keys were added (critical for subcircuit
/// parameter lists in SPICE/Verilog netlisting and clean diffs across all file
/// formats), while updating existing keys in place when [`Properties::set`] is called.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Properties {
    entries: Vec<Property>,
}

impl Properties {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Look up a property value by key (e.g. `props.get("name")` or `props.get("value")`).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.as_str())
    }

    /// Insert a new key-value pair at the end, or update the value in place if `key`
    /// already exists (preserving its position in the property order).
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        if let Some(existing) = self.entries.iter_mut().find(|entry| entry.key == key) {
            existing.value = value;
        } else {
            self.entries.push(Property { key, value });
        }
    }

    /// Remove a key-value pair by key, returning its previous value if present,
    /// while preserving the relative order of all remaining entries.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        let idx = self.entries.iter().position(|entry| entry.key == key)?;
        Some(self.entries.remove(idx).value)
    }

    /// Iterate over `(&str, &str)` key-value pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|entry| (entry.key.as_str(), entry.value.as_str()))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_preserve_insertion_order_and_update_in_place() {
        let mut props = Properties::new();
        props.set("name", "M1");
        props.set("W", "10u");
        props.set("L", "0.18u");
        props.set("nf", "1");

        // Updating "W" in place must not move it to the end.
        props.set("W", "20u");

        let pairs: Vec<(&str, &str)> = props.iter().collect();
        assert_eq!(
            pairs,
            vec![("name", "M1"), ("W", "20u"), ("L", "0.18u"), ("nf", "1")]
        );
        assert_eq!(props.remove("L"), Some("0.18u".into()));
        assert_eq!(props.len(), 3);
    }
}
