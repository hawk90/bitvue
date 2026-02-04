//! Bit casting utilities and permutation functions.
//!
//! This module provides:
//! - Bit casting between types of the same size
//! - Bit matrix for bit-level operations
//! - Bit permutation utilities

/// A simple bit matrix for bit-level operations.
#[derive(Clone, Debug)]
pub struct BitMatrix {
    data: Vec<u64>,
    cols: usize,
}

impl BitMatrix {
    /// Creates a new bit matrix with the specified dimensions.
    pub fn new(rows: usize, cols: usize) -> Self {
        let words_per_row = (cols + 63) / 64;
        BitMatrix {
            data: vec![0; rows * words_per_row],
            cols,
        }
    }

    /// Returns the number of rows.
    pub fn rows(&self) -> usize {
        if self.cols == 0 {
            0
        } else {
            self.data.len() / ((self.cols + 63) / 64)
        }
    }

    /// Returns the number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Sets a bit at the specified position.
    pub fn set(&mut self, row: usize, col: usize, value: bool) {
        if row >= self.rows() || col >= self.cols {
            return;
        }
        let word_idx = row * ((self.cols + 63) / 64) + (col / 64);
        let bit_idx = col % 64;
        if value {
            self.data[word_idx] |= 1u64 << bit_idx;
        } else {
            self.data[word_idx] &= !(1u64 << bit_idx);
        }
    }

    /// Gets the value of a bit at the specified position.
    pub fn get(&self, row: usize, col: usize) -> bool {
        if row >= self.rows() || col >= self.cols {
            return false;
        }
        let word_idx = row * ((self.cols + 63) / 64) + (col / 64);
        let bit_idx = col % 64;
        (self.data[word_idx] >> bit_idx) & 1 == 1
    }

    /// Flips a bit at the specified position.
    pub fn flip(&mut self, row: usize, col: usize) {
        if row >= self.rows() || col >= self.cols {
            return;
        }
        let word_idx = row * ((self.cols + 63) / 64) + (col / 64);
        let bit_idx = col % 64;
        self.data[word_idx] ^= 1u64 << bit_idx;
    }

    /// Clears all bits.
    pub fn clear(&mut self) {
        for word in &mut self.data {
            *word = 0;
        }
    }

    /// Returns true if all bits are clear.
    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|&w| w == 0)
    }

    /// Counts the total number of set bits.
    pub fn count_ones(&self) -> usize {
        self.data.iter().map(|&w| w.count_ones() as usize).sum()
    }
}

/// Bit permutation utilities.
pub struct BitPermutation;

impl BitPermutation {
    /// Reverses the bits in a byte (8-bit).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitPermutation;
    ///
    /// assert_eq!(BitPermutation::reverse_byte(0b11001010), 0b01010011);
    /// ```
    pub const fn reverse_byte(mut b: u8) -> u8 {
        b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
        b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
        b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
        b
    }

    /// Reverses the bits in a 16-bit value.
    pub const fn reverse_u16(mut x: u16) -> u16 {
        x = ((x & 0xFF00) >> 8) | ((x & 0x00FF) << 8);
        x = ((x & 0xF0F0) >> 4) | ((x & 0x0F0F) << 4);
        x = ((x & 0xCCCC) >> 2) | ((x & 0x3333) << 2);
        x = ((x & 0xAAAA) >> 1) | ((x & 0x5555) << 1);
        x
    }

    /// Rotates bits left by a specified count.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitPermutation;
    ///
    /// assert_eq!(BitPermutation::rotl_u8(0b10000001, 1), 0b00000011);
    /// ```
    pub const fn rotl_u8(x: u8, n: u32) -> u8 {
        // Mask n to [0, 7] to prevent underflow in (8 - n) and undefined behavior
        // from shifting by >= bit width. Rotation by multiples of 8 is identity.
        let n = n & 7;
        (x << n) | (x >> (8 - n))
    }

