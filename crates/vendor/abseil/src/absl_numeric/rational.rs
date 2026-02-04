//! Rational number (fraction) type.

use alloc::string::String;
use core::cmp::{Ord, Ordering, PartialOrd};
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

use super::division::gcd;

/// A rational number (fraction) represented as numerator/denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rational {
    numerator: i64,
    denominator: i64,
}

impl Rational {
    /// Creates a new rational number.
    ///
    /// # Panics
    ///
    /// Panics if denominator is zero.
    #[inline]
    pub fn new(numerator: i64, denominator: i64) -> Self {
        assert!(denominator != 0, "Denominator cannot be zero");
        let gcd_value = gcd(numerator.abs(), denominator.abs());
        Rational {
            numerator: numerator / gcd_value,
            denominator: denominator / gcd_value,
        }
    }

    /// Returns the numerator.
    #[inline]
    pub const fn numerator(&self) -> i64 {
        self.numerator
    }

    /// Returns the denominator.
    #[inline]
    pub const fn denominator(&self) -> i64 {
        self.denominator
    }

    /// Creates a rational from an integer.
    #[inline]
    pub const fn from_int(value: i64) -> Self {
        Rational {
            numerator: value,
            denominator: 1,
        }
    }

    /// Converts to f64.
    #[inline]
    pub fn to_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// Adds two rational numbers.
    ///
    /// # Panics
    ///
    /// Panics if the result would overflow.
    #[inline]
    pub fn add(&self, other: &Rational) -> Rational {
        self.checked_add(other)
            .expect("Rational::add: arithmetic overflow")
    }

    /// Checked addition that returns None on overflow.
    #[inline]
    pub fn checked_add(&self, other: &Rational) -> Option<Rational> {
        // a/b + c/d = (ad + bc) / bd
        let ad = self.numerator.checked_mul(other.denominator)?;
        let bc = other.numerator.checked_mul(self.denominator)?;
        let numerator = ad.checked_add(bc)?;
        let denominator = self.denominator.checked_mul(other.denominator)?;
        Some(Rational::new(numerator, denominator))
    }

    /// Subtracts two rational numbers.
    ///
    /// # Panics
    ///
    /// Panics if the result would overflow.
    #[inline]
    pub fn sub(&self, other: &Rational) -> Rational {
        self.checked_sub(other)
            .expect("Rational::sub: arithmetic overflow")
    }

    /// Checked subtraction that returns None on overflow.
    #[inline]
    pub fn checked_sub(&self, other: &Rational) -> Option<Rational> {
        // a/b - c/d = (ad - bc) / bd
        let ad = self.numerator.checked_mul(other.denominator)?;
        let bc = other.numerator.checked_mul(self.denominator)?;
        let numerator = ad.checked_sub(bc)?;
        let denominator = self.denominator.checked_mul(other.denominator)?;
        Some(Rational::new(numerator, denominator))
    }

    /// Multiplies two rational numbers.
    ///
    /// # Panics
    ///
    /// Panics if the result would overflow.
    #[inline]
    pub fn mul(&self, other: &Rational) -> Rational {
        self.checked_mul(other)
            .expect("Rational::mul: arithmetic overflow")
    }

    /// Checked multiplication that returns None on overflow.
    #[inline]
    pub fn checked_mul(&self, other: &Rational) -> Option<Rational> {
        // a/b * c/d = ac / bd
        let numerator = self.numerator.checked_mul(other.numerator)?;
        let denominator = self.denominator.checked_mul(other.denominator)?;
        Some(Rational::new(numerator, denominator))
    }

    /// Divides two rational numbers.
    ///
    /// # Panics
    ///
    /// Panics if the result would overflow or if other is zero.
    #[inline]
    pub fn div(&self, other: &Rational) -> Rational {
        self.checked_div(other)
            .expect("Rational::div: arithmetic overflow or division by zero")
    }

    /// Checked division that returns None on overflow.
    #[inline]
    pub fn checked_div(&self, other: &Rational) -> Option<Rational> {
        // a/b / c/d = ad / bc
        if other.numerator == 0 {
            return None; // Division by zero
        }
        let numerator = self.numerator.checked_mul(other.denominator)?;
        let denominator = self.denominator.checked_mul(other.numerator)?;
        Some(Rational::new(numerator, denominator))
    }

    /// Negates the rational number.
    #[inline]
    pub fn neg(&self) -> Rational {
        Rational::new(-self.numerator, self.denominator)
    }

    /// Computes the absolute value.
    #[inline]
    pub fn abs(&self) -> Rational {
        Rational::new(self.numerator.abs(), self.denominator)
    }

    /// Reduces the fraction to lowest terms.
    #[inline]
    pub fn reduce(&self) -> Rational {
        let g = gcd(self.numerator.abs(), self.denominator);
        if g == 1 {
            return *self;
        }
        Rational::new(self.numerator / g, self.denominator / g)
    }

    /// Reciprocal of the rational number.
    #[inline]
    pub fn reciprocal(&self) -> Rational {
        Rational::new(self.denominator, self.numerator)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl Add for Rational {
    type Output = Rational;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        self.add(&other)
    }
}

impl Sub for Rational {
    type Output = Rational;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        self.sub(&other)
    }
}

impl Mul for Rational {
    type Output = Rational;

    #[inline]
    fn mul(self, other: Self) -> Self::Output {
        self.mul(&other)
    }
}

impl Div for Rational {
    type Output = Rational;

    #[inline]
    fn div(self, other: Self) -> Self::Output {
        self.div(&other)
    }
}

impl Neg for Rational {
    type Output = Rational;

    #[inline]
    fn neg(self) -> Self::Output {
        self.neg()
    }
}

impl PartialOrd for Rational {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare a/b vs c/d by comparing ad vs bc
        // Use floating-point comparison to avoid overflow in multiplication
        // This may lose precision for very large values, but avoids panicking
        let self_val = self.numerator as f64 / self.denominator as f64;
        let other_val = other.numerator as f64 / other.denominator as f64;

        // For values that can be exactly represented in f64, use the direct comparison
        // For edge cases near f64 precision limits, fall back to partial comparison
        if self_val < other_val {
            Ordering::Less
        } else if self_val > other_val {
            Ordering::Greater
        } else {
            // Values are equal in f64 representation - try exact comparison if safe
            match (
                self.numerator.checked_mul(other.denominator),
                other.numerator.checked_mul(self.denominator),
            ) {
                (Some(lhs), Some(rhs)) => lhs.cmp(&rhs),
                _ => {
                    // Overflow would occur - use the f64 comparison result
                    Ordering::Equal
                }
            }
        }
    }
}
