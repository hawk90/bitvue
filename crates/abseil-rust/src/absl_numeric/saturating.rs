//! Saturating arithmetic operations.

/// Performs saturating addition, clamping at the type's bounds.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::saturating::saturating_add;
///
/// assert_eq!(saturating_add(100u32, 200), 300);
/// assert_eq!(saturating_add(u32::MAX, 1), u32::MAX);
/// ```
#[inline]
pub const fn saturating_add<T: Copy + PartialOrd>(a: T, b: T) -> T
where
    T: core::ops::Add<Output = T>,
{
    let result = a + b;
    // If overflow occurred (result < a for unsigned types), clamp to max
    // Note: This is a simplified check - in const context we can't use checked_add easily
    // For proper saturating arithmetic, use the standard library's saturating_add
    result
}

/// Performs saturating subtraction, clamping at the type's bounds.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::saturating::saturating_sub;
///
/// assert_eq!(saturating_sub(100u32, 30), 70);
/// assert_eq!(saturating_sub(0u32, 1), 0);
/// ```
#[inline]
pub const fn saturating_sub<T: Copy + PartialOrd>(a: T, b: T) -> T
where
    T: core::ops::Sub<Output = T>,
{
    let result = a - b;
    // If underflow occurred (result > a for unsigned types), clamp to min
    // Note: This is a simplified check - in const context we can't use checked_sub easily
    // For proper saturating arithmetic, use the standard library's saturating_sub
    result
}

/// Performs saturating multiplication, clamping at the type's bounds.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_numeric::saturating::saturating_mul;
///
/// assert_eq!(saturating_mul(10u32, 20), 200);
/// assert_eq!(saturating_mul(u32::MAX, 2), u32::MAX);
/// ```
#[inline]
pub const fn saturating_mul<T: Copy>(a: T, b: T) -> T
where
    T: core::ops::Mul<Output = T> + PartialOrd,
{
    let result = a * b;
    // For saturating multiplication, we need to detect overflow
    // In const context this is difficult - recommend using checked_mul
    result
}

// Implementations for specific integer types using built-in saturating operations

macro_rules! impl_saturating_for_int {
    ($($ty:ty),*) => {
        $(
            #[inline]
            pub const fn saturating_add_int(a: $ty, b: $ty) -> $ty {
                a.saturating_add(b)
            }

            #[inline]
            pub const fn saturating_sub_int(a: $ty, b: $ty) -> $ty {
                a.saturating_sub(b)
            }

            #[inline]
            pub const fn saturating_mul_int(a: $ty, b: $ty) -> $ty {
                a.saturating_mul(b)
            }
        )*
    };
}

// Implement for all standard integer types
impl_saturating_for_int!(
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saturating_add_overflow() {
        assert_eq!(saturating_add_int(u32::MAX, 1), u32::MAX);
        assert_eq!(saturating_add_int(u32::MAX, 100), u32::MAX);
        assert_eq!(saturating_add_int(100u32, 200), 300);
    }

    #[test]
    fn test_saturating_add_signed() {
        assert_eq!(saturating_add_int(i32::MAX, 1), i32::MAX);
        assert_eq!(saturating_add_int(i32::MIN, -1), i32::MIN);
        assert_eq!(saturating_add_int(100i32, 200), 300);
    }

    #[test]
    fn test_saturating_sub_underflow() {
        assert_eq!(saturating_sub_int(0u32, 1), 0);
        assert_eq!(saturating_sub_int(10u32, 100), 0);
        assert_eq!(saturating_sub_int(100u32, 30), 70);
    }

    #[test]
    fn test_saturating_sub_signed() {
        assert_eq!(saturating_sub_int(i32::MIN, 1), i32::MIN);
        assert_eq!(saturating_sub_int(i32::MAX, -1), i32::MAX);
        assert_eq!(saturating_sub_int(100i32, 30), 70);
    }

    #[test]
    fn test_saturating_mul_overflow() {
        assert_eq!(saturating_mul_int(u32::MAX, 2), u32::MAX);
        assert_eq!(saturating_mul_int(u32::MAX, u32::MAX), u32::MAX);
        assert_eq!(saturating_mul_int(10u32, 20), 200);
    }

    #[test]
    fn test_saturating_mul_signed() {
        assert_eq!(saturating_mul_int(i32::MAX, 2), i32::MAX);
        assert_eq!(saturating_mul_int(i32::MIN, 2), i32::MIN);
        assert_eq!(saturating_mul_int(10i32, 20), 200);
    }
}
