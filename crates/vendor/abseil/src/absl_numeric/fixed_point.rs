//! Fixed-point arithmetic.

use core::ops::{Add, Div, Mul, Neg, Sub};

/// A fixed-point number with specified fractional bits.
///
/// Represents a signed fixed-point number where the lower F bits are fractional.
///
/// # Type Parameters
///
/// * `F` - Number of fractional bits. Must be < 63 to ensure safe arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed<const F: u32, i64> {
    value: i64,
}

impl<const F: u32, i64> Fixed<F, i64> {
    // Validate F is < 63 to prevent overflow in shift operations.
    // This is checked at compile time via const assert pattern.
    const ASSERT_F_VALID: () = assert!(F < 63, "F must be < 63 for safe fixed-point arithmetic");
    const ONE: i64 = 1i64 << F;

    /// Creates a new fixed-point number from an integer value.
    ///
    /// # Panics
    ///
    /// Panics if the value overflows when shifted by F bits.
    #[inline]
    pub const fn from_int(value: i64) -> Self {
        // Use checked_shl to prevent overflow. Panic with clear message.
        // We need to evaluate the const assert to ensure it runs.
        let _ = Self::ASSERT_F_VALID;
        match value.checked_shl(F) {
            Some(v) => Self { value: v },
            None => panic!("from_int overflow: value {} << {} exceeds i64 range", value, F),
        }
    }

    /// Creates a new fixed-point number from the raw internal value.
    #[inline]
    pub const fn from_raw(value: i64) -> Self {
        Self { value }
    }

    /// Returns the integer part (truncates towards zero).
    #[inline]
    pub const fn trunc(self) -> i64 {
        // SAFETY: F is validated to be < 63, so shifting by F is safe.
        // Use arithmetic shift for signed integers.
        self.value >> (F as u32)
    }

    /// Returns the raw internal value.
    #[inline]
    pub const fn raw(self) -> i64 {
        self.value
    }

    /// Converts to f64 (approximate).
    #[inline]
    pub const fn to_f64(self) -> f64 {
        self.value as f64 / (Self::ONE as f64)
    }

    /// Adds two fixed-point numbers.
    #[inline]
    pub const fn add(self, other: Self) -> Self {
        Self { value: self.value + other.value }
    }

    /// Subtracts two fixed-point numbers.
    #[inline]
    pub const fn sub(self, other: Self) -> Self {
        Self { value: self.value - other.value }
    }

    /// Multiplies two fixed-point numbers.
    #[inline]
    pub const fn mul(self, other: Self) -> Self {
        Self {
            value: (self.value * other.value) >> F
        }
    }

    /// Divides two fixed-point numbers.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The divisor is zero
    /// - The numerator overflows when shifted by F bits
    #[inline]
    pub fn div(self, other: Self) -> Self {
        if other.value == 0 {
            panic!("Fixed point division by zero");
        }
        // Use checked_shl to prevent overflow
        match self.value.checked_shl(F as u32) {
            Some(shifted) => Self { value: shifted / other.value },
            None => panic!("div overflow: numerator << {} exceeds i64 range", F),
        }
    }

    /// Computes the absolute value.
    #[inline]
    pub const fn abs(self) -> Self {
        Self {
            value: if self.value < 0 { -self.value } else { self.value }
        }
    }

    /// Computes the square root using Newton's method.
    ///
    /// # Panics
    ///
    /// Panics if F + 32 >= 64 (which would cause overflow in shift).
    pub fn sqrt(self) -> Self {
        if self.value <= 0 {
            return Self::from_int(0);
        }

        // Check that F + 32 < 64 to prevent overflow in shift below
        if F + 32 >= 64 {
            panic!("sqrt: F must be < 32 for this implementation");
        }

        let mut x = self.value;
        let mut y = (x + (Self::ONE << 32)) / 2;

        const ITERATIONS: u32 = 10;
        for _ in 0..ITERATIONS {
            let new_y = (x / y + y) / 2;
            if new_y == y {
                break;
            }
            y = new_y;
        }

        Self::from_raw(y)
    }

    /// Negates the fixed-point number.
    #[inline]
    pub const fn neg(self) -> Self {
        Self { value: -self.value }
    }
}

impl<const F: u32, i64> Add for Fixed<F, i64> {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        self.add(other)
    }
}

impl<const F: u32, i64> Sub for Fixed<F, i64> {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        self.sub(other)
    }
}

impl<const F: u32, i64> Mul for Fixed<F, i64> {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self::Output {
        self.mul(other)
    }
}

impl<const F: u32, i64> Div for Fixed<F, i64> {
    type Output = Self;

    #[inline]
    fn div(self, other: Self) -> Self::Output {
        self.div(other)
    }
}

impl<const F: u32, i64> Neg for Fixed<F, i64> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        self.neg()
    }
}
