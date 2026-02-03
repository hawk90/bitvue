//! Miscellaneous utility types.

use core::fmt;

/// A type-level alignment marker.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::Aligned;
///
/// #[repr(C)]
/// struct AlignedStruct {
///     _align: Aligned<16>,
///     value: u32,
/// }
/// ```
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Aligned<const N: usize> {
    _align: [u8; N],
}

impl<const N: usize> Aligned<N> {
    /// Creates a new aligned marker.
    pub const fn new() -> Self {
        Self { _align: [0; N] }
    }
}

/// Pads a type to a specific size.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::Padded;
///
/// type PaddedU32 = Padded<u32, 8>;
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Padded<T, const N: usize> {
    pub value: T,
    pub padding: [u8; N],
}

impl<T: Default, const N: usize> Default for Padded<T, N> {
    fn default() -> Self {
        Self {
            value: T::default(),
            padding: [0; N],
        }
    }
}

/// A union that provides type-safe punning.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::Pun;
///
/// let pun = Pun::<u32, [u8; 4]>::new(0x12345678);
/// assert_eq!(pun.into_right(), [0x78, 0x56, 0x34, 0x12]);
/// ```
#[derive(Copy, Clone)]
#[repr(C)]
pub union Pun<L, R> {
    pub left: L,
    pub right: R,
}

impl<L, R> Pun<L, R> {
    /// Creates a new pun from the left type.
    #[inline]
    pub const fn new(left: L) -> Self {
        Self { left }
    }

    /// Creates a new pun from the right type.
    #[inline]
    pub const fn from_right(right: R) -> Self {
        Self { right }
    }

    /// Gets the left value.
    #[inline]
    pub const fn as_left(&self) -> &L {
        unsafe { &self.left }
    }

    /// Gets the right value.
    #[inline]
    pub const fn as_right(&self) -> &R {
        unsafe { &self.right }
    }

    /// Returns the left value.
    #[inline]
    pub const fn into_left(self) -> L {
        self.left
    }

    /// Returns the right value.
    #[inline]
    pub const fn into_right(self) -> R {
        self.right
    }
}

/// A tagged union type.
///
/// This provides a type-safe alternative to raw unions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::Tagged;
///
/// let tagged: Tagged<i32, &str> = Tagged::A(42);
/// match tagged {
///     Tagged::A(n) => println!("Number: {}", n),
///     Tagged::B(s) => println!("String: {}", s),
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tagged<A, B> {
    /// Variant A.
    A(A),
    /// Variant B.
    B(B),
}

impl<A, B> Tagged<A, B> {
    /// Returns true if this is the A variant.
    #[inline]
    pub const fn is_a(&self) -> bool {
        matches!(self, Tagged::A(_))
    }

    /// Returns true if this is the B variant.
    #[inline]
    pub const fn is_b(&self) -> bool {
        matches!(self, Tagged::B(_))
    }

    /// Returns the A value, or None.
    #[inline]
    pub const fn a(&self) -> Option<&A> {
        match self {
            Tagged::A(v) => Some(v),
            Tagged::B(_) => None,
        }
    }

    /// Returns the B value, or None.
    #[inline]
    pub const fn b(&self) -> Option<&B> {
        match self {
            Tagged::A(_) => None,
            Tagged::B(v) => Some(v),
        }
    }

    /// Maps the A variant using a function.
    #[inline]
    pub fn map_a<C, F>(self, f: F) -> Tagged<C, B>
    where
        F: FnOnce(A) -> C,
    {
        match self {
            Tagged::A(v) => Tagged::A(f(v)),
            Tagged::B(v) => Tagged::B(v),
        }
    }

    /// Maps the B variant using a function.
    #[inline]
    pub fn map_b<C, F>(self, f: F) -> Tagged<A, C>
    where
        F: FnOnce(B) -> C,
    {
        match self {
            Tagged::A(v) => Tagged::A(v),
            Tagged::B(v) => Tagged::B(f(v)),
        }
    }
}

/// A tuple that provides named access to its elements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::Pair;
///
/// let pair = Pair::new(1, "hello");
/// assert_eq!(pair.get_first(), &1);
/// assert_eq!(pair.get_second(), &"hello");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pair<A, B> {
    pub first: A,
    pub second: B,
}