    /// Rotates bits right by a specified count.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitPermutation;
    ///
    /// assert_eq!(BitPermutation::rotr_u8(0b10000001, 1), 0b11000000);
    /// ```
    pub const fn rotr_u8(x: u8, n: u32) -> u8 {
        // Mask n to [0, 7] to prevent underflow in (8 - n) and undefined behavior
        // from shifting by >= bit width. Rotation by multiples of 8 is identity.
        let n = n & 7;
        (x >> n) | (x << (8 - n))
    }

    /// Merges two bytes by alternating bits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitPermutation;
    ///
    /// // merge_bytes(0b10101010, 0b11001100) returns bytes where
    /// // result bits are: a[7], b[7], a[6], b[6], ...
    /// ```
    pub const fn merge_bytes(a: u8, b: u8) -> u16 {
        let mut result = 0u16;
        let mut x = a as u16;
        let mut y = b as u16;

        let mut i = 0;
        while i < 8 {
            result |= ((x & 1) << (2 * i + 1)) | ((y & 1) << (2 * i));
            x >>= 1;
            y >>= 1;
            i += 1;
        }

        result
    }

    /// Splits a 16-bit value into two bytes by de-interleaving bits.
    pub const fn split_bytes(value: u16) -> (u8, u8) {
        let mut a = 0u8;
        let mut b = 0u8;
        let mut v = value;

        let mut i = 0;
        while i < 8 {
            a |= ((v & 1) as u8) << i;
            v >>= 1;
            b |= ((v & 1) as u8) << i;
            v >>= 1;
            i += 1;
        }

        (a, b)
    }
}

/// Bit twiddling hacks collection.
pub struct BitHacks;

impl BitHacks {
    /// Checks if exactly one bit is set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert!(BitHacks::has_single_bit(0b00100000u8));
    /// assert!(!BitHacks::has_single_bit(0b00100100u8));
    /// ```
    pub const fn has_single_bit(x: u8) -> bool {
        x != 0 && (x & (x - 1)) == 0
    }

    /// Checks if exactly one bit is set (16-bit).
    pub const fn has_single_bit16(x: u16) -> bool {
        x != 0 && (x & (x - 1)) == 0
    }

    /// Checks if exactly one bit is set (32-bit).
    pub const fn has_single_bit32(x: u32) -> bool {
        x != 0 && (x & (x - 1)) == 0
    }

    /// Computes the average of two integers without overflow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert_eq!(BitHacks::average(10u8, 20u8), 15);
    /// ```
    pub const fn average(a: u8, b: u8) -> u8 {
        (a & b) + ((a ^ b) >> 1)
    }

    /// Computes the average of two 16-bit integers.
    pub const fn average16(a: u16, b: u16) -> u16 {
        (a & b) + ((a ^ b) >> 1)
    }

    /// Computes the average of two 32-bit integers.
    pub const fn average32(a: u32, b: u32) -> u32 {
        (a & b) + ((a ^ b) >> 1)
    }

    /// Determines if two integers have the same sign.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert!(BitHacks::same_sign(5i8, 3i8));
    /// assert!(!BitHacks::same_sign(5i8, -3i8));
    /// ```
    pub const fn same_sign(a: i8, b: i8) -> bool {
        (a ^ b) >= 0
    }

    /// Determines if two 32-bit integers have the same sign.
    pub const fn same_sign32(a: i32, b: i32) -> bool {
        (a ^ b) >= 0
    }

    /// Computes the absolute value without branching.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert_eq!(BitHacks::abs_branchless(-5i8), 5);
    /// assert_eq!(BitHacks::abs_branchless(5i8), 5);
    /// ```
    pub const fn abs_branchless(x: i8) -> i8 {
        let mask = x >> 7;
        (x + mask) ^ mask
    }

    /// Computes the absolute value of a 32-bit integer without branching.
    pub const fn abs_branchless32(x: i32) -> i32 {
        let mask = x >> 31;
        (x + mask) ^ mask
    }

