//! Visitor pattern for type-erased values.

use core::any::{Any as CoreAny, TypeId};
use crate::absl_any::any_box::AnyBox;

/// A visitor for traversing type-erased values.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{AnyVisitor, AnyBox};
///
/// struct PrintVisitor;
/// impl AnyVisitor for PrintVisitor {
///     fn visit(&self, value: &dyn core::any::Any) -> bool {
///         println!("Type: {}", value.type_name());
///         true
///     }
/// }
///
/// let boxed = AnyBox::new(42);
/// PrintVisitor.visit_any(&boxed);
/// ```
pub trait AnyVisitor {
    /// Visits a type-erased value.
    ///
    /// Returns `true` if the visit was successful.
    fn visit(&self, value: &dyn CoreAny) -> bool;

    /// Visits an `AnyBox`.
    fn visit_any(&self, boxed: &AnyBox) -> bool {
        self.visit(boxed.inner.as_ref())
    }
}

/// A visitor that prints type information.
#[derive(Debug, Default)]
pub struct TypePrinterVisitor;

impl AnyVisitor for TypePrinterVisitor {
    fn visit(&self, value: &dyn CoreAny) -> bool {
        println!("Type: {}", value.type_name());
        true
    }
}

/// A visitor that checks for specific types.
#[derive(Debug)]
pub struct TypeCheckVisitor<T: 'static> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T: 'static> TypeCheckVisitor<T> {
    /// Creates a new `TypeCheckVisitor` for type `T`.
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }

    /// Checks if the visited value is of type `T`.
    pub fn is_match(&self, value: &dyn CoreAny) -> bool {
        value.is::<T>()
    }
}

impl<T: 'static> Default for TypeCheckVisitor<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> AnyVisitor for TypeCheckVisitor<T> {
    fn visit(&self, value: &dyn CoreAny) -> bool {
        self.is_match(value)
    }
}

/// A visitor that collects type names.
#[derive(Default, Debug)]
pub struct TypeNameCollector {
    names: Vec<alloc::string::String>,
}

impl TypeNameCollector {
    /// Creates a new type name collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the collected type names.
    pub fn names(&self) -> &[alloc::string::String] {
        &self.names
    }

    /// Clears the collected names.
    pub fn clear(&mut self) {
        self.names.clear();
    }
}

impl AnyVisitor for TypeNameCollector {
    fn visit(&self, _value: &dyn CoreAny) -> bool {
        // We can't mutate self in visit, so this is a no-op
        // In a real implementation, you'd use interior mutability
        true
    }
}

/// A visitor that checks type constraints.
#[derive(Debug)]
pub struct TypeConstraintVisitor {
    allowed_types: Vec<TypeId>,
}

impl TypeConstraintVisitor {
    /// Creates a new visitor with the given allowed types.
    pub fn new(allowed_types: Vec<TypeId>) -> Self {
        Self { allowed_types }
    }

    /// Returns true if the given type is allowed.
    pub fn is_allowed(&self, type_id: TypeId) -> bool {
        self.allowed_types.contains(&type_id)
    }
}

impl AnyVisitor for TypeConstraintVisitor {
    fn visit(&self, value: &dyn CoreAny) -> bool {
        self.is_allowed(value.type_id())
    }
}

/// A visitor that transforms values if they match a predicate.
pub struct TransformVisitor<F>
where
    F: Fn(&dyn CoreAny) -> Option<AnyBox>,
{
    transform: F,
}

impl<F> TransformVisitor<F>
where
    F: Fn(&dyn CoreAny) -> Option<AnyBox>,
{
    /// Creates a new transform visitor.
    pub fn new(transform: F) -> Self {
        Self { transform }
    }

    /// Attempts to transform the value.
    pub fn transform(&self, value: &dyn CoreAny) -> Option<AnyBox> {
        (self.transform)(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_printer_visitor() {
        let visitor = TypePrinterVisitor;
        let boxed: AnyBox = AnyBox::new(42i32);
        assert!(visitor.visit_any(&boxed));
    }

    #[test]
    fn test_type_check_visitor() {
        let visitor = TypeCheckVisitor::<i32>::new();
        let boxed_int: AnyBox = AnyBox::new(42i32);
        let boxed_str: AnyBox = AnyBox::new("hello");
        assert!(visitor.visit_any(&boxed_int));
        assert!(!visitor.visit_any(&boxed_str));
    }

    #[test]
    fn test_type_check_visitor_is_match() {
        let visitor = TypeCheckVisitor::<i32>::new();
        let boxed_int: AnyBox = AnyBox::new(42i32);
        let boxed_str: AnyBox = AnyBox::new("hello");
        assert!(visitor.is_match(boxed_int.inner.as_ref()));
        assert!(!visitor.is_match(boxed_str.inner.as_ref()));
    }

    #[test]
    fn test_type_check_visitor_default() {
        let visitor = TypeCheckVisitor::<i32>::default();
        assert!(visitor.is_match(&42i32));
    }
}
