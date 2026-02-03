//! Utility traits.

/// Trait for types that can be converted to a slice of bytes.
///
/// This is similar to Abseil's `Span` concept.
pub trait AsBytes {
    /// Returns a byte slice representing the type.
    fn as_bytes(&self) -> &[u8];
}

/// Trait for types that can be constructed from a byte slice.
pub trait FromBytes: Sized {
    /// Constructs a value from a byte slice.
    ///
    /// Returns `Err` if the bytes are not valid for this type.
    fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str>;
}

impl AsBytes for [u8] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl AsBytes for str {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }
}

impl<T: AsBytes> AsBytes for &[T] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }
}

impl<T: AsBytes> AsBytes for &mut [T] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }
}

impl FromBytes for () {
    #[inline]
    fn from_bytes(_bytes: &[u8]) -> Result<Self, &'static str> {
        Ok(())
    }
}

/// Trait for types that have a defined bit pattern representation.
///
/// This is useful for serialization and low-level manipulation.
pub trait BitPattern: Copy {
    /// The byte representation of this type.
    type Bytes: AsRef<[u8]>;

    /// Converts the type to its byte representation.
    fn to_bytes(&self) -> Self::Bytes;

    /// Creates a value from its byte representation.
    fn from_bytes(bytes: Self::Bytes) -> Self;
}

impl BitPattern for () {
    type Bytes = [u8; 0];

    fn to_bytes(&self) -> Self::Bytes {
        []
    }

    fn from_bytes(_bytes: Self::Bytes) -> Self {}
}

impl BitPattern for u8 {
    type Bytes = [u8; 1];

    fn to_bytes(&self) -> Self::Bytes {
        [*self]
    }

    fn from_bytes(bytes: Self::Bytes) -> Self {
        bytes[0]
    }
}

impl BitPattern for i8 {
    type Bytes = [u8; 1];

    fn to_bytes(&self) -> Self::Bytes {
        [*self as u8]
    }

    fn from_bytes(bytes: Self::Bytes) -> Self {
        bytes[0] as i8
    }
}

impl BitPattern for bool {
    type Bytes = [u8; 1];

    fn to_bytes(&self) -> Self::Bytes {
        [*self as u8]
    }

    fn from_bytes(bytes: Self::Bytes) -> Self {
        bytes[0] != 0
    }
}

/// Trait for callable objects.
///
/// This trait abstracts over functions, closures, and other callable types.
pub trait Callable<R> {
    /// Calls the callable with no arguments.
    fn call(&self) -> R;
}

impl<F, R> Callable<R> for F
where
    F: Fn() -> R,
{
    #[inline]
    fn call(&self) -> R {
        (self)()
    }
}

/// Trait for callable objects with one argument.
pub trait Callable1<A, R> {
    /// Calls the callable with one argument.
    fn call(&self, arg: A) -> R;
}

impl<F, A, R> Callable1<A, R> for F
where
    F: Fn(A) -> R,
{
    #[inline]
    fn call(&self, arg: A) -> R {
        (self)(arg)
    }
}

/// Trait for callable objects with two arguments.
pub trait Callable2<A, B, R> {
    /// Calls the callable with two arguments.
    fn call(&self, a: A, b: B) -> R;
}

impl<F, A, B, R> Callable2<A, B, R> for F
where
    F: Fn(A, B) -> R,
{
    #[inline]
    fn call(&self, a: A, b: B) -> R {
        (self)(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callable_trait() {
        let f = || 42;
        assert_eq!(f.call(), 42);

        let add_one = |x: i32| x + 1;
        assert_eq!(Callable1::call(&add_one, 5), 6);

        let add = |a: i32, b: i32| a + b;
        assert_eq!(Callable2::call(&add, 3, 4), 7);
    }
}
