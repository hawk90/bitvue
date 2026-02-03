//! Advanced mathematical functions.

/// Computes the greatest common divisor using Euclidean algorithm.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::gcd;
///
/// assert_eq!(gcd(48, 18), 6);
/// assert_eq!(gcd(17, 5), 1);
/// assert_eq!(gcd(0, 5), 5);
/// ```
#[inline]
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Computes the least common multiple.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::lcm;
///
/// assert_eq!(lcm(4, 6), 12);
/// assert_eq!(lcm(5, 7), 35);
/// assert_eq!(lcm(3, 9), 9);
/// ```
#[inline]
pub fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        return 0;
    }
    (a / gcd(a, b)) * b
}

/// Computes integer square root (floor of sqrt).
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::isqrt;
///
/// assert_eq!(isqrt(0), 0);
/// assert_eq!(isqrt(1), 1);
/// assert_eq!(isqrt(4), 2);
/// assert_eq!(isqrt(8), 2);
/// assert_eq!(isqrt(9), 3);
/// ```
#[inline]
pub fn isqrt(n: u64) -> u64 {
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

/// Computes base-2 logarithm (floor of log2).
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::log2;
///
/// assert_eq!(log2(1), 0);
/// assert_eq!(log2(2), 1);
/// assert_eq!(log2(4), 2);
/// assert_eq!(log2(8), 3);
/// assert_eq!(log2(16), 4);
/// ```
#[inline]
pub fn log2(n: u64) -> u32 {
    if n == 0 {
        return u32::MAX; // Undefined, return max
    }
    63 - n.leading_zeros()
}

/// Computes base-10 logarithm (approximate).
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::log10;
///
/// assert_eq!(log10(1), 0);
/// assert_eq!(log10(10), 1);
/// assert_eq!(log10(100), 2);
/// assert_eq!(log10(1000), 3);
/// ```
#[inline]
pub fn log10(mut n: u64) -> u32 {
    if n == 0 {
        return u32::MAX; // Undefined
    }

    let mut log = 0u32;

    // Powers of 2
    if n >= 1000000000 {
        n /= 1000000000;
        log += 9;
    }
    if n >= 100000000 {
        n /= 100000000;
        log += 8;
    }
    if n >= 10000000 {
        n /= 10000000;
        log += 7;
    }
    if n >= 1000000 {
        n /= 1000000;
        log += 6;
    }
    if n >= 100000 {
        n /= 100000;
        log += 5;
    }
    if n >= 10000 {
        n /= 10000;
        log += 4;
    }
    if n >= 1000 {
        n /= 1000;
        log += 3;
    }
    if n >= 100 {
        n /= 100;
        log += 2;
    }
    if n >= 10 {
        n /= 10;
        log += 1;
    }

    log
}

/// Raises a number to a power (exponentiation by squaring).
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::pow;
///
/// assert_eq!(pow(2u32, 10), 1024);
/// assert_eq!(pow(5u64, 3), 125);
/// ```
#[inline]
pub fn pow<T: Into<u64>>(base: T, exp: u32) -> u64 {
    let base = base.into();
    if exp == 0 {
        return 1;
    }

    let mut result = 1u64;
    let mut base = base;
    let mut exp = exp;

    while exp > 0 {
        if exp & 1 == 1 {
            result *= base;
        }
        exp >>= 1;
        base *= base;
    }

    result
}

/// Computes the nth triangular number.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::triangular;
///
/// assert_eq!(triangular(0), 0);
/// assert_eq!(triangular(1), 1);
/// assert_eq!(triangular(2), 3);
/// assert_eq!(triangular(3), 6);
/// assert_eq!(triangular(10), 55);
/// ```
#[inline]
pub const fn triangular(n: u64) -> u64 {
    n * (n + 1) / 2
}

/// Checks if a number is a triangular number.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::is_triangular;
///
/// assert!(is_triangular(0));
/// assert!(is_triangular(1));
/// assert!(is_triangular(3));
/// assert!(is_triangular(6));
/// assert!(!is_triangular(4));
/// ```
#[inline]
pub fn is_triangular(n: u64) -> bool {
    if n == 0 {
        return true;
    }

    // Solve n(n+1)/2 = target
    // n^2 + n - 2*target = 0
    // n = (-1 + sqrt(1 + 8*target)) / 2
    let discriminant = 1 + 8 * n;
    let sqrt_disc = isqrt(discriminant);

    sqrt_disc * sqrt_disc == discriminant && (sqrt_disc - 1) % 2 == 0
}

/// Computes the nth Catalan number.
///
/// Returns None if the result would overflow.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::catalan;
///
/// assert_eq!(catalan(0), Some(1));
/// assert_eq!(catalan(1), Some(1));
/// assert_eq!(catalan(2), Some(2));
/// assert_eq!(catalan(3), Some(5));
/// assert_eq!(catalan(4), Some(14));
/// ```
#[inline]
pub fn catalan(n: u64) -> Option<u64> {
    // Catalan(n) = binomial(2n, n) / (n + 1)
    let two_n = n.checked_mul(2)?;
    super::number_theory::binomial(two_n, n).map(|c| c / (n as u64 + 1))
}

/// Computes the nth Lucas number.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::lucas;
///
/// assert_eq!(lucas(0), 2);
/// assert_eq!(lucas(1), 1);
/// assert_eq!(lucas(2), 3);
/// assert_eq!(lucas(3), 4));
/// assert_eq!(lucas(4), 7));
/// ```
#[inline]
pub fn lucas(n: u64) -> u64 {
    if n == 0 {
        return 2;
    }

    // Lucas: L(n) = L(n-1) + L(n-2)
    let mut a = 2u64;
    let mut b = 1u64;

    for _ in 1..=n {
        let sum = a.wrapping_add(b);
        a = b;
        b = sum;
    }

    a
}

/// Checks if a number is a perfect power (a^b for some a > 1, b > 1).
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::is_perfect_power;
///
/// assert!(is_perfect_power(4));  // 2^2
/// assert!(is_perfect_power(8));  // 2^3
/// assert!(is_perfect_power(9));  // 3^2
/// assert!(!is_perfect_power(10));
/// ```
#[inline]
pub fn is_perfect_power(n: u64) -> bool {
    if n <= 1 {
        return false;
    }

    // Check for each possible exponent
    for exp in 2..= log2(n) {
        let base = isqrt(n);
        if pow(base, exp as u32) == n {
            return true;
        }
        if pow(base + 1, exp as u32) == n {
            return true;
        }
    }

    false
}

/// Computes the sum of digits.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::digit_sum;
///
/// assert_eq!(digit_sum(1234), 10);
/// assert_eq!(digit_sum(0), 0);
/// ```
#[inline]
pub fn digit_sum(mut n: u64) -> u64 {
    let mut sum = 0;
    while n > 0 {
        sum += n % 10;
        n /= 10;
    }
    sum
}

/// Reverses the digits of a number.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::reverse_digits;
///
/// assert_eq!(reverse_digits(1234), 4321);
/// assert_eq!(reverse_digits(1000), 1); // Leading zeros dropped
/// ```
#[inline]
pub fn reverse_digits(mut n: u64) -> u64 {
    let mut reversed = 0;
    while n > 0 {
        reversed = reversed * 10 + n % 10;
        n /= 10;
    }
    reversed
}

/// Checks if a number is a palindrome.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::is_palindrome;
///
/// assert!(is_palindrome(121));
/// assert!(is_palindrome(1221));
/// assert!(!is_palindrome(123));
/// ```
#[inline]
pub fn is_palindrome(n: u64) -> bool {
    n == reverse_digits(n)
}

/// Computes the number of divisors of a number.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::divisor_count;
///
/// assert_eq!(divisor_count(1), 1);
/// assert_eq!(divisor_count(6), 4); // 1, 2, 3, 6
/// assert_eq!(divisor_count(12), 6); // 1, 2, 3, 4, 6, 12
/// ```
#[inline]
pub fn divisor_count(mut n: u64) -> u64 {
    if n == 0 {
        return 0; // Undefined
    }

    let mut count = 1u64;
    let mut i = 2;

    while i * i <= n {
        if n % i == 0 {
            let mut exp = 0u64;
            while n % i == 0 {
                n /= i;
                exp += 1;
            }
            count *= exp + 1;
        }
        i += 1;
    }

    if n > 1 {
        count *= 2;
    }

    count
}

/// Computes the sum of divisors of a number.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::divisor_sum;
///
/// assert_eq!(divisor_sum(1), 1);
/// assert_eq!(divisor_sum(6), 12); // 1 + 2 + 3 + 6
/// ```
#[inline]
pub fn divisor_sum(mut n: u64) -> u64 {
    if n == 0 {
        return 0;
    }

    let mut sum = 1u64; // 1 is always a divisor
    let mut i = 2;

    while i * i <= n {
        if n % i == 0 {
            sum += i;
            let other = n / i;
            if other != i {
                sum += other;
            }
        }
        i += 1;
    }

    if n > 1 {
        sum += n;
    }

    sum
}

/// Checks if a number is an abundant number (sum of proper divisors > number).
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::is_abundant;
///
/// assert!(is_abundant(12)); // 1+2+3+4+6 = 16 > 12
/// assert!(!is_abundant(6));  // 1+2+3 = 6
/// ```
#[inline]
pub fn is_abundant(n: u64) -> bool {
    if n == 0 {
        return false;
    }
    divisor_sum(n) > n
}

/// Checks if a number is a perfect number (sum of proper divisors = number).
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::math::is_perfect;
///
/// assert!(is_perfect(6));   // 1 + 2 + 3 = 6
/// assert!(is_perfect(28));  // 1 + 2 + 4 + 7 + 14 = 28
/// assert!(!is_perfect(12));
/// ```
#[inline]
pub fn is_perfect(n: u64) -> bool {
    if n == 0 {
        return false;
    }
    divisor_sum(n) == n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(48, 18), 6);
        assert_eq!(gcd(17, 5), 1);
        assert_eq!(gcd(0, 5), 5);
    }

    #[test]
    fn test_lcm() {
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(5, 7), 35);
        assert_eq!(lcm(3, 9), 9);
    }

    #[test]
    fn test_isqrt() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(8), 2);
        assert_eq!(isqrt(9), 3);
    }

    #[test]
    fn test_log2() {
        assert_eq!(log2(1), 0);
        assert_eq!(log2(2), 1);
        assert_eq!(log2(4), 2);
        assert_eq!(log2(8), 3);
    }

    #[test]
    fn test_log10() {
        assert_eq!(log10(1), 0);
        assert_eq!(log10(10), 1);
        assert_eq!(log10(100), 2);
        assert_eq!(log10(1000), 3);
    }

    #[test]
    fn test_pow() {
        assert_eq!(pow(2u32, 10), 1024);
        assert_eq!(pow(5u64, 3), 125);
    }

    #[test]
    fn test_triangular() {
        assert_eq!(triangular(0), 0);
        assert_eq!(triangular(1), 1);
        assert_eq!(triangular(10), 55);
    }

    #[test]
    fn test_catalan() {
        assert_eq!(catalan(0), Some(1));
        assert_eq!(catalan(1), Some(1));
        assert_eq!(catalan(2), Some(2));
        assert_eq!(catalan(3), Some(5));
        assert_eq!(catalan(4), Some(14));
    }

    #[test]
    fn test_lucas() {
        assert_eq!(lucas(0), 2);
        assert_eq!(lucas(1), 1);
        assert_eq!(lucas(4), 7);
    }

    #[test]
    fn test_is_perfect_power() {
        assert!(is_perfect_power(4));
        assert!(is_perfect_power(8));
        assert!(is_perfect_power(9));
        assert!(!is_perfect_power(10));
    }

    #[test]
    fn test_digit_sum() {
        assert_eq!(digit_sum(1234), 10);
        assert_eq!(digit_sum(0), 0);
    }

    #[test]
    fn test_reverse_digits() {
        assert_eq!(reverse_digits(1234), 4321);
        assert_eq!(reverse_digits(1000), 1);
    }

    #[test]
    fn test_is_palindrome() {
        assert!(is_palindrome(121));
        assert!(is_palindrome(1221));
        assert!(!is_palindrome(123));
    }

    #[test]
    fn test_divisor_count() {
        assert_eq!(divisor_count(1), 1);
        assert_eq!(divisor_count(6), 4);
        assert_eq!(divisor_count(12), 6);
    }

    #[test]
    fn test_is_perfect() {
        assert!(is_perfect(6));
        assert!(is_perfect(28));
        assert!(!is_perfect(12));
    }

    #[test]
    fn test_is_abundant() {
        assert!(is_abundant(12));
        assert!(!is_abundant(6));
    }
}