    /// Returns the minimum of two integers without branching.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert_eq!(BitHacks::min_branchless(5i8, 3i8), 3);
    /// ```
    pub const fn min_branchless(a: i8, b: i8) -> i8 {
        b ^ ((a ^ b) & -(a < b as i8))
    }

    /// Returns the maximum of two integers without branching.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert_eq!(BitHacks::max_branchless(5i8, 3i8), 5);
    /// ```
    pub const fn max_branchless(a: i8, b: i8) -> i8 {
        a ^ ((a ^ b) & -(a < b as i8))
    }

    /// Returns the sign of an integer (-1, 0, or 1).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert_eq!(BitHacks::sign(-5i8), -1);
    /// assert_eq!(BitHacks::sign(0i8), 0);
    /// assert_eq!(BitHacks::sign(5i8), 1);
    /// ```
    pub const fn sign(x: i8) -> i8 {
        (x > 0 as i8) as i8 - (x < 0 as i8) as i8
    }

    /// Checks if a number is a power of two (0 and 1 return false).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert!(BitHacks::is_power_of_two(8u8));
    /// assert!(!BitHacks::is_power_of_two(0u8));
    /// ```
    pub const fn is_power_of_two(x: u8) -> bool {
        x > 1 && (x & (x - 1)) == 0
    }

    /// Modulo by power of two without division.
    ///
    /// # Panics
    ///
    /// Panics if power is 0 or not a power of two.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert_eq!(BitHacks::mod_power_of_two(13u8, 8), 5); // 13 % 8 = 5
    /// ```
    pub const fn mod_power_of_two(x: u8, power: u8) -> u8 {
        // Validate power is non-zero and a power of two to prevent underflow
        // in (power - 1) and incorrect results for non-powers-of-two.
        // A power of two has exactly one bit set: power & (power - 1) == 0.
        // Also check power != 0 to avoid underflow in the validation itself.
        if power == 0 || (power & (power - 1)) != 0 {
            panic!("mod_power_of_two: power must be a non-zero power of two");
        }
        x & (power - 1)
    }

    /// Checks if a number is even.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert!(BitHacks::is_even(4u8));
    /// assert!(!BitHacks::is_even(5u8));
    /// ```
    pub const fn is_even(x: u8) -> bool {
        (x & 1) == 0
    }

    /// Checks if a number is odd.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// assert!(BitHacks::is_odd(5u8));
    /// assert!(!BitHacks::is_odd(4u8));
    /// ```
    pub const fn is_odd(x: u8) -> bool {
        (x & 1) == 1
    }

    /// Swaps two integers without a temporary variable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// let (mut a, mut b) = (5u8, 3u8);
    /// BitHacks::swap_xor(&mut a, &mut b);
    /// assert_eq!((a, b), (3, 5));
    /// ```
    pub fn swap_xor(a: &mut u8, b: &mut u8) {
        if a != b {
            *a ^= *b;
            *b ^= *a;
            *a ^= *b;
        }
    }

    /// Swaps two integers using addition and subtraction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_bits::bit_cast::BitHacks;
    ///
    /// let (mut a, mut b) = (5u8, 3u8);
    /// BitHacks::swap_add(&mut a, &mut b);
    /// assert_eq!((a, b), (3, 5));
    /// ```
    pub fn swap_add(a: &mut u8, b: &mut u8) {
        *a = *a.wrapping_add(*b);
        *b = *a.wrapping_sub(*b);
        *a = *a.wrapping_sub(*b);
    }
}

/// Safely casts between types of the same size.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_bits::bit_cast::bit_cast;
///
/// let f: f32 = 3.14159;
/// let i: u32 = bit_cast(f);
/// ```
pub fn bit_cast<Src, Dst>(src: Src) -> Dst
where
    Src: IntoBytes,
    Dst: FromBytes<Src::Output>,
{
    Dst::from_bytes(src.into_bytes())
}

/// Trait for converting to bytes.
pub trait IntoBytes: Sized {
    type Output;
    fn into_bytes(self) -> Self::Output;
}

