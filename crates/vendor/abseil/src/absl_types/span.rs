//! Span type for contiguous sequences.
//!
//! Provides a span/view type for contiguous sequences similar to C++20's std::span.
//! This extends Rust's built-in slice with additional utility methods.

use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Deref, DerefMut};

/// A span over a contiguous sequence of values.
///
/// Similar to `&[T]` but with additional utility methods and a more ergonomic API.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::span::Span;
///
/// let data = [1, 2, 3, 4, 5];
/// let span = Span::from(&data[..]);
/// assert_eq!(span.len(), 5);
/// assert_eq!(span[0], 1);
/// assert_eq!(span.first(), Some(&1));
/// assert_eq!(span.last(), Some(&5));
/// ```
#[derive(Copy, Clone)]
pub struct Span<'a, T> {
    data: &'a [T],
}

impl<'a, T> Span<'a, T> {
    /// Creates a new span from a slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::span::Span;
    ///
    /// let data = [1, 2, 3];
    /// let span = Span::new(&data[..]);
    /// ```
    #[inline]
    pub fn new(data: &'a [T]) -> Self {
        Span { data }
    }

    /// Returns the number of elements in the span.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a pointer to the first element.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Returns the underlying slice.
    #[inline]
    pub fn as_slice(&self) -> &'a [T] {
        self.data
    }

    /// Returns a reference to the first element, or `None` if empty.
    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.data.first()
    }

    /// Returns a reference to the last element, or `None` if empty.
    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.data.last()
    }

    /// Returns a subslice spanning the given range.
    #[inline]
    pub fn slice(&self, range: core::ops::Range<usize>) -> Option<Span<'a, T>> {
        self.data.get(range.clone()).map(|data| Span { data })
    }

    /// Returns the first `n` elements of the span.
    #[inline]
    pub fn take_first(&self, n: usize) -> Option<Span<'a, T>> {
        self.data.get(..n).map(|data| Span { data })
    }

    /// Returns the last `n` elements of the span.
    #[inline]
    pub fn take_last(&self, n: usize) -> Option<Span<'a, T>> {
        let len = self.data.len();
        if n > len {
            None
        } else {
            Some(Span {
                data: &self.data[len - n..],
            })
        }
    }
}

impl<'a, T> Deref for Span<'a, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.data
    }
}

impl<'a, T> From<&'a [T]> for Span<'a, T> {
    #[inline]
    fn from(data: &'a [T]) -> Self {
        Span { data }
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for Span<'a, T> {
    #[inline]
    fn from(data: &'a [T; N]) -> Self {
        Span { data: &data[..] }
    }
}

impl<'a, T: PartialEq> PartialEq for Span<'a, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'a, T: Eq> Eq for Span<'a, T> {}

impl<'a, T: PartialOrd> PartialOrd for Span<'a, T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<'a, T: Ord> Ord for Span<'a, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.data.cmp(&other.data)
    }
}

impl<'a, T: Hash> Hash for Span<'a, T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for Span<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

/// A mutable span over a contiguous sequence of values.
///
/// Similar to `&mut [T]` but with additional utility methods.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::span::SpanMut;
///
/// let mut data = [1, 2, 3, 4, 5];
/// let mut span = SpanMut::from(&mut data[..]);
/// span[0] = 10;
/// assert_eq!(data[0], 10);
/// ```
pub struct SpanMut<'a, T> {
    data: &'a mut [T],
}

impl<'a, T> SpanMut<'a, T> {
    /// Creates a new mutable span from a mutable slice.
    #[inline]
    pub fn new(data: &'a mut [T]) -> Self {
        SpanMut { data }
    }

