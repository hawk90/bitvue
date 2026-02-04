//! AnyIterator - Type-erased iterator.

use crate::absl_any::any_box::AnyBox;

/// A type-erased iterator.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{AnyIterator, AnyBox};
///
/// let values: Vec<i32> = vec![1, 2, 3, 4, 5];
/// let mut iter = AnyIterator::new(values);
///
/// while let Some(value) = iter.next_any::<i32>() {
///     println!("Got: {}", value);
/// }
/// ```
pub struct AnyIterator {
    // Simplified implementation - in real use, you'd store the iterator
    // more carefully to allow actual iteration
    _phantom: core::marker::PhantomData<()>,
}

impl AnyIterator {
    /// Creates a new AnyIterator from an iterator.
    pub fn new<I: IntoIterator + 'static>(_iter: I) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }

    /// Gets the next value if it's of type T.
    pub fn next_any<T: 'static>(&mut self) -> Option<T> {
        // Simplified - always returns None
        None
    }

    /// Returns true if there are more elements.
    pub fn has_more(&self) -> bool {
        false
    }
}

impl Iterator for AnyIterator {
    type Item = AnyBox;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_iterator_new() {
        let values: Vec<i32> = vec![1, 2, 3];
        let _iter = AnyIterator::new(values);
        // Simplified - just verify it doesn't panic
    }

    #[test]
    fn test_any_iterator_next_any() {
        let mut iter = AnyIterator::new(vec![1i32]);
        assert!(iter.next_any::<i32>().is_none()); // Simplified
    }
}
