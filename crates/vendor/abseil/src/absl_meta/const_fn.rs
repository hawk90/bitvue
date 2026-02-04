//! Const function wrappers for compile-time evaluation.

/// A wrapper that enables const evaluation for certain operations.
///
/// This provides const fn wrappers around operations that may not
/// always be const in all Rust versions.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::Const;
///
/// assert_eq!(Const::min(1, 2), 1);
/// assert_eq!(Const::max(1, 2), 2);
/// assert_eq!(Const::clamp(5, 0, 10), 5);
/// ```
pub struct Const;

/// Trait for types that support absolute value.
pub trait SignedNum: Copy {
    fn abs(self) -> Self;
}

macro_rules! impl_signed_num {
    ($($t:ty),*) => {
        $(
            impl SignedNum for $t {
                #[inline]
                fn abs(self) -> Self {
                    // Handle the edge case where abs(MIN_VALUE) would overflow
                    // For example, i8::MIN = -128, but i8::MAX = 127
                    // Negating -128 would give 128 which overflows
                    // We saturate at MAX_VALUE in this case
                    if self == Self::MIN {
                        Self::MAX
                    } else if self < 0 {
                        -self
                    } else {
                        self
                    }
                }
            }
        )*
    };
}

impl_signed_num!(i8, i16, i32, i64, i128, isize);

impl Const {
    /// Returns the minimum of two values (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// const MIN: i32 = Const::min(10, 5);
    /// assert_eq!(MIN, 5);
    /// ```
    #[inline]
    pub const fn min<T: Ord>(a: T, b: T) -> T {
        if a < b { a } else { b }
    }

    /// Returns the maximum of two values (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// const MAX: i32 = Const::max(10, 5);
    /// assert_eq!(MAX, 10);
    /// ```
    #[inline]
    pub const fn max<T: Ord>(a: T, b: T) -> T {
        if a > b { a } else { b }
    }

