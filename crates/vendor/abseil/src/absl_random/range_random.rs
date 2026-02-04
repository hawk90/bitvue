//! Range-based random number generation.

use crate::absl_random::bit_gen::BitGen;

/// Generates a random i8 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_i8;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_i8(10..20, &mut rng);
/// assert!(n >= 10 && n < 20);
/// ```
#[inline]
pub fn random_range_i8(range: core::ops::Range<i8>, rng: &mut BitGen) -> i8 {
    let start = range.start;
    let len = range.end.wrapping_sub(start) as u8;
    start.wrapping_add(rng.gen_u8() % len)
}

/// Generates a random i16 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_i16;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_i16(100..200, &mut rng);
/// assert!(n >= 100 && n < 200);
/// ```
#[inline]
pub fn random_range_i16(range: core::ops::Range<i16>, rng: &mut BitGen) -> i16 {
    let start = range.start;
    let len = range.end.wrapping_sub(start) as u16;
    start.wrapping_add(rng.gen_u16() % len)
}

/// Generates a random i32 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_i32;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_i32(1000..2000, &mut rng);
/// assert!(n >= 1000 && n < 2000);
/// ```
#[inline]
pub fn random_range_i32(range: core::ops::Range<i32>, rng: &mut BitGen) -> i32 {
    let start = range.start;
    let len = range.end.wrapping_sub(start) as u32;
    start.wrapping_add(rng.gen_u32() % len)
}

/// Generates a random i64 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_i64;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_i64(1000..2000, &mut rng);
/// assert!(n >= 1000 && n < 2000);
/// ```
#[inline]
pub fn random_range_i64(range: core::ops::Range<i64>, rng: &mut BitGen) -> i64 {
    let start = range.start;
    let len = range.end.wrapping_sub(start) as u64;
    start.wrapping_add(rng.gen_u64() % len)
}

/// Generates a random isize in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_isize;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_isize(100..200, &mut rng);
/// assert!(n >= 100 && n < 200);
/// ```
#[inline]
pub fn random_range_isize(range: core::ops::Range<isize>, rng: &mut BitGen) -> isize {
    let start = range.start;
    let len = range.end.wrapping_sub(start) as usize;
    start.wrapping_add(rng.gen_usize() % len)
}

/// Generates a random u8 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_u8;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_u8(10..20, &mut rng);
/// assert!(n >= 10 && n < 20);
/// ```
#[inline]
pub fn random_range_u8(range: core::ops::Range<u8>, rng: &mut BitGen) -> u8 {
    let start = range.start;
    let len = range.end.wrapping_sub(start);
    start.wrapping_add(rng.gen_u8() % len)
}

/// Generates a random u16 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_u16;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_u16(100..200, &mut rng);
/// assert!(n >= 100 && n < 200);
/// ```
#[inline]
pub fn random_range_u16(range: core::ops::Range<u16>, rng: &mut BitGen) -> u16 {
    let start = range.start;
    let len = range.end.wrapping_sub(start);
    start.wrapping_add(rng.gen_u16() % len)
}

/// Generates a random u32 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_u32;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_u32(1000..2000, &mut rng);
/// assert!(n >= 1000 && n < 2000);
/// ```
#[inline]
pub fn random_range_u32(range: core::ops::Range<u32>, rng: &mut BitGen) -> u32 {
    let start = range.start;
    let len = range.end.wrapping_sub(start);
    start.wrapping_add(rng.gen_u32() % len)
}

/// Generates a random u64 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_u64;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_u64(1000..2000, &mut rng);
/// assert!(n >= 1000 && n < 2000);
/// ```
#[inline]
pub fn random_range_u64(range: core::ops::Range<u64>, rng: &mut BitGen) -> u64 {
    let start = range.start;
    let len = range.end.wrapping_sub(start);
    start.wrapping_add(rng.gen_u64() % len)
}

/// Generates a random usize in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_usize;
///
/// let mut rng = BitGen::new(42);
/// let n = random_range_usize(100..200, &mut rng);
/// assert!(n >= 100 && n < 200);
/// ```
#[inline]
pub fn random_range_usize(range: core::ops::Range<usize>, rng: &mut BitGen) -> usize {
    let start = range.start;
    let len = range.end.wrapping_sub(start);
    start.wrapping_add(rng.gen_usize() % len)
}

/// Generates a random f32 in the range [0.0, 1.0).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_f32;
///
/// let mut rng = BitGen::new(42);
/// let f = random_f32(&mut rng);
/// assert!(f >= 0.0 && f < 1.0);
/// ```
#[inline]
pub fn random_f32(rng: &mut BitGen) -> f32 {
    (rng.gen_u32() as f64 / (u32::MAX as f64)) as f32
}

/// Generates a random f64 in the range [0.0, 1.0].
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_f64;
///
/// let mut rng = BitGen::new(42);
/// let f = random_f64(&mut rng);
/// assert!(f >= 0.0 && f < 1.0);
/// ```
#[inline]
pub fn random_f64(rng: &mut BitGen) -> f64 {
    rng.gen_u64() as f64 / (u64::MAX as f64)
}

/// Generates a random f32 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_f32;
///
/// let mut rng = BitGen::new(42);
/// let f = random_range_f32(10.0..20.0, &mut rng);
/// assert!(f >= 10.0 && f < 20.0);
/// ```
#[inline]
pub fn random_range_f32(range: core::ops::Range<f32>, rng: &mut BitGen) -> f32 {
    range.start + random_f32(rng) * (range.end - range.start)
}

/// Generates a random f64 in the given range.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_random::random_range_f64;
///
/// let mut rng = BitGen::new(42);
/// let f = random_range_f64(10.0..20.0, &mut rng);
/// assert!(f >= 10.0 && f < 20.0);
/// ```
#[inline]
pub fn random_range_f64(range: core::ops::Range<f64>, rng: &mut BitGen) -> f64 {
    range.start + random_f64(rng) * (range.end - range.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_range_i32() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let n = random_range_i32(100..200, &mut rng);
            assert!(n >= 100 && n < 200);
        }
    }

    #[test]
    fn test_random_range_u32() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let n = random_range_u32(100..200, &mut rng);
            assert!(n >= 100 && n < 200);
        }
    }

    #[test]
    fn test_random_range_f32() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let f = random_range_f32(10.0..20.0, &mut rng);
            assert!(f >= 10.0 && f < 20.0);
        }
    }

    #[test]
    fn test_random_range_f64() {
        let mut rng = BitGen::new(42);
        for _ in 0..100 {
            let f = random_range_f64(10.0..20.0, &mut rng);
            assert!(f >= 10.0 && f < 20.0);
        }
    }
}
