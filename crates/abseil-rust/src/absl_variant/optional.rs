//! OptionalVariant - A variant that can be empty.

use alloc::string::String;
use core::fmt;

use super::multi_variant::MultiVariant;

/// An optional variant that can be empty.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionalVariant {
    /// No value.
    Empty,
    /// A variant with a value.
    Some(MultiVariant),
}

impl OptionalVariant {
    /// Creates an empty optional variant.
    pub fn empty() -> Self {
        OptionalVariant::Empty
    }

    /// Creates an optional variant with a value.
    pub fn some(variant: MultiVariant) -> Self {
        OptionalVariant::Some(variant)
    }

    /// Returns true if the optional variant has a value.
    pub fn is_some(&self) -> bool {
        matches!(self, OptionalVariant::Some(_))
    }

    /// Returns true if the optional variant is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, OptionalVariant::Empty)
    }

    /// Returns the inner variant if present.
    pub fn get(&self) -> Option<&MultiVariant> {
        match self {
            OptionalVariant::Some(v) => Some(v),
            OptionalVariant::Empty => None,
        }
    }

    /// Returns the inner variant or a default.
    pub fn unwrap_or(&self, default: MultiVariant) -> MultiVariant {
        match self {
            OptionalVariant::Some(v) => v.clone(),
            OptionalVariant::Empty => default,
        }
    }

    /// Maps the inner variant if present.
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(MultiVariant) -> MultiVariant,
    {
        match self {
            OptionalVariant::Some(v) => OptionalVariant::Some(f(v)),
            OptionalVariant::Empty => OptionalVariant::Empty,
        }
    }

    /// Filters the optional variant.
    pub fn filter<F>(self, predicate: F) -> Self
    where
        F: FnOnce(&MultiVariant) -> bool,
    {
        match self {
            OptionalVariant::Some(v) if predicate(&v) => OptionalVariant::Some(v),
            _ => OptionalVariant::Empty,
        }
    }
}

impl Default for OptionalVariant {
    fn default() -> Self {
        OptionalVariant::Empty
    }
}

impl fmt::Display for OptionalVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionalVariant::Empty => write!(f, "(empty)"),
            OptionalVariant::Some(v) => write!(f, "{}", v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optional_variant_empty() {
        let ov = OptionalVariant::empty();
        assert!(ov.is_empty());
        assert!(!ov.is_some());
        assert_eq!(ov.get(), None);
    }

    #[test]
    fn test_optional_variant_some() {
        let ov = OptionalVariant::some(MultiVariant::I32(42));
        assert!(ov.is_some());
        assert!(!ov.is_empty());
        assert!(ov.get().is_some());
    }

    #[test]
    fn test_optional_variant_unwrap_or() {
        let ov = OptionalVariant::empty();
        let default = MultiVariant::I32(100);
        let result = ov.unwrap_or(default.clone());
        assert_eq!(result, default);

        let ov = OptionalVariant::some(MultiVariant::I32(42));
        let result = ov.unwrap_or(default);
        assert_eq!(result.as_i32(), Some(&42));
    }

    #[test]
    fn test_optional_variant_default() {
        let ov = OptionalVariant::default();
        assert!(ov.is_empty());
    }

    #[test]
    fn test_optional_variant_display() {
        let ov = OptionalVariant::empty();
        assert_eq!(format!("{}", ov), "(empty)");

        let ov = OptionalVariant::some(MultiVariant::I32(42));
        assert_eq!(format!("{}", ov), "42");
    }

    #[test]
    fn test_optional_variant_clone() {
        let ov1 = OptionalVariant::some(MultiVariant::I32(42));
        let ov2 = ov1.clone();
        assert_eq!(ov1, ov2);
    }

    #[test]
    fn test_optional_variant_map() {
        let ov = OptionalVariant::some(MultiVariant::I32(42));
        let mapped = ov.map(|v| match v {
            MultiVariant::I32(i) => MultiVariant::I32(i * 2),
            _ => v,
        });
        assert!(mapped.is_some());
        assert_eq!(mapped.get().unwrap().as_i32(), Some(&84));
    }

    #[test]
    fn test_optional_variant_filter() {
        let ov = OptionalVariant::some(MultiVariant::I32(42));
        let filtered = ov.filter(|v| v.as_i32().map_or(false, |i| *i > 40));
        assert!(filtered.is_some());

        let ov = OptionalVariant::some(MultiVariant::I32(42));
        let filtered = ov.filter(|v| v.as_i32().map_or(false, |i| *i > 100));
        assert!(filtered.is_empty());
    }
}
