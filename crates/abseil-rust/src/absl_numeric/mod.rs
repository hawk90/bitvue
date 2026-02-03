//! Numeric utilities.
//!
//! This module provides numeric utilities similar to Abseil's `absl/numeric` directory.
//! Rust's standard library already provides many numeric utilities through `core::num`
//! and `core::intrinsics`, but this module provides additional compatibility helpers
//! and Abseil-specific utilities.
//!
//! # Overview
//!
//! Numeric utilities provide common numeric operations and helper functions that enhance
//! Rust's built-in numeric system. These include:
//!
//! - Safe arithmetic operations with overflow checking
//! - Bit manipulation utilities
//! - Numeric representation conversions
//! - Comparison helpers (clamp, min, max)
//! - Rounding operations
//! - Safe integer division
//!
//! # Modules
//!
//! - [`int128`] - 128-bit integer type with additional utilities
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_numeric::{safe_add, clamp, is_power_of_two};
//!
//! // Safe arithmetic that returns None on overflow
//! assert_eq!(safe_add(100i32, 200), Some(300));
//! assert_eq!(safe_add(i32::MAX, 1), None);
//!
//! // Clamp a value to a range
//! assert_eq!(clamp(5, 0, 10), 5);
//! assert_eq!(clamp(-5, 0, 10), 0);
//! assert_eq!(clamp(15, 0, 10), 10);
//!
//! // Check if a number is a power of two
//! assert!(is_power_of_two(16));
//! assert!(!is_power_of_two(15));
//! ```

// Submodules
pub mod bits;
pub mod casting;
pub mod comparison;
pub mod division;
pub mod error;
pub mod fixed_point;
pub mod float_utils;
pub mod ieee754;
pub mod interpolation;
pub mod int128;
pub mod number_theory;
pub mod rational;
pub mod rounding;
pub mod safe_arithmetic;
pub mod saturating;
pub mod math;
pub mod special_functions;

// Re-exports from int128
pub use int128::{int128, uint128};

// Re-exports from error
pub use error::NumericError;

// Re-exports from safe_arithmetic
pub use safe_arithmetic::{checked_add, checked_div, checked_mul, checked_sub, safe_add, safe_div, safe_mul, safe_rem, safe_sub};

// Re-exports from saturating
pub use saturating::{saturating_add, saturating_mul, saturating_sub};

// Re-exports from comparison
pub use comparison::{clamp, max, median, min};

// Re-exports from bits
pub use bits::{
    count_leading_zeros, count_trailing_zeros, is_power_of_two, popcount, reverse_bits,
    rotate_left, rotate_right, round_down_to_power_of_two, round_up_to_power_of_two, swap_bytes,
};

// Re-exports from division
pub use division::{ceil_div, div_rem, floor_div, gcd, lcm};

// Re-exports from rounding
pub use rounding::{
    round_away_from_zero, round_down, round_half_to_even, round_towards_zero, round_to_places,
    round_up,
};

// Re-exports from float_utils
pub use float_utils::{copy_sign, is_finite, is_infinite, is_nan, next_after, sign};

// Re-exports from fixed_point
pub use fixed_point::Fixed;

// Re-exports from rational
pub use rational::Rational;

// Re-exports from interpolation
pub use interpolation::{inverse_lerp, lerp, lerp_int, remap, remap_clamp, smoothstep, smootherstep};

// Re-exports from number_theory
pub use number_theory::{binomial, factorial, fibonacci, is_prime, mod_inverse, mod_pow};

// Re-exports from casting
pub use casting::{cast_clamp, in_range, safe_cast};

// Re-exports from ieee754
pub use ieee754::{
    bits_to_double, bits_to_float, construct_f32, construct_f64, double_eq_bits, double_to_bits,
    extract_exponent_f32, extract_exponent_f64, extract_mantissa_f32, extract_mantissa_f64,
    float_eq_bits, float_to_bits,
};

// Re-exports from math
pub use math::{
    catalan, digit_sum, divisor_count, divisor_sum, gcd, is_abundant, is_palindrome, is_perfect,
    is_perfect_power, is_triangular, isqrt, lcm, log10, log2, lucas, pow, reverse_digits,
    triangular,
};

