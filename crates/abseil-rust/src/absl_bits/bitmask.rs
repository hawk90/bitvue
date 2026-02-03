//! Bit mask and bit field types.
//!
//! This module provides types for manipulating bits within integers:
//! - `BitMask<T>` - comprehensive bit mask type
//! - `BitField<T>` - for accessing named bit fields
//! - `BitBuilder<T>` - for constructing bit patterns

use core::ops::{BitAnd, BitOr, BitXor, Not};
use crate::bits;

/// A bit mask for manipulating bits within integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitMask<T> {
    value: T,
}

impl<T> BitMask<T>
where
    T: Copy
        + BitAnd<Output = T>
        + BitOr<Output = T>
        + BitXor<Output = T>
        + Not<Output = T>
        + PartialEq
        + Eq,
{
    /// Creates a new bit mask with all bits set to 0.
    pub const fn zero() -> Self {
        Self { value: T::zero() }
    }

    /// Creates a new bit mask with all bits set to 1.
    pub fn ones() -> Self
    where
        T: From<u8>,
    {
        Self { value: T::from(!0u8) }
    }

    /// Creates a new bit mask with the specified value.
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Returns the underlying value.
    pub const fn get(&self) -> T {
        self.value
    }

    /// Sets the bit at the specified position.
    pub fn set_bit(mut self, position: u32) -> Self
    where
        T: core::ops::Shl<u32, Output = T> + core::ops::BitOr<T, Output = T> + From<u8>,
    {
        self.value = self.value | (T::from(1u8) << position);
        self
    }

    /// Clears the bit at the specified position.
    pub fn clear_bit(mut self, position: u32) -> Self
    where
        T: core::ops::Shl<u32, Output = T> + core::ops::Not<Output = T> + core::ops::BitAnd<T, Output = T>,
    {
        self.value = self.value & !(T::from(1u8) << position);
        self
    }

    /// Toggles the bit at the specified position.
    pub fn toggle_bit(mut self, position: u32) -> Self
    where
        T: core::ops::Shl<u32, Output = T> + core::ops::BitXor<T, Output = T>,
    {
        self.value = self.value ^ (T::from(1u8) << position);
        self
    }

    /// Returns true if the bit at the specified position is set.
    pub fn is_set(&self, position: u32) -> bool
    where
        T: core::ops::Shr<u32, Output = T> + From<u8>,
    {
        (self.value >> position) & T::from(1u8) != T::zero()
    }

    /// Returns true if the bit at the specified position is clear.
    pub fn is_clear(&self, position: u32) -> bool
    where
        T: core::ops::Shr<u32, Output = T> + From<u8>,
    {
        !self.is_set(position)
    }

    /// Performs a bitwise AND with another mask.
    pub fn and(self, other: BitMask<T>) -> BitMask<T> {
        BitMask {
            value: self.value & other.value,
        }
    }

    /// Performs a bitwise OR with another mask.
    pub fn or(self, other: BitMask<T>) -> BitMask<T> {
        BitMask {
            value: self.value | other.value,
        }
    }

    /// Performs a bitwise XOR with another mask.
    pub fn xor(self, other: BitMask<T>) -> BitMask<T> {
        BitMask {
            value: self.value ^ other.value,
        }
    }

    /// Returns the bitwise NOT of this mask.
    pub fn not(self) -> BitMask<T> {
        BitMask { value: !self.value }
    }

    /// Sets a range of bits to 1.
    ///
    /// # Panics
    ///
    /// Panics if end > width or if start >= end.
    pub fn set_range(mut self, start: u32, end: u32) -> Self
    where
        T: bit_width::BitWidth
            + core::ops::Shl<u32, Output = T>
            + BitOr<T, Output = T>
            + Sub<T, Output = T>
            + From<u8>
            + Copy,
        u32: Into<T>,
    {
        let width = bit_width::BitWidth::bit_width(&self.value);
        let range_len = end - start;
        if range_len > width {
            panic!("set_range: range length ({}) must be <= width ({})", range_len, width);
        }
        if start >= end {
            panic!("set_range: start ({}) must be < end ({})", start, end);
        }
        let mask = (T::from(!0u8) >> (width - range_len)) & (T::from(!0u8) << start);
        self.value = self.value | mask;
        self
    }

    /// Clears a range of bits to 0.
    ///
    /// # Panics
    ///
    /// Panics if end > width or if start >= end.
    pub fn clear_range(mut self, start: u32, end: u32) -> Self
    where
        T: bit_width::BitWidth
            + core::ops::Shl<u32, Output = T>
            + Not<Output = T>
            + BitAnd<T, Output = T>
            + From<u8>
            + Copy,
    {
        let width = bit_width::BitWidth::bit_width(&self.value);
        let range_len = end - start;
        if range_len > width {
            panic!("clear_range: range length ({}) must be <= width ({})", range_len, width);
        }
        if start >= end {
            panic!("clear_range: start ({}) must be < end ({})", start, end);
        }
        let mask = (T::from(!0u8) >> (width - range_len)) & (T::from(!0u8) << start);
        self.value = self.value & !mask;
        self
    }

    /// Extracts a range of bits.
    ///
    /// # Panics
    ///
    /// Panics if end > width or if start >= end.
    pub fn extract_range(&self, start: u32, end: u32) -> T
    where
        T: bit_width::BitWidth
            + core::ops::Shr<u32, Output = T>
            + core::ops::Shl<u32, Output = T>
            + BitAnd<T, Output = T>
            + From<u8>
            + Copy
            + Sub<T, Output = T>,
    {
        let width = bit_width::BitWidth::bit_width(&self.value);
        let range_len = end - start;
        if range_len > width {
            panic!("extract_range: range length ({}) must be <= width ({})", range_len, width);
        }
        if start >= end {
            panic!("extract_range: start ({}) must be < end ({})", start, end);
        }
        let mask = (T::from(!0u8) >> (width - range_len)) & (T::from(!0u8) << start);
        (self.value & mask) >> start
    }

    /// Inserts a value into a range of bits.
    ///
    /// # Panics
    ///
    /// Panics if end > width, if start >= end, or if value is too large for the range.
    pub fn insert_range(mut self, start: u32, end: u32, value: T) -> Self
    where
        T: bit_width::BitWidth
            + core::ops::Shl<u32, Output = T>
            + BitAnd<T, Output = T>
            + BitOr<T, Output = T>
            + Not<Output = T>
            + From<u8>
            + Copy,
    {
        let width = bit_width::BitWidth::bit_width(&self.value);
        let range_len = end - start;
        if range_len > width {
            panic!("insert_range: range length ({}) must be <= width ({})", range_len, width);
        }
        if start >= end {
            panic!("insert_range: start ({}) must be < end ({})", start, end);
        }
        let mask = (T::from(!0u8) >> (width - range_len)) & (T::from(!0u8) << start);
        self.value = (self.value & !mask) | ((value & (T::from(!0u8) >> (width - range_len))) << start);
        self
    }

    /// Returns the number of bits set in the mask.
    pub fn count_ones(&self) -> u32
    where
        T: Copy,
    {
        bits::popcount(self.value)
    }

    /// Returns the number of bits clear in the mask.
    pub fn count_zeros(&self) -> u32
    where
        T: Copy + bit_width::BitWidth,
    {
        T::bit_width() - self.count_ones()
    }

    /// Returns the position of the highest set bit, or None if all bits are clear.
    pub fn highest_set_bit(&self) -> Option<u32>
    where
        T: Copy,
    {
        bits::highest_bit(self.value)
    }

    /// Returns the position of the lowest set bit, or None if all bits are clear.
    pub fn lowest_set_bit(&self) -> Option<u32>
    where
        T: Copy,
    {
        bits::lowest_bit(self.value)
    }

    /// Returns the position of the highest clear bit, or None if all bits are set.
    pub fn highest_clear_bit(&self) -> Option<u32>
    where
        T: Copy + bit_width::BitWidth + Not<Output = T>,
    {
        bits::highest_bit(!self.value)
    }

    /// Returns the position of the lowest clear bit, or None if all bits are clear.
    pub fn lowest_clear_bit(&self) -> Option<u32>
    where
        T: Copy + bit_width::BitWidth + Not<Output = T>,
    {
        bits::lowest_bit(!self.value)
    }

    /// Shifts all bits left by n positions, filling with zeros.
    pub fn shift_left(self, n: u32) -> Self
    where
        T: core::ops::Shl<u32, Output = T>,
    {
        BitMask {
            value: self.value << n,
        }
    }

    /// Shifts all bits right by n positions, filling with sign bit.
    pub fn shift_right(self, n: u32) -> Self
    where
        T: core::ops::Shr<u32, Output = T>,
    {
        BitMask {
            value: self.value >> n,
        }
    }

    /// Shifts all bits right by n positions, filling with zeros.
    pub fn shift_right_unsigned(self, n: u32) -> Self
    where
        T: core::ops::Shr<u32, Output = T>,
    {
        BitMask {
            value: self.value >> n,
        }
    }

    /// Rotates bits left by n positions.
    pub fn rotate_left(self, n: u32) -> Self
    where
        T: Copy + bit_width::BitWidth,
    {
        BitMask {
            value: bits::rotate_left(self.value, n),
        }
    }

    /// Rotates bits right by n positions.
    pub fn rotate_right(self, n: u32) -> Self
    where
        T: Copy + bit_width::BitWidth,
    {
        BitMask {
            value: bits::rotate_right(self.value, n),
        }
    }

    /// Reverses the order of bits.
    pub fn reverse_bits(self) -> Self
    where
        T: Copy,
    {
        BitMask {
            value: bits::reverse_bits(self.value),
        }
    }

    /// Reverses the order of bytes.
    pub fn reverse_bytes(self) -> Self
    where
        T: Copy,
    {
        BitMask {
            value: bits::reverse_bytes(self.value),
        }
    }

    /// Creates a bitmask with only the specified bit set.
    pub fn single_bit(position: u32) -> Self
    where
        T: core::ops::Shl<u32, Output = T> + From<u8>,
    {
        BitMask::new(T::from(1u8) << position)
    }

    /// Creates a bitmask with a range of bits set.
    pub fn range(start: u32, end: u32) -> Self
    where
        T: bit_width::BitWidth
            + Copy
            + core::ops::Shl<u32, Output = T>
            + core::ops::Sub<T, Output = T>
            + Not<Output = T>
            + BitAnd<T, Output = T>
            + From<u8>,
    {
        BitMask::ones().set_range(start, end)
    }

    /// Returns a bitmask with the lower n bits set.
    ///
    /// # Panics
    ///
    /// Panics if n > width of the type.
    pub fn lower_bits(n: u32) -> Self
    where
        T: bit_width::BitWidth + Copy + From<u8>,
    {
        let width = bit_width::BitWidth::bit_width(&self.value);
        if n > width {
            panic!("lower_bits: n ({}) must be <= width ({})", n, width);
        }
        BitMask::ones().shift_right(width - n)
    }

    /// Returns a bitmask with the upper n bits set.
    ///
    /// # Panics
    ///
    /// Panics if n > width of the type.
    pub fn upper_bits(n: u32) -> Self
    where
        T: bit_width::BitWidth + Copy,
    {
        let width = bit_width::BitWidth::bit_width(&self.value);
        if n > width {
            panic!("upper_bits: n ({}) must be <= width ({})", n, width);
        }
        BitMask::ones().shift_left(width - n)
    }
}