    /// Clamps a value between a minimum and maximum (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// const CLAMPED: i32 = Const::clamp(15, 0, 10);
    /// assert_eq!(CLAMPED, 10);
    /// ```
    #[inline]
    pub const fn clamp<T: Ord>(value: T, min: T, max: T) -> T {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Returns the absolute value of a signed number (const fn).
    ///
    /// # Edge Case Handling
    ///
    /// For signed integers, the absolute value of MIN_VALUE would overflow
    /// (e.g., `i8::MIN = -128`, but `i8::MAX = 127`). This implementation
    /// **saturates at MAX_VALUE** for such inputs to prevent overflow:
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::abs(i32::MIN), i32::MAX); // Saturates, doesn't overflow
    /// assert_eq!(Const::abs(i8::MIN), i8::MAX);
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::abs(-5), 5);
    /// assert_eq!(Const::abs(5), 5);
    /// ```
    #[inline]
    pub const fn abs<T: SignedNum>(value: T) -> T {
        value.abs()
    }

    /// Checks if a value is within a range (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert!(Const::in_range(5, 0, 10));
    /// assert!(!Const::in_range(15, 0, 10));
    /// ```
    #[inline]
    pub const fn in_range<T: PartialOrd>(value: T, min: T, max: T) -> bool {
        value >= min && value <= max
    }

    /// Divides with ceiling instead of floor (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::div_ceil(10, 3), 4);
    /// assert_eq!(Const::div_ceil(9, 3), 3);
    /// ```
    #[inline]
    pub const fn div_ceil(a: u64, b: u64) -> u64 {
        (a + b - 1) / b
    }

    /// Rounds up to the next multiple of b (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::round_up(10, 3), 12);
    /// assert_eq!(Const::round_up(9, 3), 9);
    /// ```
    #[inline]
    pub const fn round_up(a: u64, b: u64) -> u64 {
        (a + b - 1) / b * b
    }

    /// Returns the square of a value (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::sqr(5), 25);
    /// assert_eq!(Const::sqr(-3), 9);
    /// ```
    #[inline]
    pub const fn sqr<T: SignedNum>(value: T) -> T {
        let v = value.abs();
        v * v
    }

    /// Returns x raised to the power of n (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::pow(2u32, 3), 8);
    /// assert_eq!(Const::pow(5u32, 0), 1);
    /// ```
    #[inline]
    pub const fn pow(mut base: u32, mut exp: u32) -> u32 {
        let mut result = 1u32;
        while exp > 0 {
            if exp & 1 == 1 {
                result *= base;
            }
            exp >>= 1;
            base *= base;
        }
        result
    }

    /// Computes the greatest common divisor (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::gcd(48, 18), 6);
    /// assert_eq!(Const::gcd(17, 5), 1);
    /// ```
    #[inline]
    pub const fn gcd(mut a: u64, mut b: u64) -> u64 {
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }

    /// Computes the least common multiple (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::lcm(4, 6), 12);
    /// assert_eq!(Const::lcm(5, 7), 35);
    /// ```
    #[inline]
    pub const fn lcm(a: u64, b: u64) -> u64 {
        a / Const::gcd(a, b) * b
    }

    /// Checks if a value is even (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert!(Const::is_even(4));
    /// assert!(!Const::is_even(5));
    /// ```
    #[inline]
    pub const fn is_even(n: u64) -> bool {
        n % 2 == 0
    }

    /// Checks if a value is odd (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert!(Const::is_odd(5));
    /// assert!(!Const::is_odd(4));
    /// ```
    #[inline]
    pub const fn is_odd(n: u64) -> bool {
        n % 2 == 1
    }

    /// Counts leading zeros in a value (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::leading_zeros(0u32), 32);
    /// assert_eq!(Const::leading_zeros(1u32), 31);
    /// assert_eq!(Const::leading_zeros(0x8000_0000u32), 0);
    /// ```
    #[inline]
    pub const fn leading_zeros(n: u32) -> u32 {
        let mut count = 0u32;
        let mut value = n;
        while value != 0 && (value & 0x8000_0000) == 0 {
            count += 1;
            value <<= 1;
        }
        if n == 0 {
            32
        } else {
            count
        }
    }

    /// Counts trailing zeros in a value (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::trailing_zeros(0u32), 32);
    /// assert_eq!(Const::trailing_zeros(1u32), 0);
    /// assert_eq!(Const::trailing_zeros(2u32), 1);
    /// ```
    #[inline]
    pub const fn trailing_zeros(n: u32) -> u32 {
        let mut count = 0u32;
        let mut value = n;
        while value != 0 && (value & 1) == 0 {
            count += 1;
            value >>= 1;
        }
        if n == 0 {
            32
        } else {
            count
        }
    }

    /// Counts the number of set bits (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::popcount(0u32), 0);
    /// assert_eq!(Const::popcount(1u32), 1);
    /// assert_eq!(Const::popcount(0xFFu32), 8);
    /// ```
    #[inline]
    pub const fn popcount(mut n: u32) -> u32 {
        let mut count = 0u32;
        while n != 0 {
            count += n & 1;
            n >>= 1;
        }
        count
    }

    /// Reverses the bytes of a value (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::swap_bytes(0x12345678u32), 0x78563412);
    /// ```
    #[inline]
    pub const fn swap_bytes(n: u32) -> u32 {
        ((n & 0x000000FF) << 24)
            | ((n & 0x0000FF00) << 8)
            | ((n & 0x00FF0000) >> 8)
            | ((n & 0xFF000000) >> 24)
    }

    /// Rotates bits left (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::rotate_left(0x80000001u32, 1), 0x00000003);
    /// ```
    #[inline]
    pub const fn rotate_left(n: u32, shift: u32) -> u32 {
        let shift = shift % 32;
        (n << shift) | (n >> (32 - shift))
    }

    /// Rotates bits right (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::rotate_right(0x80000001u32, 1), 0xC0000000);
    /// ```
    #[inline]
    pub const fn rotate_right(n: u32, shift: u32) -> u32 {
        let shift = shift % 32;
        (n >> shift) | (n << (32 - shift))
    }

    /// Checks if a value is a power of two (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert!(Const::is_power_of_two(1));
    /// assert!(Const::is_power_of_two(2));
    /// assert!(Const::is_power_of_two(16));
    /// assert!(!Const::is_power_of_two(0));
    /// assert!(!Const::is_power_of_two(3));
    /// ```
    #[inline]
    pub const fn is_power_of_two(n: u64) -> bool {
        n != 0 && (n & (n - 1)) == 0
    }

    /// Returns the next power of two (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::next_power_of_two(5), 8);
    /// assert_eq!(Const::next_power_of_two(16), 16);
    /// assert_eq!(Const::next_power_of_two(0), 1);
    /// ```
    #[inline]
    pub const fn next_power_of_two(n: u64) -> u64 {
        if n == 0 {
            return 1;
        }
        let mut n = n - 1;
        n |= n >> 1;
        n |= n >> 2;
        n |= n >> 4;
        n |= n >> 8;
        n |= n >> 16;
        n |= n >> 32;
        n + 1
    }

    /// Integer square root (floor) (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::isqrt(0), 0);
    /// assert_eq!(Const::isqrt(1), 1);
    /// assert_eq!(Const::isqrt(4), 2);
    /// assert_eq!(Const::isqrt(8), 2);
    /// assert_eq!(Const::isqrt(9), 3);
    /// ```
    #[inline]
    pub const fn isqrt(n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let mut x = n;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }
        x
    }

    /// Log base 2 (floor) (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::log2(1), 0);
    /// assert_eq!(Const::log2(2), 1);
    /// assert_eq!(Const::log2(8), 3);
    /// assert_eq!(Const::log2(16), 4);
    /// ```
    #[inline]
    pub const fn log2(n: u64) -> u64 {
        64 - Const::leading_zeros(n as u32) as u64 - 1
    }

    /// Saturating addition (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::saturating_add(u32::MAX, 1), u32::MAX);
    /// assert_eq!(Const::saturating_add(5, 3), 8);
    /// ```
    #[inline]
    pub const fn saturating_add(a: u32, b: u32) -> u32 {
        match a.checked_add(b) {
            Some(sum) => sum,
            None => u32::MAX,
        }
    }

    /// Saturating subtraction (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::saturating_sub(0u32, 1), 0);
    /// assert_eq!(Const::saturating_sub(5, 3), 2);
    /// ```
    #[inline]
    pub const fn saturating_sub(a: u32, b: u32) -> u32 {
        match a.checked_sub(b) {
            Some(diff) => diff,
            None => 0,
        }
    }

    /// Saturating multiplication (const fn).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_meta::Const;
    ///
    /// assert_eq!(Const::saturating_mul(u32::MAX, 2), u32::MAX);
    /// assert_eq!(Const::saturating_mul(5, 3), 15);
    /// ```
    #[inline]
    pub const fn saturating_mul(a: u32, b: u32) -> u32 {
        match a.checked_mul(b) {
            Some(prod) => prod,
            None => u32::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_min() {
        assert_eq!(Const::min(1, 2), 1);
        assert_eq!(Const::min(2, 1), 1);
        assert_eq!(Const::min(5, 5), 5);
    }

    #[test]
    fn test_const_max() {
        assert_eq!(Const::max(1, 2), 2);
        assert_eq!(Const::max(2, 1), 2);
        assert_eq!(Const::max(5, 5), 5);
    }

    #[test]
    fn test_const_clamp() {
        assert_eq!(Const::clamp(5, 0, 10), 5);
        assert_eq!(Const::clamp(-5, 0, 10), 0);
        assert_eq!(Const::clamp(15, 0, 10), 10);
    }

    #[test]
    fn test_const_abs() {
        assert_eq!(Const::abs(-5), 5);
        assert_eq!(Const::abs(5), 5);
        assert_eq!(Const::abs(0), 0);
    }

    #[test]
    fn test_const_in_range() {
        assert!(Const::in_range(5, 0, 10));
        assert!(Const::in_range(0, 0, 10));
        assert!(Const::in_range(10, 0, 10));
        assert!(!Const::in_range(15, 0, 10));
        assert!(!Const::in_range(-5, 0, 10));
    }

    #[test]
    fn test_const_div_ceil() {
        assert_eq!(Const::div_ceil(10, 3), 4);
        assert_eq!(Const::div_ceil(9, 3), 3);
        assert_eq!(Const::div_ceil(1, 3), 1);
    }

    #[test]
    fn test_const_round_up() {
        assert_eq!(Const::round_up(10, 3), 12);
        assert_eq!(Const::round_up(9, 3), 9);
        assert_eq!(Const::round_up(1, 3), 3);
    }

    #[test]
    fn test_const_min_max_edge_cases() {
        assert_eq!(Const::min(i8::MIN, i8::MAX), i8::MIN);
        assert_eq!(Const::max(i8::MIN, i8::MAX), i8::MAX);
    }

    #[test]
    fn test_signed_abs_edge_cases() {
        // For MIN_VALUE, we saturate at MAX_VALUE to prevent overflow
        // For example, i32::MIN = -2147483648, but i32::MAX = 2147483647
        // Negating MIN_VALUE would overflow, so we return MAX_VALUE instead
        assert_eq!(Const::abs(i32::MIN), i32::MAX);
        assert_eq!(Const::abs(i8::MIN), i8::MAX);
        assert_eq!(Const::abs(i16::MIN), i16::MAX);
        assert_eq!(Const::abs(0), 0);
        assert_eq!(Const::abs(-1), 1);
        assert_eq!(Const::abs(1), 1);
    }

    #[test]
    fn test_const_sqr() {
        assert_eq!(Const::sqr(5), 25);
        assert_eq!(Const::sqr(-3), 9);
        assert_eq!(Const::sqr(0), 0);
    }

    #[test]
    fn test_const_pow() {
        assert_eq!(Const::pow(2u32, 3), 8);
        assert_eq!(Const::pow(5u32, 0), 1);
        assert_eq!(Const::pow(3u32, 4), 81);
    }

    #[test]
    fn test_const_gcd() {
        assert_eq!(Const::gcd(48, 18), 6);
        assert_eq!(Const::gcd(17, 5), 1);
        assert_eq!(Const::gcd(0, 5), 5);
    }

    #[test]
    fn test_const_lcm() {
        assert_eq!(Const::lcm(4, 6), 12);
        assert_eq!(Const::lcm(5, 7), 35);
        assert_eq!(Const::lcm(3, 9), 9);
    }

    #[test]
    fn test_const_is_even() {
        assert!(Const::is_even(4));
        assert!(Const::is_even(0));
        assert!(!Const::is_even(5));
    }

    #[test]
    fn test_const_is_odd() {
        assert!(Const::is_odd(5));
        assert!(Const::is_odd(1));
        assert!(!Const::is_odd(4));
    }

    #[test]
    fn test_const_leading_zeros() {
        assert_eq!(Const::leading_zeros(0u32), 32);
        assert_eq!(Const::leading_zeros(1u32), 31);
        assert_eq!(Const::leading_zeros(0x8000_0000u32), 0);
        assert_eq!(Const::leading_zeros(0xFFu32), 24);
    }

    #[test]
    fn test_const_trailing_zeros() {
        assert_eq!(Const::trailing_zeros(0u32), 32);
        assert_eq!(Const::trailing_zeros(1u32), 0);
        assert_eq!(Const::trailing_zeros(2u32), 1);
        assert_eq!(Const::trailing_zeros(8u32), 3);
    }

    #[test]
    fn test_const_popcount() {
        assert_eq!(Const::popcount(0u32), 0);
        assert_eq!(Const::popcount(1u32), 1);
        assert_eq!(Const::popcount(0xFFu32), 8);
        assert_eq!(Const::popcount(0x1010u32), 2);
    }

    #[test]
    fn test_const_swap_bytes() {
        assert_eq!(Const::swap_bytes(0x12345678u32), 0x78563412);
        assert_eq!(Const::swap_bytes(0x11223344u32), 0x44332211);
    }

    #[test]
    fn test_const_rotate_left() {
        assert_eq!(Const::rotate_left(0x80000001u32, 1), 0x00000003);
        assert_eq!(Const::rotate_left(0x12345678u32, 4), 0x23456781);
    }

    #[test]
    fn test_const_rotate_right() {
        assert_eq!(Const::rotate_right(0x80000001u32, 1), 0xC0000000);
        assert_eq!(Const::rotate_right(0x12345678u32, 4), 0x81234567);
    }

    #[test]
    fn test_const_is_power_of_two() {
        assert!(Const::is_power_of_two(1));
        assert!(Const::is_power_of_two(2));
        assert!(Const::is_power_of_two(16));
        assert!(!Const::is_power_of_two(0));
        assert!(!Const::is_power_of_two(3));
        assert!(!Const::is_power_of_two(15));
    }

    #[test]
    fn test_const_next_power_of_two() {
        assert_eq!(Const::next_power_of_two(0), 1);
        assert_eq!(Const::next_power_of_two(1), 1);
        assert_eq!(Const::next_power_of_two(5), 8);
        assert_eq!(Const::next_power_of_two(16), 16);
        assert_eq!(Const::next_power_of_two(17), 32);
    }

    #[test]
    fn test_const_isqrt() {
        assert_eq!(Const::isqrt(0), 0);
        assert_eq!(Const::isqrt(1), 1);
        assert_eq!(Const::isqrt(4), 2);
        assert_eq!(Const::isqrt(8), 2);
        assert_eq!(Const::isqrt(9), 3);
        assert_eq!(Const::isqrt(15), 3);
        assert_eq!(Const::isqrt(16), 4);
    }

    #[test]
    fn test_const_log2() {
        assert_eq!(Const::log2(1), 0);
        assert_eq!(Const::log2(2), 1);
        assert_eq!(Const::log2(4), 2);
        assert_eq!(Const::log2(8), 3);
        assert_eq!(Const::log2(16), 4);
    }

    #[test]
    fn test_const_saturating_add() {
        assert_eq!(Const::saturating_add(u32::MAX, 1), u32::MAX);
        assert_eq!(Const::saturating_add(5, 3), 8);
        assert_eq!(Const::saturating_add(5, u32::MAX), u32::MAX);
    }

    #[test]
    fn test_const_saturating_sub() {
        assert_eq!(Const::saturating_sub(0u32, 1), 0);
        assert_eq!(Const::saturating_sub(5, 3), 2);
        assert_eq!(Const::saturating_sub(3, 5), 0);
    }

    #[test]
    fn test_const_saturating_mul() {
        assert_eq!(Const::saturating_mul(u32::MAX, 2), u32::MAX);
        assert_eq!(Const::saturating_mul(5, 3), 15);
        assert_eq!(Const::saturating_mul(5, u32::MAX), u32::MAX);
    }
}