    /// Returns the number of elements in the span.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a pointer to the first element.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.data.as_ptr()
    }

    /// Returns a mutable pointer to the first element.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr()
    }

    /// Returns the underlying slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.data
    }

    /// Returns the underlying mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.data
    }

    /// Returns a reference to the first element, or `None` if empty.
    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.data.first()
    }

    /// Returns a mutable reference to the first element, or `None` if empty.
    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.data.first_mut()
    }

    /// Returns a reference to the last element, or `None` if empty.
    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.data.last()
    }

    /// Returns a mutable reference to the last element, or `None` if empty.
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.data.last_mut()
    }

    /// Returns a subslice spanning the given range.
    #[inline]
    pub fn slice(&self, range: core::ops::Range<usize>) -> Option<Span<'_, T>> {
        self.data.get(range.clone()).map(|data| Span { data })
    }

    /// Returns a mutable subslice spanning the given range.
    #[inline]
    pub fn slice_mut(&mut self, range: core::ops::Range<usize>) -> Option<SpanMut<'_, T>> {
        self.data.get_mut(range.clone()).map(|data| SpanMut { data })
    }

    /// Returns an immutable span over this data.
    #[inline]
    pub fn as_span(&self) -> Span<'_, T> {
        Span {
            data: self.data,
        }
    }

    /// Fills the span with the given value.
    #[inline]
    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        self.data.fill(value);
    }

    /// Copies elements from `src` into this span.
    ///
    /// Returns `true` if the copy was successful.
    #[inline]
    pub fn copy_from_slice(&mut self, src: &[T]) -> bool
    where
        T: Clone,
    {
        if self.len() != src.len() {
            return false;
        }
        self.data.clone_from_slice(src);
        true
    }
}

impl<'a, T> Deref for SpanMut<'a, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.data
    }
}

impl<'a, T> DerefMut for SpanMut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.data
    }
}

impl<'a, T> From<&'a mut [T]> for SpanMut<'a, T> {
    #[inline]
    fn from(data: &'a mut [T]) -> Self {
        SpanMut { data }
    }
}

impl<'a, T, const N: usize> From<&'a mut [T; N]> for SpanMut<'a, T> {
    #[inline]
    fn from(data: &'a mut [T; N]) -> Self {
        SpanMut { data: &mut data[..] }
    }
}

impl<'a, T: PartialEq> PartialEq for SpanMut<'a, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<'a, T: Eq> Eq for SpanMut<'a, T> {}

impl<'a, T: fmt::Debug> fmt::Debug for SpanMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_new() {
        let data = [1, 2, 3];
        let span = Span::new(&data[..]);
        assert_eq!(span.len(), 3);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_span_empty() {
        let data: [i32; 0] = [];
        let span = Span::new(&data[..]);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }

    #[test]
    fn test_span_first_last() {
        let data = [1, 2, 3, 4, 5];
        let span = Span::from(&data[..]);
        assert_eq!(span.first(), Some(&1));
        assert_eq!(span.last(), Some(&5));
    }

    #[test]
    fn test_span_take_first_last() {
        let data = [1, 2, 3, 4, 5];
        let span = Span::from(&data[..]);

        let first_two = span.take_first(2).unwrap();
        assert_eq!(first_two.as_slice(), &[1, 2]);

        let last_two = span.take_last(2).unwrap();
        assert_eq!(last_two.as_slice(), &[4, 5]);
    }

    #[test]
    fn test_span_index() {
        let data = [1, 2, 3, 4, 5];
        let span = Span::from(&data[..]);
        assert_eq!(span[0], 1);
        assert_eq!(span[4], 5);
    }

    #[test]
    fn test_span_slice() {
        let data = [1, 2, 3, 4, 5];
        let span = Span::from(&data[..]);
        let middle = span.slice(1..4).unwrap();
        assert_eq!(middle.as_slice(), &[2, 3, 4]);
    }

    #[test]
    fn test_span_equality() {
        let data = [1, 2, 3];
        let span1 = Span::from(&data[..]);
        let span2 = Span::from(&data[..]);
        assert_eq!(span1, span2);
    }

    #[test]
    fn test_span_mut_fill() {
        let mut data = [1, 2, 3, 4, 5];
        let mut span = SpanMut::from(&mut data[..]);
        span.fill(10);
        assert_eq!(data, [10, 10, 10, 10, 10]);
    }

    #[test]
    fn test_span_mut_copy_from_slice() {
        let mut dest = [0, 0, 0];
        let src = [1, 2, 3];
        let mut span = SpanMut::from(&mut dest[..]);
        assert!(span.copy_from_slice(&src[..]));
        assert_eq!(dest, [1, 2, 3]);
    }

    #[test]
    fn test_span_mut_index() {
        let mut data = [1, 2, 3];
        let mut span = SpanMut::from(&mut data[..]);
        span[0] = 10;
        assert_eq!(data, [10, 2, 3]);
    }

    #[test]
    fn test_span_from_array() {
        const DATA: [i32; 3] = [1, 2, 3];
        let span = Span::from(&DATA);
        assert_eq!(span.len(), 3);
    }

    #[test]
    fn test_span_first_last_empty() {
        let data: [i32; 0] = [];
        let span = Span::from(&data[..]);
        assert_eq!(span.first(), None);
        assert_eq!(span.last(), None);
    }
}
