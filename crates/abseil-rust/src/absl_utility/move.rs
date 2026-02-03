//! Move utilities.

/// Utility for moving values out of references in certain contexts.
///
/// This is a placeholder for Abseil's `move_on_copy` utility.
/// In Rust, this pattern is typically handled differently.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::move_::MoveOnCopy;
///
/// let value = MoveOnCopy::new(42);
/// let copied = value;  // Copies the value
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MoveOnCopy<T: Copy>(pub T);

impl<T: Copy> MoveOnCopy<T> {
    /// Creates a new MoveOnCopy wrapper.
    #[inline]
    pub const fn new(value: T) -> Self {
        MoveOnCopy(value)
    }

    /// Extracts the inner value.
    #[inline]
    pub const fn into_inner(self) -> T {
        self.0
    }
}

/// Always returns false.
///
/// This is useful as a placeholder or for generic code.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::move_::always_false;
///
/// assert!(!always_false::<i32>());
/// assert!(always_false::<String>());
/// ```
#[inline]
pub const fn always_false<T: ?Sized>() -> bool {
    false
}

/// Always returns true.
///
/// This is useful as a placeholder or for generic code.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::move_::always_true;
///
/// assert!(always_true());
/// ```
#[inline]
pub const fn always_true() -> bool {
    true
}
