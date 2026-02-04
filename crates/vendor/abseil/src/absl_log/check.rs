//! CHECK macros for runtime assertions.
//!
//! These macros provide assertion functionality similar to Abseil's CHECK macros.
//! Unlike Rust's built-in assert!, CHECK macros are always enabled (even in release).

use std::fmt;
use std::io::{self, Write};

/// Checks that a condition is true, terminating the program if false.
///
/// Unlike `assert!`, this is always enabled (even in release builds).
///
/// # Syntax
///
/// ```ignore
/// check!(condition)
/// check!(condition, "message {}", arg1)
/// ```
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check;
/// # let value = 42;
/// check!(value > 0, "Value must be positive, got {}", value);
/// # }
/// ```
#[macro_export]
macro_rules! check {
    ($cond:expr) => {
        $crate::check_impl!($cond, concat!("Check failed: ", stringify!($cond)))
    };
    ($cond:expr, $($arg:tt)*) => {
        $crate::check_impl!($cond, format_args!($($arg)*))
    };
}

/// Internal implementation of check macro.
#[doc(hidden)]
#[doc(hidden)]
#[macro_export]
macro_rules! check_impl {
    ($cond:expr, $msg:expr) => {{
        if !$cond {
            $crate::absl_log::check::do_check_fail(file!(), line!(), format_args!("{}", $msg));
        }
    }};
}

/// CHECK with equality comparison.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_eq;
/// # let a = 42;
/// # let b = 42;
/// check_eq!(a, b, "a and b must be equal");
/// # }
/// ```
#[macro_export]
macro_rules! check_eq {
    ($left:expr, $right:expr) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l == r,
            format_args!("Check failed: {} == {}\n  left: {:?}\n right: {:?}", stringify!($left), stringify!($right), l, r)
        );
    }};
    ($left:expr, $right:expr, $($arg:tt)*) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l == r,
            format_args!("{} == {}\n  left: {:?}\n right: {:?}\nAdditional info: {}", stringify!($left), stringify!($right), l, r, format_args!($($arg)*))
        );
    }};
}

/// CHECK with inequality comparison.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_ne;
/// # let a = 1;
/// # let b = 2;
/// check_ne!(a, b, "a and b must be different");
/// # }
/// ```
#[macro_export]
macro_rules! check_ne {
    ($left:expr, $right:expr) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l != r,
            format_args!(
                "Check failed: {} != {}\n  left: {:?}\n right: {:?}",
                stringify!($left),
                stringify!($right),
                l,
                r
            )
        );
    }};
}

/// CHECK with less-than comparison.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_lt;
/// # let a = 1;
/// # let b = 2;
/// check_lt!(a, b, "a must be less than b");
/// # }
/// ```
#[macro_export]
macro_rules! check_lt {
    ($left:expr, $right:expr) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l < r,
            format_args!(
                "Check failed: {} < {}\n  left: {:?}\n right: {:?}",
                stringify!($left),
                stringify!($right),
                l,
                r
            )
        );
    }};
}

/// CHECK with less-than-or-equal comparison.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_le;
/// # let a = 1;
/// # let b = 2;
/// check_le!(a, b, "a must be less than or equal to b");
/// # }
/// ```
#[macro_export]
macro_rules! check_le {
    ($left:expr, $right:expr) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l <= r,
            format_args!(
                "Check failed: {} <= {}\n  left: {:?}\n right: {:?}",
                stringify!($left),
                stringify!($right),
                l,
                r
            )
        );
    }};
}

/// CHECK with greater-than comparison.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_gt;
/// # let a = 2;
/// # let b = 1;
/// check_gt!(a, b, "a must be greater than b");
/// # }
/// ```
#[macro_export]
macro_rules! check_gt {
    ($left:expr, $right:expr) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l > r,
            format_args!(
                "Check failed: {} > {}\n  left: {:?}\n right: {:?}",
                stringify!($left),
                stringify!($right),
                l,
                r
            )
        );
    }};
}

/// CHECK with greater-than-or-equal comparison.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_ge;
/// # let a = 2;
/// # let b = 2;
/// check_ge!(a, b, "a must be greater than or equal to b");
/// # }
/// ```
#[macro_export]
macro_rules! check_ge {
    ($left:expr, $right:expr) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l >= r,
            format_args!(
                "Check failed: {} >= {}\n  left: {:?}\n right: {:?}",
                stringify!($left),
                stringify!($right),
                l,
                r
            )
        );
    }};
}

/// CHECK that a Result is Ok, returning the inner value.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_ok;
/// # let result: Result<i32, &str> = Ok(42);
/// let value = check_ok!(result);
/// # let result2: Result<i32, &str> = Ok(100);
/// let value2 = check_ok!(result2, "Operation failed");
/// # }
/// ```
#[macro_export]
macro_rules! check_ok {
    ($expr:expr) => {{
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::absl_log::check::do_check_fail(
                    file!(),
                    line!(),
                    format_args!("Check failed: {} is Ok(), but was Err({:?})", stringify!($expr), e)
                );
            }
        }
    }};
    ($expr:expr, $($arg:tt)*) => {{
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::absl_log::check::do_check_fail(
                    file!(),
                    line!(),
                    format_args!("{} is Ok(), but was Err({:?})\nAdditional info: {}", stringify!($expr), e, format_args!($($arg)*))
                );
            }
        }
    }};
}