// Re-exports from special_functions
pub use special_functions::{
    are_coprime, binomial_multiplicative, combinations, euler_totient, fibonacci_fast,
    gcd_multiple, is_look_and_say, is_prime_extended, lcm_multiple, look_and_say, next_prime,
    permutations, prime_factors,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-5, 0, 10), 0);
        assert_eq!(clamp(15, 0, 10), 10);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(min(5, 10), 5);
        assert_eq!(min(10, 5), 5);
        assert_eq!(max(5, 10), 10);
        assert_eq!(max(10, 5), 10);
    }

    #[test]
    fn test_median() {
        assert_eq!(median(1, 5, 3), 3);
        assert_eq!(median(5, 1, 3), 3);
        assert_eq!(median(3, 5, 1), 3);
        assert_eq!(median(10, 10, 10), 10);
    }

    #[test]
    fn test_div_rem() {
        let (q, r) = div_rem(17i32, 5);
        assert_eq!(q, 3);
        assert_eq!(r, 2);

        let (q, r) = div_rem(100i32, 10);
        assert_eq!(q, 10);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(48u32, 18u32), 6);
        assert_eq!(gcd(17u32, 5u32), 1);
        assert_eq!(gcd(100u32, 100u32), 100);
        assert_eq!(gcd(0u32, 5u32), 5);
    }

    #[test]
    fn test_lcm() {
        assert_eq!(lcm(4u32, 6u32), 12);
        assert_eq!(lcm(5u32, 7u32), 35);
        assert_eq!(lcm(6u32, 8u32), 24);
    }

    #[test]
    fn test_round_functions() {
        assert_eq!(round_half_to_even(2.5), 2.0);
        assert_eq!(round_half_to_even(3.5), 4.0);
        assert_eq!(round_half_to_even(2.4), 2.0);
        assert_eq!(round_half_to_even(2.6), 3.0);

        assert_eq!(round_up(2.1), 3.0);
        assert_eq!(round_up(2.0), 2.0);
        assert_eq!(round_up(-2.1), -2.0);

        assert_eq!(round_down(2.9), 2.0);
        assert_eq!(round_down(2.0), 2.0);
        assert_eq!(round_down(-2.9), -3.0);

        assert_eq!(round_towards_zero(2.9), 2.0);
        assert_eq!(round_towards_zero(-2.9), -2.0);

        assert_eq!(round_away_from_zero(2.1), 3.0);
        assert_eq!(round_away_from_zero(-2.1), -3.0);
    }

    #[test]
    fn test_round_to_places() {
        assert_eq!(round_to_places(2.456, 2), 2.46);
        assert_eq!(round_to_places(2.454, 2), 2.45);
        assert_eq!(round_to_places(2.5, 0), 3.0);
        assert_eq!(round_to_places(2.4, 0), 2.0);
    }

    #[test]
    fn test_float_utilities() {
        assert!(is_nan(f32::NAN));
        assert!(!is_nan(0.0_f32));
        assert!(!is_nan(f32::INFINITY));

        assert!(is_finite(0.0_f32));
        assert!(is_finite(1.0_f32));
        assert!(!is_finite(f32::NAN));
        assert!(!is_finite(f32::INFINITY));

        assert!(is_infinite(f32::INFINITY));
        assert!(is_infinite(f32::NEG_INFINITY));
        assert!(!is_infinite(0.0_f32));
        assert!(!is_infinite(f32::NAN));
    }

    #[test]
    fn test_sign() {
        assert_eq!(sign(-5.0), -1.0);
        assert_eq!(sign(0.0), 0.0);
        assert_eq!(sign(5.0), 1.0);
    }

    #[test]
    fn test_copy_sign() {
        assert_eq!(copy_sign(5.0, -1.0), -5.0);
        assert_eq!(copy_sign(-5.0, 1.0), 5.0);
        assert_eq!(copy_sign(5.0, 1.0), 5.0);
        assert_eq!(copy_sign(-5.0, -1.0), -5.0);
    }

    #[test]
    fn test_next_after() {
        assert!(next_after(1.0_f32, 2.0) > 1.0);
        assert!(next_after(1.0_f32, 0.0) < 1.0);
        assert!(next_after(f32::MAX, f32::INFINITY).is_infinite());
    }

    #[test]
    fn test_fixed_point() {
        const F16: Fixed<16, i64> = Fixed::from_int(5);

        assert_eq!(F16.trunc(), 5);
        assert_eq!(Fixed::from_int(2).to_f64(), 2.0);

        let sum = Fixed::from_int(2).add(Fixed::from_int(3));
        assert_eq!(sum.trunc(), 5);

        let product = Fixed::from_int(3).mul(Fixed::from_int(4));
        assert_eq!(product.trunc(), 12);
    }

    #[test]
    fn test_fixed_point_ops() {
        type F16 = Fixed<16, i64>;

        let a = F16::from_int(10);
        let b = F16::from_int(3);

        let sum = a.add(b);
        assert_eq!(sum.trunc(), 13);

        let diff = a.sub(b);
        assert_eq!(diff.trunc(), 7);

        let prod = a.mul(b);
        assert_eq!(prod.trunc(), 30);

        let quot = a.div(b);
        assert_eq!(quot.trunc(), 3);

        let neg = a.neg();
        assert_eq!(neg.trunc(), -10);

        let abs = neg.abs();
        assert_eq!(abs.trunc(), 10);
    }

    #[test]
    fn test_rational() {
        let r = Rational::new(3, 4);
        assert_eq!(r.numerator(), 3);
        assert_eq!(r.denominator(), 4);

        let r2 = Rational::new(6, 8);
        assert_eq!(r2.numerator(), 3);
        assert_eq!(r2.denominator(), 4);
    }

    #[test]
    fn test_rational_arithmetic() {
        let a = Rational::new(1, 2);
        let b = Rational::new(1, 4);

        let sum = a.add(&b);
        assert_eq!(sum.numerator(), 3);
        assert_eq!(sum.denominator(), 4);

        let prod = a.mul(&b);
        assert_eq!(prod.numerator(), 1);
        assert_eq!(prod.denominator(), 8);

        let quotient = a.div(&b);
        assert_eq!(quotient.numerator(), 2);
        assert_eq!(quotient.denominator(), 1);
    }

    #[test]
    fn test_rational_reduce() {
        let r = Rational::new(6, 8);
        let reduced = r.reduce();
        assert_eq!(reduced.numerator(), 3);
        assert_eq!(reduced.denominator(), 4);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(0.0, 10.0, 0.25), 2.5);
        assert_eq!(lerp(0.0, 10.0, 0.75), 7.5);
    }

    #[test]
    fn test_lerp_int() {
        assert_eq!(lerp_int(0, 10, 5, 10), 5);
        assert_eq!(lerp_int(0, 100, 25, 100), 25);
    }

    #[test]
    fn test_smoothstep() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(0.5), 0.5);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!(smoothstep(0.25) < smoothstep(0.5));
    }

    #[test]
    fn test_inverse_lerp() {
        assert_eq!(inverse_lerp(0.0, 10.0, 5.0), 0.5);
        assert_eq!(inverse_lerp(0.0, 10.0, 2.5), 0.25);
        assert_eq!(inverse_lerp(0.0, 10.0, 10.0), 1.0);
    }

    #[test]
    fn test_remap() {
        assert_eq!(remap(5.0, 0.0, 10.0, 0.0, 100.0), 50.0);
        assert_eq!(remap(0.5, 0.0, 1.0, -1.0, 1.0), 0.0);
    }

    #[test]
    fn test_remap_clamp() {
        assert_eq!(remap_clamp(15.0, 0.0, 10.0, 0.0, 100.0), 100.0);
        assert_eq!(remap_clamp(-5.0, 0.0, 10.0, 0.0, 100.0), 0.0);
    }

    #[test]
    fn test_is_prime() {
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(is_prime(5));
        assert!(is_prime(7));
        assert!(!is_prime(4));
        assert!(!is_prime(1));
        assert!(!is_prime(0));
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), Some(1));
        assert_eq!(factorial(1), Some(1));
        assert_eq!(factorial(5), Some(120));
        assert_eq!(factorial(20), None);
    }

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(0), Some(0));
        assert_eq!(fibonacci(1), Some(1));
        assert_eq!(fibonacci(2), Some(1));
        assert_eq!(fibonacci(10), Some(55));
    }

    #[test]
    fn test_binomial() {
        assert_eq!(binomial(5, 2), Some(10));
        assert_eq!(binomial(10, 3), Some(120));
        assert_eq!(binomial(10, 0), Some(1));
        assert_eq!(binomial(10, 10), Some(1));
        assert_eq!(binomial(10, 11), Some(0));
    }

    #[test]
    fn test_mod_inverse() {
        assert_eq!(mod_inverse(3, 7), Some(5));
        assert_eq!(mod_inverse(2, 6), None);
    }

    #[test]
    fn test_mod_pow() {
        assert_eq!(mod_pow(2, 10, 1000), Some(24));
        assert_eq!(mod_pow(3, 4, 100), Some(81));
        assert_eq!(mod_pow(2, 0, 100), Some(1));
    }

    #[test]
    fn test_safe_cast() {
        assert_eq!(safe_cast::<i32, i64>(5i64), Some(5i32));
        assert_eq!(safe_cast::<i32, i64>(i64::MAX), None);
        assert_eq!(safe_cast::<u32, i32>(-1i32), None);
    }

    #[test]
    fn test_cast_clamp() {
        assert_eq!(cast_clamp::<i32, i8>(100), 127);
        assert_eq!(cast_clamp::<i32, i8>(-100), -128);
        assert_eq!(cast_clamp::<i32, i8>(50), 50);
    }

    #[test]
    fn test_in_range() {
        assert!(in_range(5, 0, 10));
        assert!(!in_range(15, 0, 10));
        assert!(in_range(0, 0, 10));
        assert!(in_range(10, 0, 10));
    }

    #[test]
    fn test_float_bits() {
        let x = 1.0_f32;
        let bits = float_to_bits(x);
        assert_eq!(bits_to_float(bits), x);
    }

    #[test]
    fn test_double_bits() {
        let x = 1.0_f64;
        let bits = double_to_bits(x);
        assert_eq!(bits_to_double(bits), x);
    }

    #[test]
    fn test_extract_f32() {
        let x = 12.5_f32;
        let exp = extract_exponent_f32(x);
        let mantissa = extract_mantissa_f32(x);

        assert!(exp > 0 && exp < 255);
        assert!(mantissa <= 0x7FFFFF);
    }

    #[test]
    fn test_construct_f32() {
        let pos = construct_f32(false, 127, 0);
        assert!(pos.is_finite());

        let neg = construct_f32(true, 127, 0);
        assert!(neg.is_finite());
        assert!(neg < 0.0);
    }
}
