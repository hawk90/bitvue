//! Visitor pattern for variant pattern matching.

use super::multi_variant::MultiVariant;

/// A variant visitor trait for pattern matching.
pub trait Visitor<R> {
    /// Visits an i32 value.
    fn visit_i32(&mut self, value: &i32) -> R;

    /// Visits an i64 value.
    fn visit_i64(&mut self, value: &i64) -> R;

    /// Visits a u32 value.
    fn visit_u32(&mut self, value: &u32) -> R;

    /// Visits a u64 value.
    fn visit_u64(&mut self, value: &u64) -> R;

    /// Visits an f32 value.
    fn visit_f32(&mut self, value: &f32) -> R;

    /// Visits an f64 value.
    fn visit_f64(&mut self, value: &f64) -> R;

    /// Visits a bool value.
    fn visit_bool(&mut self, value: &bool) -> R;

    /// Visits a string value.
    fn visit_string(&mut self, value: &str) -> R;
}

/// Applies a visitor to a variant.
pub fn match_variant<R>(variant: &MultiVariant, visitor: &mut impl Visitor<R>) -> R {
    match variant {
        MultiVariant::I32(v) => visitor.visit_i32(v),
        MultiVariant::I64(v) => visitor.visit_i64(v),
        MultiVariant::U32(v) => visitor.visit_u32(v),
        MultiVariant::U64(v) => visitor.visit_u64(v),
        MultiVariant::F32(v) => visitor.visit_f32(v),
        MultiVariant::F64(v) => visitor.visit_f64(v),
        MultiVariant::Bool(v) => visitor.visit_bool(v),
        MultiVariant::String(v) => visitor.visit_string(v),
    }
}

/// A visitor that converts variants to a common type.
pub struct ConvertVisitor<T, F> {
    converter: F,
    _phantom: core::marker::PhantomData<T>,
}

