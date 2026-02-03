//! Monad-style operations for MultiVariant.

use super::multi_variant::MultiVariant;

impl MultiVariant {
    /// Maps the variant if it matches the predicate.
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(MultiVariant) -> MultiVariant,
    {
        f(self)
    }

    /// Maps the variant if it's of a specific type.
    pub fn map_if<F>(self, type_name: &str, f: F) -> Self
    where
        F: FnOnce(MultiVariant) -> MultiVariant,
    {
        if self.type_name() == type_name {
            f(self)
        } else {
            self
        }
    }

    /// Applies a function to the variant, returning a new variant.
    pub fn and_then<F>(self, f: F) -> MultiVariant
    where
        F: FnOnce(MultiVariant) -> MultiVariant,
    {
        f(self)
    }

    /// Returns the variant if it matches a predicate, or a default.
    pub fn filter_or_default<F>(self, predicate: F) -> Self
    where
        F: FnOnce(&MultiVariant) -> bool,
    {
        if predicate(&self) {
            self
        } else {
            MultiVariant::default()
        }
    }

    /// Combines two variants of the same type.
    pub fn combine<F>(self, other: MultiVariant, f: F) -> MultiVariant
    where
        F: FnOnce(MultiVariant, MultiVariant) -> MultiVariant,
    {
        f(self, other)
    }

    /// Attempts to add two numeric variants.
    pub fn try_add(self, other: MultiVariant) -> Option<MultiVariant> {
        match (self, other) {
            (MultiVariant::I32(a), MultiVariant::I32(b)) => Some(MultiVariant::I32(a.wrapping_add(b))),
            (MultiVariant::I64(a), MultiVariant::I64(b)) => Some(MultiVariant::I64(a.wrapping_add(b))),
            (MultiVariant::U32(a), MultiVariant::U32(b)) => Some(MultiVariant::U32(a.wrapping_add(b))),
            (MultiVariant::U64(a), MultiVariant::U64(b)) => Some(MultiVariant::U64(a.wrapping_add(b))),
            (MultiVariant::F32(a), MultiVariant::F32(b)) => Some(MultiVariant::F32(a + b)),
            (MultiVariant::F64(a), MultiVariant::F64(b)) => Some(MultiVariant::F64(a + b)),
            _ => None,
        }
    }

    /// Attempts to multiply two numeric variants.
    pub fn try_mul(self, other: MultiVariant) -> Option<MultiVariant> {
        match (self, other) {
            (MultiVariant::I32(a), MultiVariant::I32(b)) => Some(MultiVariant::I32(a.wrapping_mul(b))),
            (MultiVariant::I64(a), MultiVariant::I64(b)) => Some(MultiVariant::I64(a.wrapping_mul(b))),
            (MultiVariant::U32(a), MultiVariant::U32(b)) => Some(MultiVariant::U32(a.wrapping_mul(b))),
            (MultiVariant::U64(a), MultiVariant::U64(b)) => Some(MultiVariant::U64(a.wrapping_mul(b))),
            (MultiVariant::F32(a), MultiVariant::F32(b)) => Some(MultiVariant::F32(a * b)),
            (MultiVariant::F64(a), MultiVariant::F64(b)) => Some(MultiVariant::F64(a * b)),
            _ => None,
        }
    }

    /// Negates a numeric variant if possible.
    pub fn try_neg(self) -> Option<MultiVariant> {
        match self {
            MultiVariant::I32(v) => Some(MultiVariant::I32(v.wrapping_neg())),
            MultiVariant::I64(v) => Some(MultiVariant::I64(v.wrapping_neg())),
            MultiVariant::F32(v) => Some(MultiVariant::F32(-v)),
            MultiVariant::F64(v) => Some(MultiVariant::F64(-v)),
            _ => None,
        }
    }

    /// Attempts to compare two variants for ordering.
    pub fn try_compare(&self, other: &MultiVariant) -> Option<core::cmp::Ordering> {
        use core::cmp::Ordering;
        match (self, other) {
            (MultiVariant::I32(a), MultiVariant::I32(b)) => Some(a.cmp(b)),
            (MultiVariant::I64(a), MultiVariant::I64(b)) => Some(a.cmp(b)),
            (MultiVariant::U32(a), MultiVariant::U32(b)) => Some(a.cmp(b)),
            (MultiVariant::U64(a), MultiVariant::U64(b)) => Some(a.cmp(b)),
            (MultiVariant::F32(a), MultiVariant::F32(b)) => a.partial_cmp(b),
            (MultiVariant::F64(a), MultiVariant::F64(b)) => a.partial_cmp(b),
            (MultiVariant::Bool(a), MultiVariant::Bool(b)) => Some(a.cmp(b)),
            (MultiVariant::String(a), MultiVariant::String(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering;

    #[test]
    fn test_multi_variant_map() {
        let v = MultiVariant::I32(42);
        let mapped = v.map(|v| match v {
            MultiVariant::I32(i) => MultiVariant::I32(i * 2),
            _ => v,
        });
        assert_eq!(mapped.as_i32(), Some(&84));
    }

    #[test]
    fn test_multi_variant_map_if() {
        let v = MultiVariant::I32(42);
        let mapped = v.map_if("i32", |v| match v {
            MultiVariant::I32(i) => MultiVariant::I32(i * 2),
            _ => v,
        });
        assert_eq!(mapped.as_i32(), Some(&84));

        let mapped = v.map_if("string", |v| v);
        assert_eq!(mapped.as_i32(), Some(&42)); // Unchanged
    }

    #[test]
    fn test_multi_variant_filter_or_default() {
        let v = MultiVariant::I32(42);
        let filtered = v.filter_or_default(|v| v.as_i32().map_or(false, |i| *i > 40));
        assert!(filtered.is_i32());
        assert_eq!(filtered.as_i32(), Some(&42));

        let filtered = v.filter_or_default(|v| v.as_i32().map_or(false, |i| *i > 100));
        assert!(filtered.is_i32());
        assert_eq!(filtered.as_i32(), Some(&0)); // Default
    }

    #[test]
    fn test_multi_variant_try_add() {
        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::I32(8);
        let result = v1.try_add(v2);
        assert_eq!(result, Some(MultiVariant::I32(50)));

        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::String("hello".to_string());
        assert_eq!(v1.try_add(v2), None);
    }

    #[test]
    fn test_multi_variant_try_mul() {
        let v1 = MultiVariant::I32(6);
        let v2 = MultiVariant::I32(7);
        let result = v1.try_mul(v2);
        assert_eq!(result, Some(MultiVariant::I32(42)));
    }

    #[test]
    fn test_multi_variant_try_neg() {
        let v = MultiVariant::I32(42);
        let negated = v.try_neg();
        assert_eq!(negated, Some(MultiVariant::I32(-42)));

        let v = MultiVariant::String("hello".to_string());
        assert_eq!(v.try_neg(), None);
    }

    #[test]
    fn test_multi_variant_try_compare() {
        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::I32(50);
        assert_eq!(v1.try_compare(&v2), Some(Ordering::Less));

        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::I32(42);
        assert_eq!(v1.try_compare(&v2), Some(Ordering::Equal));

        let v1 = MultiVariant::String("abc".to_string());
        let v2 = MultiVariant::String("def".to_string());
        assert_eq!(v1.try_compare(&v2), Some(Ordering::Less));

        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::String("hello".to_string());
        assert_eq!(v1.try_compare(&v2), None);
    }
}