/// Trait for converting from bytes.
pub trait FromBytes<T>: Sized {
    fn from_bytes(bytes: T) -> Self;
}

impl IntoBytes for u8 {
    type Output = [u8; 1];
    fn into_bytes(self) -> Self::Output {
        [self]
    }
}

impl FromBytes<[u8; 1]> for u8 {
    fn from_bytes(bytes: [u8; 1]) -> Self {
        bytes[0]
    }
}

impl IntoBytes for u32 {
    type Output = [u8; 4];
    fn into_bytes(self) -> Self::Output {
        self.to_be_bytes()
    }
}

impl FromBytes<[u8; 4]> for u32 {
    fn from_bytes(bytes: [u8; 4]) -> Self {
        u32::from_be_bytes(bytes)
    }
}

impl IntoBytes for u64 {
    type Output = [u8; 8];
    fn into_bytes(self) -> Self::Output {
        self.to_be_bytes()
    }
}

impl FromBytes<[u8; 8]> for u64 {
    fn from_bytes(bytes: [u8; 8]) -> Self {
        u64::from_be_bytes(bytes)
    }
}

impl IntoBytes for f32 {
    type Output = [u8; 4];
    fn into_bytes(self) -> Self::Output {
        self.to_be_bytes()
    }
}

impl FromBytes<[u8; 4]> for f32 {
    fn from_bytes(bytes: [u8; 4]) -> Self {
        f32::from_be_bytes(bytes)
    }
}

impl IntoBytes for f64 {
    type Output = [u8; 8];
    fn into_bytes(self) -> Self::Output {
        self.to_be_bytes()
    }
}

