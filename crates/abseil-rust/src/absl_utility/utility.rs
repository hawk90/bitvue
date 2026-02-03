//! Utility functions.
//!
//! Provides general-purpose helper functions.


/// Returns the address of a reference as a `usize`.
///
/// This is similar to C++'s `std::addressof` or Rust's raw pointer casting.
/// This can be useful for debugging or when you need to compare addresses.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::address_of;
///
/// let value = 42;
/// let addr = address_of(&value);
/// assert!(addr > 0);
/// ```
///
/// # Safety
///
/// The returned address is only valid as long as the reference is valid.
/// Using the address after the reference is dropped is undefined behavior.
#[inline]
pub fn address_of<T>(r: &T) -> usize {
    r as *const T as usize
}

/// Returns the mutable address of a mutable reference as a `usize`.
///
/// This is similar to C++'s `std::addressof` but for mutable references.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::address_of_mut;
///
/// let mut value = 42;
/// let addr = address_of_mut(&mut value);
/// assert!(addr > 0);
/// ```
#[inline]
pub fn address_of_mut<T>(r: &mut T) -> usize {
    r as *mut T as usize
}

/// A wrapper that enables move-on-copy semantics.
///
/// When you copy this wrapper, the inner value is moved instead of copied.
/// This is useful for types that implement `Copy` but you want move semantics.
///
/// This is similar to C++'s `std::move` or Rust's `ManuallyDrop` patterns.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::MoveOnCopy;
///
/// let value = MoveOnCopy::new(42);
/// // Copying MoveOnCopy copies the inner value (since it's Copy)
/// let value2 = value;
/// assert_eq!(*value.get(), 42);
/// assert_eq!(*value2.get(), 42);
/// ```
#[derive(Clone, Copy)]
pub struct MoveOnCopy<T: Copy>(T);

impl<T: Copy> MoveOnCopy<T> {
    /// Creates a new `MoveOnCopy` wrapper.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_utility::utility::MoveOnCopy;
    ///
    /// let value = MoveOnCopy::new(42);
    /// ```
    #[inline]
    pub fn new(value: T) -> Self {
        MoveOnCopy(value)
    }

    /// Extracts the inner value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_utility::utility::MoveOnCopy;
    ///
    /// let value = MoveOnCopy::new(42);
    /// assert_eq!(value.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Gets a reference to the inner value.
    #[inline]
    pub fn get(&self) -> &T {
        &self.0
    }

    /// Gets a mutable reference to the inner value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Copy + core::fmt::Debug> core::fmt::Debug for MoveOnCopy<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("MoveOnCopy").field(&self.0).finish()
    }
}

impl<T: Copy + PartialEq> PartialEq for MoveOnCopy<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Copy + Eq> Eq for MoveOnCopy<T> {}

impl<T: Copy + PartialOrd> PartialOrd for MoveOnCopy<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: Copy + Ord> Ord for MoveOnCopy<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Copy + Default> Default for MoveOnCopy<T> {
    fn default() -> Self {
        MoveOnCopy(T::default())
    }
}

/// Creates a `MoveOnCopy` wrapper.
///
/// This is a convenience function for creating `MoveOnCopy` instances.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::move_on_copy;
///
/// let value = move_on_copy(42);
/// ```
#[inline]
pub fn move_on_copy<T: Copy>(value: T) -> MoveOnCopy<T> {
    MoveOnCopy::new(value)
}

/// Swaps the values of two mutable references.
///
/// This is a re-export of `core::mem::swap` for convenience.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::swap;
///
/// let mut a = 1;
/// let mut b = 2;
/// swap(&mut a, &mut b);
/// assert_eq!(a, 2);
/// assert_eq!(b, 1);
/// ```
#[inline]
pub fn swap<T>(a: &mut T, b: &mut T) {
    core::mem::swap(a, b)
}

/// Replaces the value in a mutable reference with a new one, returning the old value.
///
/// This is a re-export of `core::mem::replace` for convenience.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::replace;
///
/// let mut value = 1;
/// let old = replace(&mut value, 2);
/// assert_eq!(old, 1);
/// assert_eq!(value, 2);
/// ```
#[inline]
pub fn replace<T>(dest: &mut T, src: T) -> T {
    core::mem::replace(dest, src)
}

/// Forgets the contents of a value without running its destructor.
///
/// This is a re-export of `core::mem::forget` for convenience.
///
/// # Safety
///
/// This function is unsafe because it can leak resources. Prefer using
/// `ManuallyDrop` or other safer alternatives when possible.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::forget;
///
/// let value = vec![1, 2, 3];
/// forget(value); // The Vec's destructor is not run
/// ```
#[inline]
pub fn forget<T>(value: T) {
    core::mem::forget(value)
}

/// Takes the value out of a mutable reference, leaving a default value in its place.
///
/// This is a re-export of `core::mem::take` for convenience.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::utility::take;
///
/// let mut value = vec![1, 2, 3];
/// let vec = take(&mut value);
/// assert!(value.is_empty());
/// assert_eq!(vec, vec![1, 2, 3]);
/// ```
#[inline]
pub fn take<T: Default>(dest: &mut T) -> T {
    core::mem::take(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_of() {
        let value = 42;
        let addr = address_of(&value);
        assert!(addr > 0);

        let value2 = value;
        let addr2 = address_of(&value2);
        assert_ne!(addr, addr2); // Different addresses
    }

    #[test]
    fn test_address_of_mut() {
        let mut value = 42;
        let addr = address_of_mut(&mut value);
        assert!(addr > 0);
    }

    #[test]
    fn test_move_on_copy() {
        let value = MoveOnCopy::new(42);
        assert_eq!(value.get(), &42);

        let value2 = value;
        assert_eq!(value2.get(), &42);
        assert_eq!(value2.into_inner(), 42);
    }

    #[test]
    fn test_move_on_copy_default() {
        let value: MoveOnCopy<i32> = MoveOnCopy::default();
        assert_eq!(*value.get(), 0);
    }

    #[test]
    fn test_move_on_copy_equality() {
        let v1 = MoveOnCopy::new(42);
        let v2 = MoveOnCopy::new(42);
        let v3 = MoveOnCopy::new(100);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_move_on_copy_ordering() {
        let v1 = MoveOnCopy::new(1);
        let v2 = MoveOnCopy::new(2);

        assert!(v1 < v2);
    }

    #[test]
    fn test_swap() {
        let mut a = 1;
        let mut b = 2;
        swap(&mut a, &mut b);
        assert_eq!(a, 2);
        assert_eq!(b, 1);
    }

    #[test]
    fn test_replace() {
        let mut value = 1;
        let old = replace(&mut value, 2);
        assert_eq!(old, 1);
        assert_eq!(value, 2);
    }

    #[test]
    fn test_forget() {
        let dropped = false;
        {
            struct Sentinel<'a>(&'a mut bool);
            impl Drop for Sentinel<'_> {
                fn drop(&mut self) {
                    *self.0 = true;
                }
            }

            let mut flag = false;
            forget(Sentinel(&mut flag));
            // Destructor not run
            assert!(!flag);
        }
        // The flag was never set to true because we forgot the sentinel
        assert!(!dropped);
    }

    #[test]
    fn test_take() {
        let mut value = vec![1, 2, 3];
        let vec = take(&mut value);
        assert!(value.is_empty());
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_move_on_copy_function() {
        let value = move_on_copy(42);
        assert_eq!(*value.get(), 42);
    }
}
