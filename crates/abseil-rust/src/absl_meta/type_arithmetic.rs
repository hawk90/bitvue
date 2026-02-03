//! Type-level arithmetic operations.

use super::type_constants::{Int, UInt};

/// Type-level addition.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::{Add, Int};
///
/// type Result = Add::<Int<10>, Int<5>>;
/// assert_eq!(Result::VALUE, 15);
/// ```
pub trait Add<R> {
    /// The sum of the two values.
    type Output;
}

impl<const A: isize, const B: isize> Add<Int<B>> for Int<A> {
    type Output = Int<{ A + B }>;
}

impl<const A: usize, const B: usize> Add<UInt<B>> for UInt<A> {
    type Output = UInt<{ A + B }>;
}

/// Type-level subtraction.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::{Sub, Int};
///
/// type Result = Sub::<Int<10>, Int<5>>;
/// assert_eq!(Result::VALUE, 5);
/// ```
pub trait Sub<R> {
    /// The difference of the two values.
    type Output;
}

impl<const A: isize, const B: isize> Sub<Int<B>> for Int<A> {
    type Output = Int<{ A - B }>;
}

/// Type-level multiplication.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::{Mul, Int};
///
/// type Result = Mul::<Int<3>, Int<4>>;
/// assert_eq!(Result::VALUE, 12);
/// ```
pub trait Mul<R> {
    /// The product of the two values.
    type Output;
}

impl<const A: isize, const B: isize> Mul<Int<B>> for Int<A> {
    type Output = Int<{ A * B }>;
}

impl<const A: usize, const B: usize> Mul<UInt<B>> for UInt<A> {
    type Output = UInt<{ A * B }>;
}

/// Type-level division (for unsigned integers).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::{Div, UInt};
///
/// type Result = Div::<UInt<10>, UInt<2>>;
/// assert_eq!(Result::VALUE, 5);
/// ```
pub trait Div<R> {
    /// The quotient of the two values.
    type Output;
}

impl<const A: usize, const B: usize> Div<UInt<B>> for UInt<A> {
    type Output = UInt<{ A / B }>;
}

/// Type-level modulo (for unsigned integers).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::{Mod, UInt};
///
/// type Result = Mod::<UInt<10>, UInt<3>>;
/// assert_eq!(Result::VALUE, 1);
/// ```
pub trait Mod<R> {
    /// The remainder of the division.
    type Output;
}

impl<const A: usize, const B: usize> Mod<UInt<B>> for UInt<A> {
    type Output = UInt<{ A % B }>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for Add trait
    #[test]
    fn test_add_int() {
        type Result = Add::<Int<10>, Int<5>>;
        assert_eq!(Result::VALUE, 15);
    }

    #[test]
    fn test_add_uint() {
        type Result = Add::<UInt<10>, UInt<5>>;
        assert_eq!(Result::VALUE, 15);
    }

    #[test]
    fn test_add_negative() {
        type Result = Add::<Int<-5>, Int<3>>;
        assert_eq!(Result::VALUE, -2);
    }

    // Tests for Sub trait
    #[test]
    fn test_sub_int() {
        type Result = Sub::<Int<10>, Int<3>>;
        assert_eq!(Result::VALUE, 7);
    }

    #[test]
    fn test_sub_negative() {
        type Result = Sub::<Int<5>, Int<10>>;
        assert_eq!(Result::VALUE, -5);
    }

    // Tests for Mul trait
    #[test]
    fn test_mul_int() {
        type Result = Mul::<Int<3>, Int<4>>;
        assert_eq!(Result::VALUE, 12);
    }

    #[test]
    fn test_mul_uint() {
        type Result = Mul::<UInt<5>, UInt<6>>;
        assert_eq!(Result::VALUE, 30);
    }

    #[test]
    fn test_mul_negative() {
        type Result = Mul::<Int<-3>, Int<4>>;
        assert_eq!(Result::VALUE, -12);
    }

    // Tests for Div trait
    #[test]
    fn test_div_uint() {
        type Result = Div::<UInt<10>, UInt<2>>;
        assert_eq!(Result::VALUE, 5);
    }

    #[test]
    fn test_div_rounding() {
        type Result = Div::<UInt<11>, UInt<2>>;
        assert_eq!(Result::VALUE, 5);
    }

    // Tests for Mod trait
    #[test]
    fn test_mod_uint() {
        type Result = Mod::<UInt<10>, UInt<3>>;
        assert_eq!(Result::VALUE, 1);
    }

    #[test]
    fn test_mod_zero() {
        type Result = Mod::<UInt<12>, UInt<4>>;
        assert_eq!(Result::VALUE, 0);
    }
}