impl<T, F> ConvertVisitor<T, F> {
    /// Creates a new convert visitor.
    pub fn new(converter: F) -> Self {
        Self {
            converter,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T, F> Visitor<Option<T>> for ConvertVisitor<T, F>
where
    F: Fn(&MultiVariant) -> Option<T>,
{
    fn visit_i32(&mut self, value: &i32) -> Option<T> {
        (self.converter)(&MultiVariant::I32(*value))
    }

    fn visit_i64(&mut self, value: &i64) -> Option<T> {
        (self.converter)(&MultiVariant::I64(*value))
    }

    fn visit_u32(&mut self, value: &u32) -> Option<T> {
        (self.converter)(&MultiVariant::U32(*value))
    }

    fn visit_u64(&mut self, value: &u64) -> Option<T> {
        (self.converter)(&MultiVariant::U64(*value))
    }

    fn visit_f32(&mut self, value: &f32) -> Option<T> {
        (self.converter)(&MultiVariant::F32(*value))
    }

    fn visit_f64(&mut self, value: &f64) -> Option<T> {
        (self.converter)(&MultiVariant::F64(*value))
    }

    fn visit_bool(&mut self, value: &bool) -> Option<T> {
        (self.converter)(&MultiVariant::Bool(*value))
    }

    fn visit_string(&mut self, value: &str) -> Option<T> {
        (self.converter)(&MultiVariant::String(value.to_string()))
    }
}

/// A visitor that collects variant types.
#[derive(Debug, Default)]
pub struct TypeCollector {
    types: Vec<&'static str>,
}

impl TypeCollector {
    /// Creates a new type collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the collected types.
    pub fn types(&self) -> &[&str] {
        &self.types
    }

    /// Clears the collected types.
    pub fn clear(&mut self) {
        self.types.clear();
    }
}

impl Visitor<()> for TypeCollector {
    fn visit_i32(&mut self, _value: &i32) {
        self.types.push("i32");
    }

    fn visit_i64(&mut self, _value: &i64) {
        self.types.push("i64");
    }

    fn visit_u32(&mut self, _value: &u32) {
        self.types.push("u32");
    }

    fn visit_u64(&mut self, _value: &u64) {
        self.types.push("u64");
    }

    fn visit_f32(&mut self, _value: &f32) {
        self.types.push("f32");
    }

    fn visit_f64(&mut self, _value: &f64) {
        self.types.push("f64");
    }

    fn visit_bool(&mut self, _value: &bool) {
        self.types.push("bool");
    }

    fn visit_string(&mut self, _value: &str) {
        self.types.push("string");
    }
}

/// Returns the type of a variant as a string.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_type, MultiVariant};
///
/// let v = MultiVariant::I32(42);
/// assert_eq!(variant_type(&v), "i32");
/// ```
pub fn variant_type(variant: &MultiVariant) -> &'static str {
    variant.type_name()
}

/// Returns true if two variants have the same type.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{same_variant_type, MultiVariant};
///
/// let v1 = MultiVariant::I32(42);
/// let v2 = MultiVariant::I32(100);
/// assert!(same_variant_type(&v1, &v2));
/// ```
pub fn same_variant_type(a: &MultiVariant, b: &MultiVariant) -> bool {
    a.type_name() == b.type_name()
}

/// Attempts to convert a variant to a specific type.
///
/// # Examples
///
/// ```
/// use abseil::absl_variant::{variant_convert, MultiVariant};
///
/// let v = MultiVariant::I32(42);
/// let result: Option<i64> = variant_convert(&v);
/// assert_eq!(result, Some(42));
/// ```
pub fn variant_convert<T: TryFrom<MultiVariant>>(variant: &MultiVariant) -> Option<T> {
    T::try_from(variant.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test visitor implementation.
    struct TestVisitor;

    impl Visitor<String> for TestVisitor {
        fn visit_i32(&mut self, value: &i32) -> String {
            format!("i32:{}", value)
        }

        fn visit_i64(&mut self, value: &i64) -> String {
            format!("i64:{}", value)
        }

        fn visit_u32(&mut self, value: &u32) -> String {
            format!("u32:{}", value)
        }

        fn visit_u64(&mut self, value: &u64) -> String {
            format!("u64:{}", value)
        }

        fn visit_f32(&mut self, value: &f32) -> String {
            format!("f32:{}", value)
        }

        fn visit_f64(&mut self, value: &f64) -> String {
            format!("f64:{}", value)
        }

        fn visit_bool(&mut self, value: &bool) -> String {
            format!("bool:{}", value)
        }

        fn visit_string(&mut self, value: &str) -> String {
            format!("string:{}", value)
        }
    }

    #[test]
    fn test_match_variant() {
        let v = MultiVariant::I32(42);
        let mut visitor = TestVisitor;
        let result = match_variant(&v, &mut visitor);
        assert_eq!(result, "i32:42");
    }

    #[test]
    fn test_convert_visitor() {
        let v = MultiVariant::I32(42);
        let mut visitor = ConvertVisitor::new(|v: &MultiVariant| {
            v.as_i32().map(|i| *i as i64)
        });
        let result = match_variant(&v, &mut visitor);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_type_collector() {
        let mut collector = TypeCollector::new();
        match_variant(&MultiVariant::I32(42), &mut collector);
        match_variant(&MultiVariant::String("hello".to_string()), &mut collector);
        assert_eq!(collector.types(), &["i32", "string"]);
    }

    #[test]
    fn test_variant_type() {
        let v = MultiVariant::I32(42);
        assert_eq!(variant_type(&v), "i32");
    }

    #[test]
    fn test_same_variant_type() {
        let v1 = MultiVariant::I32(42);
        let v2 = MultiVariant::I32(100);
        assert!(same_variant_type(&v1, &v2));

        let v3 = MultiVariant::I64(42);
        assert!(!same_variant_type(&v1, &v3));
    }
}
