//! Utility functions for type operations.

use core::cmp::Ordering;
use core::mem::MaybeUninit;

/// Utility for comparing two values.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::compare;
///
/// assert_eq!(compare(&1, &2), Ordering::Less);
/// assert_eq!(compare(&2, &2), Ordering::Equal);
/// assert_eq!(compare(&3, &2), Ordering::Greater);
/// ```
#[inline]
pub fn compare<T: Ord>(a: &T, b: &T) -> Ordering {
    a.cmp(b)
}

/// Utility for clamping a value between min and max.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::clamp;
///
/// assert_eq!(clamp(&5, &0, &10), &5);
/// assert_eq!(clamp(&-5, &0, &10), &0);
/// assert_eq!(clamp(&15, &0, &10), &10);
/// ```
#[inline]
pub fn clamp<T: Ord>(value: &T, min: &T, max: &T) -> &T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Utility for safely casting between integer types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::cast;
///
/// assert_eq!(cast::<u32, i32>(42u32), 42);
/// assert_eq!(cast::<i32, u32>(-1i32), u32::MAX);
/// ```
pub fn cast<T, U>(value: T) -> U
where
    T: Into<U>,
{
    value.into()
}

/// Checked cast that returns None if the value doesn't fit.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::checked_cast;
///
/// assert_eq!(checked_cast::<i32, u32>(100), Some(100));
/// assert_eq!(checked_cast::<i32, u32>(-1), None);
/// ```
pub fn checked_cast<T: TryInto<U> + Copy, U>(value: T) -> Option<U> {
    value.try_into().ok()
}

/// Computes the offset of a field within a struct.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::offset_of;
///
/// struct MyStruct {
///     a: u32,
///     b: u32,
/// }
///
/// let s = MyStruct { a: 1, b: 2 };
/// assert!(offset_of(&s) == 4);
/// ```
#[inline]
pub const fn offset_of<S, U>(_: &S) -> usize {
    let dummy = MaybeUninit::<S>::uninit();
    unsafe {
        // Get the field offset using pointer arithmetic
        let base = &dummy as *const S as *const u8;
        let field_ptr = &dummy as *const S as *const u8;
        field_ptr as usize - base as usize
    }
}

/// Computes the offset of a field within a struct from a base address.
///
/// # Safety
///
/// The base pointer must be a valid pointer to the struct.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::field_offset;
///
/// struct MyStruct {
///     a: u32,
///     b: u32,
/// }
///
/// let s = MyStruct { a: 1, b: 2 };
/// unsafe {
///     assert_eq!(field_offset::<MyStruct, u32>(&s as *const _, &s.b as *const _), 4);
/// }
/// ```
#[inline]
pub unsafe fn field_offset<S, U>(base: *const S, field: *const U) -> usize {
    let field_addr = field as usize;
    let base_addr = base as usize;
    field_addr - base_addr
}

/// A wrapper that prevents the compiler from optimizing away operations.
///
/// This is useful for benchmarks and timing tests.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::black_box;
///
/// let x = 42;
/// let result = black_box(x) + 1;
/// ```
#[inline]
pub fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = core::ptr::read_volatile(&dummy);
        core::ptr::write_volatile(&dummy as *const _ as *mut T, dummy);
        ret
    }
}

/// Prevents a value from being used.
///
/// This is useful for marking code paths as unreachable.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_types::utility_functions::unreachable;
///
/// let x = 42;
/// unsafe { unreachable(x) }; // x is not used
/// ```
#[inline]
pub unsafe fn unreachable<T>(dummy: T) -> ! {
    black_box(dummy);
    core::hint::unreachable_unchecked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(&5, &0, &10), &5);
        assert_eq!(clamp(&-5, &0, &10), &0);
        assert_eq!(clamp(&15, &0, &10), &10);
    }

    #[test]
    fn test_checked_cast() {
        assert_eq!(checked_cast::<i32, u32>(100), Some(100));
        assert_eq!(checked_cast::<i32, u32>(-1), None);
        assert_eq!(checked_cast::<u32, i32>(u32::MAX), Some(u32::MAX as i32));
    }
}
