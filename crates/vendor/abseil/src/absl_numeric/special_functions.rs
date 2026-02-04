//! Extended math and special functions.
//!
//! Additional mathematical functions beyond the basic math module.


extern crate alloc;

use alloc::vec::Vec;

/// Computes combinations (n choose k).
///
/// Returns None if k > n or if the result would overflow.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::special_functions::combinations;
///
/// assert_eq!(combinations(5, 2), Some(10));
/// assert_eq!(combinations(10, 3), Some(120));
/// assert_eq!(combinations(5, 10), None);
/// ```
pub fn combinations(n: u64, k: u64) -> Option<u64> {
    if k > n {
        return None;
    }

    // Use symmetry: C(n,k) = C(n,n-k)
    let k = k.min(n - k);

    if k == 0 {
        return Some(1);
    }

    let mut result = 1u64;
    for i in 0..k {
        // Multiply by (n - i) and divide by (i + 1)
        // Check for overflow before multiplication
        let numerator = n - i;
        if result > u64::MAX / numerator {
            return None; // Would overflow
        }
        result = result * numerator;
        result = result / (i + 1);
    }

    Some(result)
}

/// Computes permutations (nPk).
///
/// Returns None if k > n or if the result would overflow.
pub fn permutations(n: u64, k: u64) -> Option<u64> {
    if k > n {
        return None;
    }

    let mut result = 1u64;
    for i in 0..k {
        if result > u64::MAX / (n - i) {
            return None; // Would overflow
        }
        result = result * (n - i);
    }

    Some(result)
}

/// Checks if a number is prime using trial division.
///
/// # Examples
///
/// ```
/// use abseil::absl_numeric::special_functions::is_prime_extended;
///
/// assert!(is_prime_extended(2));
/// assert!(is_prime_extended(3));
/// assert!(is_prime_extended(97));
/// assert!(!is_prime_extended(100));
/// ```
pub fn is_prime_extended(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }

    // Check divisibility up to sqrt(n)
    let mut i = 5u64;
    while i.wrapping_mul(i) <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }

    true
}

/// Finds the next prime number greater than or equal to n.
pub fn next_prime(n: u64) -> Option<u64> {
    if n > u64::MAX - 2 {
        return None;
    }

    let mut candidate = core::cmp::max(n, 2);
    if candidate % 2 == 0 {
        candidate += 1;
    }

    loop {
        if is_prime_extended(candidate) {
            return Some(candidate);
        }
        candidate += 2;
        if candidate > u64::MAX - 2 {
            return None; // Would overflow
        }
    }
}

/// Finds all prime factors of a number.
pub fn prime_factors(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();

    // Factor out 2s
    while n % 2 == 0 {
        factors.push(2);
        n /= 2;
    }

    // Check odd factors from 3 onwards
    let mut i = 3u64;
    while i.wrapping_mul(i) <= n {
        while n % i == 0 {
            factors.push(i);
            n /= i;
        }
        i += 2;
    }

    if n > 1 {
        factors.push(n);
    }

    factors
}

/// Computes the nth Fibonacci number using fast doubling.
///
/// Returns None if the result would overflow.
pub fn fibonacci_fast(n: u64) -> Option<u64> {
    fn fib_pair(n: u64) -> Option<(u64, u64)> {
        if n == 0 {
            return Some((0, 1));
        }

        let (a, b) = fib_pair(n / 2)?;

        // c = F(2k), d = F(2k+1)
        let c = a.wrapping_mul(2).wrapping_sub(b).wrapping_mul(b);
        let d = a.wrapping_mul(a).wrapping_add(b.wrapping_mul(b));

        if c.checked_add(d).is_none() {
            return None; // Would overflow
        }

        if n % 2 == 0 {
            Some((c, d))
        } else {
            Some((d, c.wrapping_add(d)))
        }
    }

    fib_pair(n).map(|(f, _)| f)
}

/// Computes Euler's totient function φ(n).
///
/// Counts the numbers from 1 to n that are coprime to n.
pub fn euler_totient(mut n: u64) -> u64 {
    if n == 0 {
        return 0;
    }

    let mut result = n;

    // Factor out 2
    if n % 2 == 0 {
        result -= result / 2;
        while n % 2 == 0 {
            n /= 2;
        }
    }

    // Check odd factors
    let mut i = 3u64;
    while i.wrapping_mul(i) <= n {
        if n % i == 0 {
            result -= result / i;
            while n % i == 0 {
                n /= i;
            }
        }
        i += 2;
    }

    if n > 1 {
        result -= result / n;
    }

    result
}

/// Computes the binomial coefficient (n choose k) using multiplicative formula.
pub fn binomial_multiplicative(n: u64, k: u64) -> Option<u64> {
    if k > n {
        return None;
    }

    let k = k.min(n - k);

    if k == 0 {
        return Some(1);
    }

    let mut result = 1u64;
    for i in 0..k {
        // result = result * (n - i) / (i + 1)
        // To avoid division issues, we multiply then divide
        // But need to be careful about order
        let g = gcd(result, i + 1);
        result /= g;

        let numerator = n - i;
        let denom = (i + 1) / g;

        if result > u64::MAX / numerator {
            return None; // Would overflow
        }
        result = result * numerator / denom;
    }

    Some(result)
}