impl<A, B> Pair<A, B> {
    /// Creates a new pair.
    #[inline]
    pub const fn new(first: A, second: B) -> Self {
        Self { first, second }
    }

    /// Gets the first element.
    #[inline]
    pub const fn get_first(&self) -> &A {
        &self.first
    }

    /// Gets the second element.
    #[inline]
    pub const fn get_second(&self) -> &B {
        &self.second
    }

    /// Creates a new pair with swapped elements.
    #[inline]
    pub const fn swap(self) -> Pair<B, A> {
        Pair {
            first: self.second,
            second: self.first,
        }
    }
}

impl<A, B> From<(A, B)> for Pair<A, B> {
    #[inline]
    fn from(pair: (A, B)) -> Self {
        Self {
            first: pair.0,
            second: pair.1,
        }
    }
}

impl<A, B> From<Pair<A, B>> for (A, B) {
    #[inline]
    fn from(pair: Pair<A, B>) -> Self {
        (pair.first, pair.second)
    }
}

/// A triple that provides named access to its elements.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::Triple;
///
/// let triple = Triple::new(1, "hello", 3.14);
/// assert_eq!(triple.get_first(), &1);
/// assert_eq!(triple.get_second(), &"hello");
/// assert_eq!(triple.get_third(), &3.14);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Triple<A, B, C> {
    pub first: A,
    pub second: B,
    pub third: C,
}

impl<A, B, C> Triple<A, B, C> {
    /// Creates a new triple.
    #[inline]
    pub const fn new(first: A, second: B, third: C) -> Self {
        Self {
            first,
            second,
            third,
        }
    }

    /// Gets the first element.
    #[inline]
    pub const fn get_first(&self) -> &A {
        &self.first
    }

    /// Gets the second element.
    #[inline]
    pub const fn get_second(&self) -> &B {
        &self.second
    }

    /// Gets the third element.
    #[inline]
    pub const fn get_third(&self) -> &C {
        &self.third
    }
}

impl<A, B, C> From<(A, B, C)> for Triple<A, B, C> {
    #[inline]
    fn from(triple: (A, B, C)) -> Self {
        Self {
            first: triple.0,
            second: triple.1,
            third: triple.2,
        }
    }
}

/// A sum type that can be one of three variants.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::OneOf3;
///
/// let result: OneOf3<i32, &str, bool> = OneOf3::T2("hello");
/// match result {
///     OneOf3::T1(n) => println!("Number: {}", n),
///     OneOf3::T2(s) => println!("String: {}", s),
///     OneOf3::T3(b) => println!("Bool: {}", b),
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneOf3<A, B, C> {
    /// First variant.
    T1(A),
    /// Second variant.
    T2(B),
    /// Third variant.
    T3(C),
}

impl<A, B, C> OneOf3<A, B, C> {
    /// Returns the index of the active variant (0, 1, or 2).
    #[inline]
    pub const fn index(&self) -> usize {
        match self {
            OneOf3::T1(_) => 0,
            OneOf3::T2(_) => 1,
            OneOf3::T3(_) => 2,
        }
    }

