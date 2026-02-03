//! Debug and assertion macros.

/// A debug assertion that is only checked in debug builds.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::debug_macros::debug_assert_checked;
///
/// debug_assert_checked!(true); // Never panics
/// ```
#[macro_export]
macro_rules! debug_assert_checked {
    ($cond:expr $(,)?) => {
        if cfg!(debug_assertions) {
            debug_assert!($cond);
        }
    };
    ($cond:expr, $msg:literal $(,)?) => {
        if cfg!(debug_assertions) {
            debug_assert!($cond, $msg);
        }
    };
}

/// A macro for unreachable code that hints to the compiler.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::debug_macros::unreachable_checked;
///
/// let x = Some(42);
/// match x {
///     Some(v) => v,
///     None => unreachable_checked!("None is not possible"),
/// }
/// ```
#[macro_export]
macro_rules! unreachable_checked {
    ($msg:expr) => {
        if cfg!(debug_assertions) {
            unreachable!($msg)
        } else {
            core::hint::unreachable_unchecked()
        }
    };
    () => {
        if cfg!(debug_assertions) {
            unreachable!()
        } else {
            core::hint::unreachable_unchecked()
        }
    };
}

/// A macro for unchecked index access that hints to the compiler.
///
/// # Safety
///
/// The caller must ensure the index is valid.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::debug_macros::get_unchecked_checked;
///
/// let arr = [1, 2, 3, 4, 5];
/// let val = unsafe { get_unchecked_checked!(arr, 2) };
/// assert_eq!(val, &3);
/// ```
#[macro_export]
macro_rules! get_unchecked_checked {
    ($slice:expr, $index:expr) => {{
        if cfg!(debug_assertions) {
            &$slice[$index]
        } else {
            core::hint::assert_unsafe_precondition!(
                core::mem::size_of::<[_]>() > 0 && $index < $slice.len()
            );
            $slice.get_unchecked($index)
        }
    }};
}
