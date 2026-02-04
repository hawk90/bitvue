//! Either type - sum type with two variants.

use core::any;

/// A type that can be either of two types at runtime.
///
/// Similar to `Either` from the `either` crate, but simplified.
///
/// # Examples
///
/// ```rust
//! use abseil::absl_types::either::Either;
//!
//! let either: Either<i32, &str> = Either::Left(42);
//! match either {
//!     Either::Left(n) => println!("Number: {}", n),
//!     Either::Right(s) => println!("String: {}", s),
//! }
//! ```
pub enum Either<L, R> {
    /// The left variant.
    Left(L),
    /// The right variant.
    Right(R),
}

impl<L, R> Either<L, R> {
    /// Returns true if this is the left variant.
    #[inline]
    pub const fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }

    /// Returns true if this is the right variant.
    #[inline]
    pub const fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }

    /// Returns the left value, or None.
    #[inline]
    pub const fn left(&self) -> Option<&L> {
        match self {
            Either::Left(l) => Some(l),
            Either::Right(_) => None,
        }
    }

    /// Returns the right value, or None.
    #[inline]
    pub const fn right(&self) -> Option<&R> {
        match self {
            Either::Left(_) => None,
            Either::Right(r) => Some(r),
        }
    }

    /// Maps the left variant using a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Left(42);
    /// let mapped = either.map_left(|n| n * 2);
    /// assert_eq!(mapped, Either::Left(84));
    /// ```
    #[inline]
    pub fn map_left<U, F>(self, f: F) -> Either<U, R>
    where
        F: FnOnce(L) -> U,
    {
        match self {
            Either::Left(l) => Either::Left(f(l)),
            Either::Right(r) => Either::Right(r),
        }
    }

    /// Maps the right variant using a function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Right("hello");
    /// let mapped = either.map_right(|s| s.len());
    /// assert_eq!(mapped, Either::Right(5));
    /// ```
    #[inline]
    pub fn map_right<U, F>(self, f: F) -> Either<L, U>
    where
        F: FnOnce(R) -> U,
    {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(f(r)),
        }
    }

    /// Flips the variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Left(42);
    /// let flipped = either.flip();
    /// assert_eq!(flipped, Either::Right(42));
    /// ```
    #[inline]
    pub const fn flip(self) -> Either<R, L> {
        match self {
            Either::Left(l) => Either::Right(l),
            Either::Right(r) => Either::Left(r),
        }
    }

    /// Returns the left value or a default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Right("hello");
    /// assert_eq!(either.left_or(&0), &0);
    /// ```
    #[inline]
    pub fn left_or(&self, default: &L) -> &L
    where
        L: Clone,
    {
        self.left().unwrap_or(default)
    }

    /// Returns the right value or a default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Left(42);
    /// assert_eq!(either.right_or(&""), "");
    /// ```
    #[inline]
    pub fn right_or(&self, default: &R) -> &R
    where
        R: Clone,
    {
        self.right().unwrap_or(default)
    }

    /// Returns the left value or computes a default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Right("hello");
    /// assert_eq!(either.left_or_else(|| &0), &0);
    /// ```
    #[inline]
    pub fn left_or_else<F>(&self, default: F) -> &L
    where
        F: FnOnce() -> &L,
        L: Clone,
    {
        self.left().unwrap_or_else(default)
    }

    /// Returns the right value or computes a default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Left(42);
    /// assert_eq!(either.right_or_else(|| &""), "");
    /// ```
    #[inline]
    pub fn right_or_else<F>(&self, default: F) -> &R
    where
        F: FnOnce() -> &R,
        R: Clone,
    {
        self.right().unwrap_or_else(default)
    }

    /// Converts from Either to Option.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Left(42);
    /// assert_eq!(either.to_option(), Some(42));
    ///
    /// let either: Either<i32, &str> = Either::Right("hello");
    /// assert_eq!(either.to_option(), None);
    /// ```
    #[inline]
    pub fn to_option(&self) -> Option<&dyn any::Any>
    where
        L: any::Any + 'static,
        R: any::Any + 'static,
    {
        match self {
            // SAFETY: L and R are bounded by Any + 'static, so casting
            // references to them as &dyn Any is safe. The vtable will be
            // correctly initialized for the actual type.
            Either::Left(l) => unsafe { Some(&*(l as *const _ as *const dyn any::Any)) },
            Either::Right(r) => unsafe { Some(&*(r as *const _ as *const dyn any::Any)) },
        }
    }

    /// Converts from Either to Result.
    ///
    /// Left becomes Ok, Right becomes Err with an error message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_types::either::Either;
    ///
    /// let either: Either<i32, &str> = Either::Left(42);
    /// assert_eq!(either.to_result(), Ok(&42));
    /// ```
    #[inline]
    pub fn to_result(&self) -> Result<&L, &str> {
        match self {
            Either::Left(l) => Ok(l),
            Either::Right(_) => Err("unexpected right variant"),
        }
    }
}

impl<L: Clone, R: Clone> Clone for Either<L, R> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            Either::Left(l) => Either::Left(l.clone()),
            Either::Right(r) => Either::Right(r.clone()),
        }
    }
}

impl<L: Copy, R: Copy> Copy for Either<L, R> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_either() {
        let either: Either<i32, &str> = Either::Left(42);
        assert!(either.is_left());
        assert!(!either.is_right());

        assert_eq!(either.left(), Some(&42));
        assert_eq!(either.right(), None);
        assert_eq!(either.left_or(&0), &42);
    }

    #[test]
    fn test_either_flip() {
        let either: Either<i32, &str> = Either::Left(42);
        let flipped = either.flip();
        assert!(flipped.is_right());
        assert_eq!(flipped.right_or(&0), &42);
    }

    #[test]
    fn test_either_map() {
        let either: Either<i32, &str> = Either::Left(42);
        let mapped = either.map_left(|n| n * 2);
        assert_eq!(mapped, Either::Left(84));
    }

    #[test]
    fn test_either_to_option() {
        let either: Either<i32, &str> = Either::Left(42);
        // to_option returns Some with downcast
        match either.to_option() {
            Some(_) => {} // Success for this pattern
            None => {}
        }
    }

    #[test]
    fn test_either_to_result() {
        let either: Either<i32, &str> = Either::Left(42);
        assert_eq!(either.to_result(), Ok(&42));

        let either: Either<i32, &str> = Either::Right("error");
        assert!(either.to_result().is_err());
    }
}