    /// Returns the T1 value, or None.
    #[inline]
    pub const fn t1(&self) -> Option<&A> {
        match self {
            OneOf3::T1(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the T2 value, or None.
    #[inline]
    pub const fn t2(&self) -> Option<&B> {
        match self {
            OneOf3::T2(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the T3 value, or None.
    #[inline]
    pub const fn t3(&self) -> Option<&C> {
        match self {
            OneOf3::T3(v) => Some(v),
            _ => None,
        }
    }
}

/// A sum type that can be one of four variants.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::misc_types::OneOf4;
///
/// let result: OneOf4<i32, &str, bool, f64> = OneOf4::T3(true);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneOf4<A, B, C, D> {
    /// First variant.
    T1(A),
    /// Second variant.
    T2(B),
    /// Third variant.
    T3(C),
    /// Fourth variant.
    T4(D),
}

impl<A, B, C, D> OneOf4<A, B, C, D> {
    /// Returns the index of the active variant (0, 1, 2, or 3).
    #[inline]
    pub const fn index(&self) -> usize {
        match self {
            OneOf4::T1(_) => 0,
            OneOf4::T2(_) => 1,
            OneOf4::T3(_) => 2,
            OneOf4::T4(_) => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_new() {
        let _aligned = Aligned::<16>::new();
    }

    #[test]
    fn test_pun() {
        let pun = Pun::<u32, [u8; 4]>::new(0x12345678);
        assert_eq!(pun.into_left(), 0x12345678);
    }

    #[test]
    fn test_pun_from_right() {
        let bytes = [0x78, 0x56, 0x34, 0x12];
        let pun = Pun::<u32, [u8; 4]>::from_right(bytes);
        assert_eq!(pun.into_right(), bytes);
    }

    #[test]
    fn test_tagged() {
        let tagged: Tagged<i32, &str> = Tagged::A(42);
        assert!(tagged.is_a());
        assert!(!tagged.is_b());
        assert_eq!(tagged.a(), Some(&42));
        assert_eq!(tagged.b(), None);
    }

    #[test]
    fn test_tagged_map() {
        let tagged: Tagged<i32, &str> = Tagged::A(42);
        let mapped = tagged.map_a(|n| n * 2);
        assert_eq!(mapped, Tagged::A(84));
    }

    #[test]
    fn test_pair() {
        let pair = Pair::new(1, "hello");
        assert_eq!(pair.get_first(), &1);
        assert_eq!(pair.get_second(), &"hello");
    }

    #[test]
    fn test_pair_swap() {
        let pair = Pair::new(1, "hello");
        let swapped = pair.swap();
        assert_eq!(swapped.get_first(), &"hello");
        assert_eq!(swapped.get_second(), &1);
    }

    #[test]
    fn test_pair_from_tuple() {
        let tuple = (1, "hello");
        let pair = Pair::from(tuple);
        assert_eq!(pair.get_first(), &1);
        assert_eq!(pair.get_second(), &"hello");
    }

    #[test]
    fn test_triple() {
        let triple = Triple::new(1, "hello", 3.14);
        assert_eq!(triple.get_first(), &1);
        assert_eq!(triple.get_second(), &"hello");
        assert_eq!(triple.get_third(), &3.14);
    }

    #[test]
    fn test_triple_from_tuple() {
        let tuple = (1, "hello", 3.14);
        let triple = Triple::from(tuple);
        assert_eq!(triple.get_first(), &1);
        assert_eq!(triple.get_second(), &"hello");
        assert_eq!(triple.get_third(), &3.14);
    }

    #[test]
    fn test_one_of3_t1() {
        let result: OneOf3<i32, &str, bool> = OneOf3::T1(42);
        assert_eq!(result.index(), 0);
        assert_eq!(result.t1(), Some(&42));
        assert_eq!(result.t2(), None);
        assert_eq!(result.t3(), None);
    }

    #[test]
    fn test_one_of3_t2() {
        let result: OneOf3<i32, &str, bool> = OneOf3::T2("hello");
        assert_eq!(result.index(), 1);
        assert_eq!(result.t1(), None);
        assert_eq!(result.t2(), Some(&"hello"));
        assert_eq!(result.t3(), None);
    }

    #[test]
    fn test_one_of3_t3() {
        let result: OneOf3<i32, &str, bool> = OneOf3::T3(true);
        assert_eq!(result.index(), 2);
        assert_eq!(result.t1(), None);
        assert_eq!(result.t2(), None);
        assert_eq!(result.t3(), Some(&true));
    }

    #[test]
    fn test_one_of4_t1() {
        let result: OneOf4<i32, &str, bool, f64> = OneOf4::T1(42);
        assert_eq!(result.index(), 0);
    }

    #[test]
    fn test_one_of4_t2() {
        let result: OneOf4<i32, &str, bool, f64> = OneOf4::T2("hello");
        assert_eq!(result.index(), 1);
    }

    #[test]
    fn test_one_of4_t3() {
        let result: OneOf4<i32, &str, bool, f64> = OneOf4::T3(true);
        assert_eq!(result.index(), 2);
    }

    #[test]
    fn test_one_of4_t4() {
        let result: OneOf4<i32, &str, bool, f64> = OneOf4::T4(3.14);
        assert_eq!(result.index(), 3);
    }
}
