//! ArrayView - A view into a contiguous sequence with a known length

/// A view into a contiguous sequence with a known length.
///
/// Similar to a slice but with explicit length tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayView<'a, T> {
    data: &'a [T],
}

impl<'a, T> ArrayView<'a, T> {
    /// Creates a new array view from a slice.
    #[inline]
    pub const fn new(data: &'a [T]) -> Self {
        Self { data }
    }

    /// Returns the length of the view.
    #[inline]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the view is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the underlying slice.
    #[inline]
    pub const fn as_slice(&self) -> &'a [T] {
        self.data
    }

    /// Gets an element at the given index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&'a T> {
        self.data.get(index)
    }

    /// Returns an iterator over the elements.
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'a, T> {
        self.data.iter()
    }

    /// Returns the first element.
    #[inline]
    pub fn first(&self) -> Option<&'a T> {
        self.data.first()
    }

    /// Returns the last element.
    #[inline]
    pub fn last(&self) -> Option<&'a T> {
        self.data.last()
    }

    /// Splits the view at the given index.
    #[inline]
    pub fn split_at(&self, mid: usize) -> (ArrayView<'a, T>, ArrayView<'a, T>) {
        let (left, right) = self.data.split_at(mid);
        (ArrayView::new(left), ArrayView::new(right))
    }
}

impl<'a, T> From<&'a [T]> for ArrayView<'a, T> {
    #[inline]
    fn from(slice: &'a [T]) -> Self {
        Self::new(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_view() {
        let data = [1, 2, 3, 4, 5];
        let view = ArrayView::new(&data);

        assert_eq!(view.len(), 5);
        assert_eq!(view.get(2), Some(&3));
        assert_eq!(view.first(), Some(&1));
        assert_eq!(view.last(), Some(&5));

        let (left, right) = view.split_at(2);
        assert_eq!(left.as_slice(), &[1, 2]);
        assert_eq!(right.as_slice(), &[3, 4, 5]);
    }

    #[test]
    fn test_array_view_from_slice() {
        let data = vec![1, 2, 3, 4, 5];
        let view: ArrayView<i32> = data.as_slice().into();
        assert_eq!(view.len(), 5);
    }
}