impl<T> Default for BitMask<T>
where
    T: Default,
{
    fn default() -> Self {
        BitMask::new(T::default())
    }
}

impl<T> PartialEq<T> for BitMask<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        self.value == *other
    }
}

/// A bit field for accessing named bits within an integer.
#[derive(Clone, Debug)]
pub struct BitField<T> {
    offset: u32,
    width: u32,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> BitField<T>
where
    T: Copy
        + BitAnd<Output = T>
        + BitOr<Output = T>
        + BitXor<Output = T>
        + core::ops::Shr<u32, Output = T>
        + core::ops::Shl<u32, Output = T>
        + From<u8>
        + PartialEq
        + Eq,
{
    /// Creates a new bit field.
    pub const fn new(offset: u32, width: u32) -> Self {
        Self {
            offset,
            width,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Extracts the bit field value from the given value.
    ///
    /// # Panics
    ///
    /// Panics if the bit field's width > width of type T, or if offset + width > width of T.
    pub fn extract(&self, value: T) -> T
    where
        T: core::ops::Shr<u32, Output = T>
            + From<u8>
            + Copy
            + Sub<T, Output = T>,
    {
        let width = bit_width::BitWidth::bit_width(&value);
        if self.width > width {
            panic!("BitField::extract: width ({}) must be <= type width ({})", self.width, width);
        }
        if self.offset + self.width > width {
            panic!("BitField::extract: offset + width ({}) must be <= type width ({})", self.offset + self.width, width);
        }
        let mask = (T::from(!0u8) >> (width - self.width)) << self.offset;
        (value & mask) >> self.offset
    }

    /// Inserts a value into the bit field position of the given value.
    ///
    /// # Panics
    ///
    /// Panics if the bit field's width > width of type T, or if offset + width > width of T.
    pub fn insert(&self, value: T, target: T) -> T
    where
        T: core::ops::Shl<u32, Output = T>
            + BitAnd<T, Output = T>
            + BitOr<T, Output = T>
            + Not<Output = T>
            + From<u8>
            + Copy
            + Sub<T, Output = T>,
    {
        let width = bit_width::BitWidth::bit_width(&value);
        if self.width > width {
            panic!("BitField::insert: width ({}) must be <= type width ({})", self.width, width);
        }
        if self.offset + self.width > width {
            panic!("BitField::insert: offset + width ({}) must be <= type width ({})", self.offset + self.width, width);
        }
        let mask = (T::from(!0u8) >> (width - self.width)) << self.offset;
        (target & !mask) | ((value & (T::from(!0u8) >> (width - self.width))) << self.offset)
    }

    /// Returns the mask for this bit field.
    ///
    /// # Panics
    ///
    /// Panics if the bit field's width > width of type T, or if offset + width > width of T.
    pub fn mask(&self) -> T
    where
        T: bit_width::BitWidth + From<u8>,
    {
        let width = bit_width::BitWidth::bit_width(&self.value);
        if self.width > width {
            panic!("BitField::mask: width ({}) must be <= type width ({})", self.width, width);
        }
        if self.offset + self.width > width {
            panic!("BitField::mask: offset + width ({}) must be <= type width ({})", self.offset + self.width, width);
        }
        (T::from(!0u8) >> (width - self.width)) << self.offset
    }
}

/// A bit builder for constructing bit patterns.
#[derive(Clone, Debug, Default)]
pub struct BitBuilder<T> {
    value: T,
}

impl<T> BitBuilder<T>
where
    T: Default
        + BitOr<T, Output = T>
        + core::ops::Shl<u32, Output = T>
        + From<u8>,
{
    /// Creates a new bit builder with value 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new bit builder with an initial value.
    pub fn with_value(value: T) -> Self {
        Self { value }
    }

    /// Sets a bit at the specified position.
    pub fn set(mut self, position: u32) -> Self {
        self.value = self.value | (T::from(1u8) << position);
        self
    }

    /// Sets a range of bits.
    pub fn set_range(self, start: u32, end: u32) -> Self
    where
        T: BitOr<T, Output = T>,
    {
        super::BitMask::new(self.value).set_range(start, end).into_inner()
    }

    /// Clears a bit at the specified position.
    pub fn clear(self, position: u32) -> Self
    where
        T: BitAnd<T, Output = T>
            + Not<Output = T>
            + core::ops::Shl<u32, Output = T>
            + From<u8>,
    {
        super::BitMask::new(self.value).clear_bit(position).into_inner()
    }

    /// Clears a range of bits.
    pub fn clear_range(self, start: u32, end: u32) -> Self
    where
        T: BitAnd<T, Output = T>,
    {
        super::BitMask::new(self.value).clear_range(start, end).into_inner()
    }

    /// Toggles a bit at the specified position.
    pub fn toggle(mut self, position: u32) -> Self
    where
        T: BitXor<T, Output = T>
            + core::ops::Shl<u32, Output = T>
            + From<u8>,
    {
        self.value = self.value ^ (T::from(1u8) << position);
        self
    }

    /// Builds and returns the final value.
    pub fn build(self) -> T {
        self.value
    }

    /// Returns the current value.
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.value
    }
}

impl<T> From<BitBuilder<T>> for T
where
    T: From<BitBuilder<T>>,
{
    fn from(builder: BitBuilder<T>) -> Self {
        builder.build()
    }
}

/// Extension trait for BitMask to provide into_inner method.
impl<T> BitMask<T>
where
    T: Copy,
{
    /// Converts BitMask back to its inner value (as BitBuilder conversion).
    pub fn into_inner(self) -> BitBuilder<T>
    where
        T: Default,
    {
        BitBuilder::with_value(self.value)
    }
}

/// A bit position type for compile-time bit position constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitPosition<T> {
    position: u32,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> BitPosition<T> {
    /// Creates a new bit position.
    pub const fn new(position: u32) -> Self {
        Self {
            position,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Returns the bit position.
    pub const fn position(&self) -> u32 {
        self.position
    }

    /// Returns a mask with this bit set.
    pub fn mask(&self) -> u64
    where
        T: Into<u64>,
    {
        1u64 << self.position
    }
}

impl<T> From<BitPosition<T>> for u32 {
    fn from(pos: BitPosition<T>) -> Self {
        pos.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_mask_set_bit() {
        let mask: BitMask<u8> = BitMask::zero().set_bit(2);
        assert_eq!(mask.get(), 0b00000100);
    }

    #[test]
    fn test_bit_mask_clear_bit() {
        let mask: BitMask<u8> = BitMask::ones().clear_bit(2);
        assert_eq!(mask.get(), 0b11111011);
    }

    #[test]
    fn test_bit_mask_toggle_bit() {
        let mask: BitMask<u8> = BitMask::zero().toggle_bit(2);
        assert_eq!(mask.get(), 0b00000100);
    }

    #[test]
    fn test_bit_mask_is_set() {
        let mask: BitMask<u8> = BitMask::zero().set_bit(2);
        assert!(mask.is_set(2));
        assert!(!mask.is_set(0));
    }

    #[test]
    fn test_bit_mask_is_clear() {
        let mask: BitMask<u8> = BitMask::zero();
        assert!(mask.is_clear(2));
    }

    #[test]
    fn test_bit_mask_count_ones() {
        let mask: BitMask<u8> = BitMask::new(0b10101010);
        assert_eq!(mask.count_ones(), 4);
    }

    #[test]
    fn test_bit_mask_single_bit() {
        let mask: BitMask<u8> = BitMask::single_bit(3);
        assert_eq!(mask.get(), 0b00001000);
    }

    #[test]
    fn test_bit_mask_range() {
        let mask: BitMask<u8> = BitMask::range(2, 5);
        assert_eq!(mask.get(), 0b00011100);
    }

    #[test]
    fn test_bit_field_extract() {
        let field = BitField::<u32>::new(8, 4);
        let value = 0x12345678;
        assert_eq!(field.extract(value), 0x6);
    }

    #[test]
    fn test_bit_field_insert() {
        let field = BitField::<u32>::new(8, 4);
        let value = 0x12345678;
        let result = field.insert(0xA, value);
        assert_eq!(result, 0x12345A78);
    }

    #[test]
    fn test_bit_field_mask() {
        let field = BitField::<u32>::new(8, 4);
        assert_eq!(field.mask(), 0xF00);
    }

    #[test]
    fn test_bit_builder_set() {
        let builder: BitBuilder<u32> = BitBuilder::new().set(2).set(5);
        assert_eq!(builder.get(), 0b100100);
    }

    #[test]
    fn test_bit_builder_clear() {
        let builder: BitBuilder<u32> = BitBuilder::new()
            .set(0)
            .set(1)
            .clear(0)
            .set(2);
        assert_eq!(builder.get(), 0b110);
    }

    #[test]
    fn test_bit_builder_toggle() {
        let builder: BitBuilder<u32> = BitBuilder::new().toggle(2);
        assert_eq!(builder.get(), 0b100);
    }

    #[test]
    fn test_bit_position_mask() {
        let pos = BitPosition::<u8>::new(3);
        assert_eq!(pos.mask(), 0b00001000);
    }

    #[test]
    fn test_bit_position_position() {
        let pos = BitPosition::<u32>::new(5);
        assert_eq!(pos.position(), 5);
    }
}