/// Computes the greatest common divisor of multiple numbers.
pub fn gcd_multiple(numbers: &[u64]) -> u64 {
    if numbers.is_empty() {
        return 0;
    }

    let mut result = numbers[0];
    for &num in &numbers[1..] {
        result = gcd_two(result, num);
        if result == 1 {
            break; // Can't get smaller than 1
        }
    }

    result
}

/// Helper function for GCD of two numbers
fn gcd_two(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Computes the least common multiple of multiple numbers.
pub fn lcm_multiple(numbers: &[u64]) -> Option<u64> {
    if numbers.is_empty() {
        return Some(0);
    }

    if numbers.iter().any(|&n| n == 0) {
        return Some(0);
    }

    let mut result = numbers[0];
    for &num in &numbers[1..] {
        let g = gcd_two(result, num);
        result = result / g * num;

        if result > u64::MAX - 1 {
            return None; // Would overflow on next iteration
        }
    }

    Some(result)
}

/// Checks if two numbers are coprime (gcd = 1).
pub fn are_coprime(a: u64, b: u64) -> bool {
    gcd_two(a, b) == 1
}

/// Computes the nth number in the look-and-say sequence.
///
/// Each term describes the previous term.
/// 1, 11, 21, 1211, 111221, ...
pub fn look_and_say(n: usize) -> String {
    let mut current = String::from("1");

    for _ in 1..n {
        current = next_look_and_say(&current);
    }

    current
}

/// Generates the next term in the look-and-say sequence.
fn next_look_and_say(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        let mut count = 1usize;

        while chars.peek() == Some(&c) {
            chars.next();
            count += 1;
        }

        result.push_str(&count.to_string());
        result.push(c);
    }

    result
}

/// Checks if a number is in the look-and-say sequence.
pub fn is_look_and_say(n: u64) -> bool {
    let s = n.to_string();

    // Look-and-say numbers only contain digits 1, 2, and 3
    if s.chars().any(|c| !matches!(c, '1' | '2' | '3')) {
        return false;
    }

    // Check that no digit appears more than 3 times in a row
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let mut count = 1;

        while i + count < chars.len() && chars[i + count] == c {
            count += 1;
        }

        if count > 3 {
            return false;
        }

        i += count;
    }

    true
}

use crate::absl_numeric::math::gcd;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combinations() {
        assert_eq!(combinations(5, 2), Some(10));
        assert_eq!(combinations(10, 3), Some(120));
        assert_eq!(combinations(5, 10), None);
        assert_eq!(combinations(0, 0), Some(1));
    }

    #[test]
    fn test_permutations() {
        assert_eq!(permutations(5, 2), Some(20));
        assert_eq!(permutations(10, 3), Some(720));
        assert_eq!(permutations(5, 10), None);
    }

    #[test]
    fn test_is_prime_extended() {
        assert!(is_prime_extended(2));
        assert!(is_prime_extended(3));
        assert!(is_prime_extended(97));
        assert!(!is_prime_extended(100));
        assert!(is_prime_extended(997));
    }

    #[test]
    fn test_next_prime() {
        assert_eq!(next_prime(10), Some(11));
        assert_eq!(next_prime(17), Some(17));
        assert_eq!(next_prime(100), Some(101));
    }

    #[test]
    fn test_prime_factors() {
        let factors = prime_factors(100);
        assert_eq!(factors, vec![2, 2, 5, 5]);

        let factors = prime_factors(97);
        assert_eq!(factors, vec![97]);
    }

    #[test]
    fn test_fibonacci_fast() {
        assert_eq!(fibonacci_fast(0), Some(0));
        assert_eq!(fibonacci_fast(1), Some(1));
        assert_eq!(fibonacci_fast(10), Some(55));
        assert_eq!(fibonacci_fast(50), Some(12586269025));
    }

    #[test]
    fn test_euler_totient() {
        assert_eq!(euler_totient(1), 1);
        assert_eq!(euler_totient(7), 6);
        assert_eq!(euler_totient(12), 4);
        assert_eq!(euler_totient(30), 8);
    }

    #[test]
    fn test_gcd_multiple() {
        assert_eq!(gcd_multiple(&[12, 18, 24]), 6);
        assert_eq!(gcd_multiple(&[7, 13, 17]), 1);
        assert_eq!(gcd_multiple(&[100, 100, 100]), 100);
    }

    #[test]
    fn test_lcm_multiple() {
        assert_eq!(lcm_multiple(&[2, 3, 4]), Some(12));
        assert_eq!(lcm_multiple(&[5, 7, 9]), Some(315));
    }

    #[test]
    fn test_are_coprime() {
        assert!(are_coprime(7, 13));
        assert!(!are_coprime(12, 18));
        assert!(are_coprime(17, 19));
    }

    #[test]
    fn test_look_and_say() {
        assert_eq!(look_and_say(0), "1");
        assert_eq!(look_and_say(1), "11");
        assert_eq!(look_and_say(2), "21");
        assert_eq!(look_and_say(3), "1211");
    }

    #[test]
    fn test_is_look_and_say() {
        assert!(is_look_and_say(1));
        assert!(is_look_and_say(11));
        assert!(is_look_and_say(21));
        assert!(is_look_and_say(1211));
        assert!(!is_look_and_say(4));
        assert!(!is_look_and_say(10));
    }
}
