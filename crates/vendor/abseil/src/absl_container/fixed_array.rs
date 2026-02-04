//! FixedArray - Fixed-size array with runtime bounds checking

use alloc::vec::Vec;

/// A fixed-size array with runtime bounds checking.
///
/// Similar to `std::array` but with size determined at runtime.
#[derive(Clone, Debug)]
pub struct FixedArray<T> {
    data: Vec<T>,
    capacity: usize,
}

impl<T> FixedArray<T> {
    /// Creates a new fixed array with the given capacity.
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_container::FixedArray;
    ///
    /// let arr: FixedArray<i32> = FixedArray::new(5);
    /// assert_eq!(arr.capacity(), 5);
    /// assert!(arr.is_empty());
    /// ```
    #[inline]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than 0");
        FixedArray {
            data: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Returns the capacity of the array.
    #[inline]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of elements in the array.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the array is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns true if the array is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.data.len() == self.capacity
    }

    /// Pushes a value into the array.
    ///
    /// Returns `Ok(())` if successful, `Err(value)` if the array is full.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_container::FixedArray;
    ///
    /// let mut arr = FixedArray::new(3);
    /// assert!(arr.push(1).is_ok());
    /// assert!(arr.push(2).is_ok());
    /// assert!(arr.push(3).is_ok());
    /// assert!(arr.push(4).is_err());
    /// ```
    #[inline]
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            Err(value)
        } else {
            self.data.push(value);
            Ok(())
        }
    }

    /// Pops a value from the array.
    ///
    /// Returns `Some(value)` if successful, `None` if empty.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    /// Clears the array.
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Gets a reference to the element at the given index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    /// Gets a mutable reference to the element at the given index.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    /// Returns an iterator over the elements.
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Returns a mutable iterator over the elements.
    #[inline]
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }
}

impl<T: core::fmt::Display> core::fmt::Display for FixedArray<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[")?;
        for (i, item) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_array() {
        let mut arr: FixedArray<i32> = FixedArray::new(3);
        assert_eq!(arr.capacity(), 3);
        assert!(arr.is_empty());
        assert!(!arr.is_full());

        assert!(arr.push(1).is_ok());
        assert!(arr.push(2).is_ok());
        assert!(arr.push(3).is_ok());
        assert!(arr.is_full());
        assert!(arr.push(4).is_err());

        assert_eq!(arr.pop(), Some(3));
        assert_eq!(arr.len(), 2);
        assert!(!arr.is_full());

        assert_eq!(arr.get(0), Some(&1));
        assert_eq!(arr.get(5), None);
    }
}
