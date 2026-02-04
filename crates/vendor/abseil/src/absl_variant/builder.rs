//! Builder for constructing MultiVariant values.

use super::multi_variant::MultiVariant;

/// A variant builder for constructing variants.
#[derive(Debug, Default)]
pub struct VariantBuilder {
    variant: Option<MultiVariant>,
}

impl VariantBuilder {
    /// Creates a new variant builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the variant to an i32 value.
    pub fn i32(mut self, value: i32) -> Self {
        self.variant = Some(MultiVariant::I32(value));
        self
    }

    /// Sets the variant to an i64 value.
    pub fn i64(mut self, value: i64) -> Self {
        self.variant = Some(MultiVariant::I64(value));
        self
    }

    /// Sets the variant to a u32 value.
    pub fn u32(mut self, value: u32) -> Self {
        self.variant = Some(MultiVariant::U32(value));
        self
    }

    /// Sets the variant to a u64 value.
    pub fn u64(mut self, value: u64) -> Self {
        self.variant = Some(MultiVariant::U64(value));
        self
    }

    /// Sets the variant to an f32 value.
    pub fn f32(mut self, value: f32) -> Self {
        self.variant = Some(MultiVariant::F32(value));
        self
    }

    /// Sets the variant to an f64 value.
    pub fn f64(mut self, value: f64) -> Self {
        self.variant = Some(MultiVariant::F64(value));
        self
    }

    /// Sets the variant to a bool value.
    pub fn bool(mut self, value: bool) -> Self {
        self.variant = Some(MultiVariant::Bool(value));
        self
    }

    /// Sets the variant to a string value.
    pub fn string(mut self, value: alloc::string::String) -> Self {
        self.variant = Some(MultiVariant::String(value));
        self
    }

    /// Sets the variant from a string slice.
    pub fn str(self, value: &str) -> Self {
        self.string(value.to_string())
    }

    /// Builds the variant.
    pub fn build(self) -> Option<MultiVariant> {
        self.variant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_builder() {
        let builder = VariantBuilder::new();
        let variant = builder.i32(42).build();
        assert!(variant.is_some());
        assert!(variant.unwrap().is_i32());
    }

    #[test]
    fn test_variant_builder_string() {
        let builder = VariantBuilder::new();
        let variant = builder.string("hello".to_string()).build();
        assert!(variant.unwrap().is_string());
    }

    #[test]
    fn test_variant_builder_str() {
        let builder = VariantBuilder::new();
        let variant = builder.str("hello").build();
        assert!(variant.unwrap().is_string());
    }

    #[test]
    fn test_variant_builder_empty() {
        let builder = VariantBuilder::new();
        let variant = builder.build();
        assert!(variant.is_none());
    }
}