/// CHECK that an Option is Some, returning the inner value.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_some;
/// # let option_value = Some(42);
/// let value = check_some!(option_value);
/// # let option_value2 = Some(100);
/// let value2 = check_some!(option_value2, "Value must be present");
/// # }
/// ```
#[macro_export]
macro_rules! check_some {
    ($expr:expr) => {{
        match $expr {
            Some(v) => v,
            None => {
                $crate::absl_log::check::do_check_fail(
                    file!(),
                    line!(),
                    format_args!("Check failed: {} is Some(), but was None", stringify!($expr))
                );
            }
        }
    }};
    ($expr:expr, $($arg:tt)*) => {{
        match $expr {
            Some(v) => v,
            None => {
                $crate::absl_log::check::do_check_fail(
                    file!(),
                    line!(),
                    format_args!("{} is Some(), but was None\nAdditional info: {}", stringify!($expr), format_args!($($arg)*))
                );
            }
        }
    }};
}

/// CHECK that two strings are equal.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::check_streq;
/// # let s1 = "hello";
/// # let s2 = "hello";
/// check_streq!(s1, s2, "Strings must match");
/// # }
/// ```
#[macro_export]
macro_rules! check_streq {
    ($left:expr, $right:expr) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l == r,
            format_args!("Check failed: {} == {}\n  left: {:?}\n right: {:?}", stringify!($left), stringify!($right), l, r)
        );
    }};
    ($left:expr, $right:expr, $($arg:tt)*) => {{
        let l = &$left;
        let r = &$right;
        $crate::check_impl!(
            l == r,
            format_args!("{} == {}\n  left: ({:?})\n right: ({:?})\nAdditional info: {}", stringify!($left), stringify!($right), l, r, format_args!($($arg)*))
        );
    }};
}

/// Debug check - only enabled in debug builds.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::dcheck;
/// # fn invariant() -> bool { true }
/// dcheck!(invariant(), "Invariant violated");
/// # }
/// ```
#[macro_export]
macro_rules! dcheck {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            check!($($arg)*);
        }
    };
}

/// Debug check with equality.
#[macro_export]
macro_rules! dcheck_eq {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            check_eq!($($arg)*);
        }
    };
}

/// Debug check with inequality.
#[macro_export]
macro_rules! dcheck_ne {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            check_ne!($($arg)*);
        }
    };
}

/// Debug check with less-than.
#[macro_export]
macro_rules! dcheck_lt {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            check_lt!($($arg)*);
        }
    };
}

/// Debug check with less-than-or-equal.
#[macro_export]
macro_rules! dcheck_le {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            check_le!($($arg)*);
        }
    };
}

/// Debug check with greater-than.
#[macro_export]
macro_rules! dcheck_gt {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            check_gt!($($arg)*);
        }
    };
}

/// Debug check with greater-than-or-equal.
#[macro_export]
macro_rules! dcheck_ge {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            check_ge!($($arg)*);
        }
    };
}

/// Internal function that handles check failure.
pub fn do_check_fail(file: &str, line: u32, msg: fmt::Arguments<'_>) -> ! {
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();

    let _ = writeln!(stderr_lock, "CHECK failure at {}:{}: {}", file, line, msg);
    let _ = stderr_lock.flush();

    #[cfg(feature = "std")]
    std::process::abort();

    #[cfg(not(feature = "std"))]
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_passes() {
        check!(true);
        check!(1 + 1 == 2);
    }

    #[test]
    fn test_check_eq_passes() {
        check_eq!(1, 1);
        check_eq!("hello", "hello");
    }

    #[test]
    fn test_check_ne_passes() {
        check_ne!(1, 2);
        check_ne!("hello", "world");
    }

    #[test]
    fn test_check_lt_passes() {
        check_lt!(1, 2);
    }

    #[test]
    fn test_check_le_passes() {
        check_le!(1, 1);
        check_le!(1, 2);
    }

    #[test]
    fn test_check_gt_passes() {
        check_gt!(2, 1);
    }

    #[test]
    fn test_check_ge_passes() {
        check_ge!(1, 1);
        check_ge!(2, 1);
    }

    #[test]
    fn test_check_ok_passes() {
        let result: Result<i32, &str> = Ok(42);
        let value = check_ok!(result);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_check_some_passes() {
        let option = Some(42);
        let value = check_some!(option);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_check_streq_passes() {
        check_streq!("hello", "hello");
    }

    #[test]
    fn test_dcheck_in_debug() {
        #[cfg(debug_assertions)]
        {
            dcheck!(true);
            dcheck_eq!(1, 1);
        }
        #[cfg(not(debug_assertions))]
        {
            // In release, dcheck should do nothing
            dcheck!(false); // Should not panic
        }
    }

    // Tests for failure cases are omitted because they would cause
    // the program to abort, which would fail the test suite
}
