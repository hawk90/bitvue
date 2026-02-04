//! Number theory utilities.

/// Checks if a number is prime (trial division).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::number_theory::is_prime;
///
/// assert!(is_prime(2));
/// assert!(is_prime(3));
/// assert!(is_prime(5));
/// assert!(is_prime(7));
/// assert!(!is_prime(4));
/// assert!(!is_prime(1));
/// assert!(!is_prime(0));
/// ```
#[inline]
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    let sqrt_n = (n as f64).sqrt() as u64;
    let mut i = 3;
    while i <= sqrt_n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

/// Computes factorial.
///
/// Returns None if the result would overflow.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::number_theory::factorial;
///
/// assert_eq!(factorial(0), Some(1));
/// assert_eq!(factorial(1), Some(1));
/// assert_eq!(factorial(5), Some(120));
/// assert_eq!(factorial(20), None); // Too large
/// ```
#[inline]
pub fn factorial(n: u64) -> Option<u64> {
    let mut result = 1u64;
    for i in 2..=n {
        match result.checked_mul(i) {
            Some(v) => result = v,
            None => return None,
        }
    }
    Some(result)
}

/// Computes Fibonacci number iteratively.
///
/// Returns None if the result would overflow.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::number_theory::fibonacci;
///
/// assert_eq!(fibonacci(0), Some(0));
/// assert_eq!(fibonacci(1), Some(1));
/// assert_eq!(fibonacci(2), Some(1));
/// assert_eq!(fibonacci(10), Some(55));
/// assert_eq!(fibonacci(100), None); // Too large
/// ```
#[inline]
pub fn fibonacci(n: u64) -> Option<u64> {
    if n == 0 {
        return Some(0);
    }

    let mut a = 0u64;
    let mut b = 1u64;

    for _ in 1..=n {
        match a.checked_add(b) {
            Some(sum) => {
                a = b;
                b = sum;
            }
            None => return None,
        }
    }

    Some(a)
}

/// Computes binomial coefficient "n choose k".
///
/// Returns None if the result would overflow.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::number_theory::binomial;
///
/// assert_eq!(binomial(5, 2), Some(10));
/// assert_eq!(binomial(10, 3), Some(120));
/// assert_eq!(binomial(10, 0), Some(1));
/// assert_eq!(binomial(10, 10), Some(1));
/// assert_eq!(binomial(10, 11), Some(0));
/// ```
#[inline]
pub fn binomial(n: u64, k: u64) -> Option<u64> {
    if k > n {
        return Some(0);
    }
    if k > n - k {
        k = n - k;
    }
    if k == 0 || k == n {
        return Some(1);
    }

    let mut result = 1u64;
    for i in 0..k {
        result = result.checked_mul(n - i)?;
        result = result / (i + 1);
    }
    Some(result)
}

/// Computes the modular inverse using extended Euclidean algorithm.
///
/// Returns None if the inverse doesn't exist.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::number_theory::mod_inverse;
///
/// assert_eq!(mod_inverse(3, 7), Some(5)); // 3 * 5 = 15 ≡ 1 (mod 7)
/// assert_eq!(mod_inverse(2, 6), None); // No inverse, gcd(2,6) != 1
/// ```
#[inline]
pub fn mod_inverse(a: i64, m: i64) -> Option<i64> {
    let m = m.rem_euclid(a);
    if m != 1 {
        return None; // No inverse exists
    }

    // Extended Euclidean algorithm
    let (mut x, mut y) = (0i64, 1i64);
    let (mut a, mut m) = (a, m);

    while a != 0 {
        let q = m / a;
        m = m % a;
        core::mem::swap(&mut a, &mut m);
        let temp = x - q * y;
        x = y;
        y = temp;
    }

    Some(x.rem_euclid(m))
}

/// Computes modular exponentiation: base^exp mod mod.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::number_theory::mod_pow;
///
/// assert_eq!(mod_pow(2, 10, 1000), Some(24)); // 2^10 = 1024 ≡ 24 (mod 1000)
/// assert_eq!(mod_pow(3, 4, 100), Some(81));   // 3^4 = 81
/// ```
#[inline]
pub fn mod_pow(mut base: i64, exp: u64, modulus: i64) -> Option<i64> {
    if modulus == 0 {
        return None;
    }

    let mut result = 1i64;
    base = base % modulus;

    while exp > 0 {
        if exp & 1 == 1 {
            result = result.checked_mul(base % modulus)?;
        }
        exp >>= 1;
        base = base.checked_mul(base % modulus)?;
    }

    Some(result % modulus)
}
