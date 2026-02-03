//! Endianness conversion utilities.
//!
//! This module provides types and functions for converting between
//! little-endian and big-endian byte orders.

/// Endianness marker type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endianness {
    /// Little-endian byte order.
    Little,
    /// Big-endian byte order.
    Big,
}

/// Converts between endianness for 16-bit values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::endianness::swap_u16;
///
/// assert_eq!(swap_u16(0x1234), 0x3412);
/// ```
pub const fn swap_u16(x: u16) -> u16 {
    x.rotate_right(8)
}

/// Converts between endianness for 32-bit values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::endianness::swap_u32;
///
/// assert_eq!(swap_u32(0x12345678), 0x78563412);
/// ```
pub const fn swap_u32(x: u32) -> u32 {
    x.rotate_right(8).swap_bytes()
}

/// Converts between endianness for 64-bit values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::endianness::swap_u64;
///
/// assert_eq!(swap_u64(0x0123456789ABCDEF), 0xEFCDAB8967452301);
/// ```
pub const fn swap_u64(x: u64) -> u64 {
    x.swap_bytes()
}

/// Converts from big-endian to native byte order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::endianness::from_be;
///
/// assert_eq!(from_be(0x12u16), 0x12); // On little-endian
/// ```
pub const fn from_be<T: FromBe>(x: T) -> T {
    T::from_be(x)
}

/// Converts from little-endian to native byte order.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::endianness::from_le;
///
/// assert_eq!(from_le(0x12u16), 0x12); // On little-endian
/// ```
pub const fn from_le<T: FromLe>(x: T) -> T {
    T::from_le(x)
}

/// Converts to big-endian byte order.
pub const fn to_be<T: ToBe>(x: T) -> T {
    T::to_be(x)
}

/// Converts to little-endian byte order.
pub const fn to_le<T: ToLe>(x: T) -> T {
    T::to_le(x)
}

/// Trait for converting from big-endian.
pub trait FromBe: Sized {
    fn from_be(x: Self) -> Self;
}

impl FromBe for u8 {
    fn from_be(x: Self) -> Self { x }
}

impl FromBe for u16 {
    fn from_be(x: Self) -> Self { u16::from_be(x) }
}

impl FromBe for u32 {
    fn from_be(x: Self) -> Self { u32::from_be(x) }
}

impl FromBe for u64 {
    fn from_be(x: Self) -> Self { u64::from_be(x) }
}

impl FromBe for i8 {
    fn from_be(x: Self) -> Self { x }
}

impl FromBe for i16 {
    fn from_be(x: Self) -> Self { i16::from_be(x) }
}

impl FromBe for i32 {
    fn from_be(x: Self) -> Self { i32::from_be(x) }
}

impl FromBe for i64 {
    fn from_be(x: Self) -> Self { i64::from_be(x) }
}

/// Trait for converting from little-endian.
pub trait FromLe: Sized {
    fn from_le(x: Self) -> Self;
}

impl FromLe for u8 {
    fn from_le(x: Self) -> Self { x }
}

impl FromLe for u16 {
    fn from_le(x: Self) -> Self { u16::from_le(x) }
}

impl FromLe for u32 {
    fn from_le(x: Self) -> Self { u32::from_le(x) }
}

impl FromLe for u64 {
    fn from_le(x: Self) -> Self { u64::from_le(x) }
}

impl FromLe for i8 {
    fn from_le(x: Self) -> Self { x }
}

impl FromLe for i16 {
    fn from_le(x: Self) -> Self { i16::from_le(x) }
}

impl FromLe for i32 {
    fn from_le(x: Self) -> Self { i32::from_le(x) }
}

impl FromLe for i64 {
    fn from_le(x: Self) -> Self { i64::from_le(x) }
}

/// Trait for converting to big-endian.
pub trait ToBe: Sized {
    fn to_be(self) -> Self;
}

impl ToBe for u8 {
    fn to_be(self) -> Self { self }
}

impl ToBe for u16 {
    fn to_be(self) -> Self { self.to_be() }
}

impl ToBe for u32 {
    fn to_be(self) -> Self { self.to_be() }
}

impl ToBe for u64 {
    fn to_be(self) -> Self { self.to_be() }
}

/// Trait for converting to little-endian.
pub trait ToLe: Sized {
    fn to_le(self) -> Self;
}

impl ToLe for u8 {
    fn to_le(self) -> Self { self }
}

impl ToLe for u16 {
    fn to_le(self) -> Self { self.to_le() }
}

impl ToLe for u32 {
    fn to_le(self) -> Self { self.to_le() }
}

impl ToLe for u64 {
    fn to_le(self) -> Self { self.to_le() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for endianness conversion
    #[test]
    fn test_swap_u16() {
        assert_eq!(swap_u16(0x1234), 0x3412);
        assert_eq!(swap_u16(0x0001), 0x0100);
    }

    #[test]
    fn test_swap_u32() {
        assert_eq!(swap_u32(0x12345678), 0x78563412);
        assert_eq!(swap_u32(0x00000001), 0x01000000);
    }

    #[test]
    fn test_swap_u64() {
        assert_eq!(swap_u64(0x0123456789ABCDEF), 0xEFCDAB8967452301);
    }
}