impl FromBytes<[u8; 8]> for f64 {
    fn from_bytes(bytes: [u8; 8]) -> Self {
        f64::from_be_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_matrix_new() {
        let matrix = BitMatrix::new(8, 8);
        assert_eq!(matrix.rows(), 8);
        assert_eq!(matrix.cols(), 8);
        assert!(matrix.is_empty());
    }

    #[test]
    fn test_bit_matrix_set_get() {
        let mut matrix = BitMatrix::new(8, 8);
        matrix.set(2, 3, true);
        assert!(matrix.get(2, 3));
        assert!(!matrix.get(2, 4));
    }

    #[test]
    fn test_bit_matrix_flip() {
        let mut matrix = BitMatrix::new(8, 8);
        matrix.set(2, 3, true);
        matrix.flip(2, 3);
        assert!(!matrix.get(2, 3));
    }

    #[test]
    fn test_bit_matrix_count_ones() {
        let mut matrix = BitMatrix::new(4, 4);
        matrix.set(0, 0, true);
        matrix.set(0, 1, true);
        matrix.set(1, 0, true);
        assert_eq!(matrix.count_ones(), 3);
    }

    #[test]
    fn test_bit_matrix_clear() {
        let mut matrix = BitMatrix::new(4, 4);
        matrix.set(0, 0, true);
        matrix.set(0, 1, true);
        matrix.clear();
        assert!(matrix.is_empty());
    }

    #[test]
    fn test_bit_permutation_reverse_byte() {
        assert_eq!(BitPermutation::reverse_byte(0b11001010), 0b01010011);
        assert_eq!(BitPermutation::reverse_byte(0b00000000), 0b00000000);
        assert_eq!(BitPermutation::reverse_byte(0b11111111), 0b11111111);
        assert_eq!(BitPermutation::reverse_byte(0b10000000), 0b00000001);
    }

    #[test]
    fn test_bit_permutation_reverse_u16() {
        assert_eq!(BitPermutation::reverse_u16(0x1234), 0x2C48);
    }

    #[test]
    fn test_bit_permutation_rotl_u8() {
        assert_eq!(BitPermutation::rotl_u8(0b10000001, 1), 0b00000011);
        assert_eq!(BitPermutation::rotl_u8(0b00000001, 4), 0b00010000);
    }

    #[test]
    fn test_bit_permutation_rotr_u8() {
        assert_eq!(BitPermutation::rotr_u8(0b10000001, 1), 0b11000000);
        assert_eq!(BitPermutation::rotr_u8(0b00010000, 4), 0b00000001);
    }

    #[test]
    fn test_bit_permutation_merge_split_roundtrip() {
        let a = 0b10101010u8;
        let b = 0b11001100u8;
        let merged = BitPermutation::merge_bytes(a, b);
        let (out_a, out_b) = BitPermutation::split_bytes(merged);
        assert_eq!((out_a, out_b), (a, b));
    }

    #[test]
    fn test_bit_hacks_has_single_bit() {
        assert!(BitHacks::has_single_bit(0b00100000u8));
        assert!(!BitHacks::has_single_bit(0b00100100u8));
        assert!(!BitHacks::has_single_bit(0u8));
    }

    #[test]
    fn test_bit_hacks_average() {
        assert_eq!(BitHacks::average(10u8, 20u8), 15);
        assert_eq!(BitHacks::average(5u8, 6u8), 5);
    }

    #[test]
    fn test_bit_hacks_same_sign() {
        assert!(BitHacks::same_sign(5i8, 3i8));
        assert!(!BitHacks::same_sign(5i8, -3i8));
        assert!(BitHacks::same_sign(-5i8, -3i8));
    }

    #[test]
    fn test_bit_hacks_abs_branchless() {
        assert_eq!(BitHacks::abs_branchless(-5i8), 5);
        assert_eq!(BitHacks::abs_branchless(5i8), 5);
        assert_eq!(BitHacks::abs_branchless(0i8), 0);
    }

    #[test]
    fn test_bit_hacks_min_branchless() {
        assert_eq!(BitHacks::min_branchless(5i8, 3i8), 3);
        assert_eq!(BitHacks::min_branchless(3i8, 5i8), 3);
    }

    #[test]
    fn test_bit_hacks_max_branchless() {
        assert_eq!(BitHacks::max_branchless(5i8, 3i8), 5);
        assert_eq!(BitHacks::max_branchless(3i8, 5i8), 5);
    }

    #[test]
    fn test_bit_hacks_sign() {
        assert_eq!(BitHacks::sign(-5i8), -1);
        assert_eq!(BitHacks::sign(0i8), 0);
        assert_eq!(BitHacks::sign(5i8), 1);
    }

    #[test]
    fn test_bit_hacks_is_power_of_two() {
        assert!(BitHacks::is_power_of_two(1u8));
        assert!(BitHacks::is_power_of_two(2u8));
        assert!(BitHacks::is_power_of_two(8u8));
        assert!(!BitHacks::is_power_of_two(0u8));
        assert!(!BitHacks::is_power_of_two(3u8));
    }

    #[test]
    fn test_bit_hacks_mod_power_of_two() {
        assert_eq!(BitHacks::mod_power_of_two(13u8, 8), 5);
        assert_eq!(BitHacks::mod_power_of_two(15u8, 16), 15);
        assert_eq!(BitHacks::mod_power_of_two(16u8, 8), 0);
    }

    #[test]
    fn test_bit_hacks_is_even() {
        assert!(BitHacks::is_even(4u8));
        assert!(!BitHacks::is_even(5u8));
        assert!(BitHacks::is_even(0u8));
    }

    #[test]
    fn test_bit_hacks_is_odd() {
        assert!(BitHacks::is_odd(5u8));
        assert!(!BitHacks::is_odd(4u8));
    }

    #[test]
    fn test_bit_hacks_swap_xor() {
        let (mut a, mut b) = (5u8, 3u8);
        BitHacks::swap_xor(&mut a, &mut b);
        assert_eq!((a, b), (3, 5));
    }

    #[test]
    fn test_bit_hacks_swap_add() {
        let (mut a, mut b) = (5u8, 3u8);
        BitHacks::swap_add(&mut a, &mut b);
        assert_eq!((a, b), (3, 5));
    }
}
