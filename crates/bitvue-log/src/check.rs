//! CHECK macros - Abseil-style assertion macros.
//!
//! These macros provide more informative failure messages than standard `assert!`
//! by automatically including variable names and values in the error output.
//!
//! # Examples
//!
//! ```no_run
//! use bitvue_log::{check, check_eq, check_lt};
//!
//! fn process_frame(index: usize, data: &[u8]) {
//!     check!(!data.is_empty(), "Frame data cannot be empty");
//!     check_eq!(data[0] & 0x80, 0);
//!     check_lt!(index, 1000, "Frame index out of range");
//! }
//! ```

/// Basic CHECK macro - panics if condition is false.
///
/// # Examples
///
/// ```ignore
/// use bitvue_log::check;
///
/// check!(x > 0);                          // "Check failed: x > 0 [file:line]"
/// check!(x > 0, "x must be positive");    // "Check failed: x > 0 - x must be positive [file:line]"
/// check!(x > 0, "value: {}", x);          // "Check failed: x > 0 - value: 5 [file:line]"
/// ```
#[macro_export]
macro_rules! check {
    ($cond:expr) => {
        if !$cond {
            $crate::__check_failed(stringify!($cond), file!(), line!(), None);
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        if !$cond {
            $crate::__check_failed(
                stringify!($cond),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    };
}

/// CHECK_EQ - Check that two values are equal.
///
/// On failure, prints both values for debugging.
///
/// # Examples
///
/// ```ignore
/// use bitvue_log::check_eq;
///
/// check_eq!(a, b);  // "Check failed: a == b (a=5, b=3) [file:line]"
/// ```
#[macro_export]
macro_rules! check_eq {
    ($left:expr, $right:expr) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val == *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "==",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val == *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "==",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_NE - Check that two values are not equal.
#[macro_export]
macro_rules! check_ne {
    ($left:expr, $right:expr) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val != *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "!=",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val != *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "!=",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_LT - Check that left < right.
#[macro_export]
macro_rules! check_lt {
    ($left:expr, $right:expr) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val < *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "<",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val < *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "<",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_LE - Check that left <= right.
#[macro_export]
macro_rules! check_le {
    ($left:expr, $right:expr) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val <= *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "<=",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val <= *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "<=",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_GT - Check that left > right.
#[macro_export]
macro_rules! check_gt {
    ($left:expr, $right:expr) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val > *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                ">",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val > *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                ">",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_GE - Check that left >= right.
#[macro_export]
macro_rules! check_ge {
    ($left:expr, $right:expr) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val >= *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                ">=",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left_val = &$left;
        let right_val = &$right;
        if !(*left_val >= *right_val) {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                ">=",
                &format!("{:?}", left_val),
                &format!("{:?}", right_val),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_STREQ - Check that two string slices are equal.
#[macro_export]
macro_rules! check_streq {
    ($left:expr, $right:expr) => {{
        let left_val: &str = $left;
        let right_val: &str = $right;
        if left_val != right_val {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "==",
                left_val,
                right_val,
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let left_val: &str = $left;
        let right_val: &str = $right;
        if left_val != right_val {
            $crate::__check_binary_failed(
                stringify!($left),
                stringify!($right),
                "==",
                left_val,
                right_val,
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_SOME - Check that an Option is Some.
#[macro_export]
macro_rules! check_some {
    ($opt:expr) => {{
        if $opt.is_none() {
            $crate::__check_failed(
                &format!("{}.is_some()", stringify!($opt)),
                file!(),
                line!(),
                None,
            );
        }
    }};
    ($opt:expr, $($arg:tt)+) => {{
        if $opt.is_none() {
            $crate::__check_failed(
                &format!("{}.is_some()", stringify!($opt)),
                file!(),
                line!(),
                Some(format!($($arg)+)),
            );
        }
    }};
}

/// CHECK_OK - Check that a Result is Ok.
#[macro_export]
macro_rules! check_ok {
    ($result:expr) => {{
        if let Err(ref e) = $result {
            $crate::__check_failed(
                &format!("{}.is_ok()", stringify!($result)),
                file!(),
                line!(),
                Some(format!("Got Err: {:?}", e)),
            );
        }
    }};
    ($result:expr, $($arg:tt)+) => {{
        if let Err(ref e) = $result {
            $crate::__check_failed(
                &format!("{}.is_ok()", stringify!($result)),
                file!(),
                line!(),
                Some(format!("{} - Got Err: {:?}", format!($($arg)+), e)),
            );
        }
    }};
}

/// DCHECK - Debug-only CHECK (optimized out in release builds).
#[macro_export]
macro_rules! dcheck {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::check!($($arg)+);
    };
}

/// DCHECK_EQ - Debug-only CHECK_EQ.
#[macro_export]
macro_rules! dcheck_eq {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::check_eq!($($arg)+);
    };
}

/// DCHECK_NE - Debug-only CHECK_NE.
#[macro_export]
macro_rules! dcheck_ne {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::check_ne!($($arg)+);
    };
}

/// DCHECK_LT - Debug-only CHECK_LT.
#[macro_export]
macro_rules! dcheck_lt {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::check_lt!($($arg)+);
    };
}

/// DCHECK_LE - Debug-only CHECK_LE.
#[macro_export]
macro_rules! dcheck_le {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::check_le!($($arg)+);
    };
}

/// DCHECK_GT - Debug-only CHECK_GT.
#[macro_export]
macro_rules! dcheck_gt {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::check_gt!($($arg)+);
    };
}

/// DCHECK_GE - Debug-only CHECK_GE.
#[macro_export]
macro_rules! dcheck_ge {
    ($($arg:tt)+) => {
        #[cfg(debug_assertions)]
        $crate::check_ge!($($arg)+);
    };
}

// Helper functions (not part of public API, but must be public for macro expansion)

/// Internal: Handle CHECK failure.
#[doc(hidden)]
#[cold]
#[inline(never)]
pub fn __check_failed(cond: &str, file: &str, line: u32, msg: Option<String>) -> ! {
    let full_msg = match msg {
        Some(m) => format!("Check failed: {} - {} [{}:{}]", cond, m, file, line),
        None => format!("Check failed: {} [{}:{}]", cond, file, line),
    };
    tracing::error!("{}", full_msg);
    panic!("{}", full_msg);
}

/// Internal: Handle binary comparison CHECK failure.
#[doc(hidden)]
#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn __check_binary_failed(
    left_expr: &str,
    right_expr: &str,
    op: &str,
    left_val: &str,
    right_val: &str,
    file: &str,
    line: u32,
    msg: Option<String>,
) -> ! {
    let base_msg = format!(
        "Check failed: {} {} {} ({} vs {}) [{}:{}]",
        left_expr, op, right_expr, left_val, right_val, file, line
    );
    let full_msg = match msg {
        Some(m) => format!("{} - {}", base_msg, m),
        None => base_msg,
    };
    tracing::error!("{}", full_msg);
    panic!("{}", full_msg);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_check_passes() {
        crate::check!(true);
        crate::check!(1 + 1 == 2);
        crate::check!(1 < 2, "one should be less than two");
    }

    #[test]
    #[should_panic(expected = "Check failed: false")]
    fn test_check_fails() {
        crate::check!(false);
    }

    #[test]
    #[should_panic(expected = "custom message")]
    fn test_check_fails_with_message() {
        crate::check!(false, "custom message");
    }

    #[test]
    fn test_check_eq_passes() {
        crate::check_eq!(5, 5);
        crate::check_eq!("hello", "hello");
    }

    #[test]
    #[should_panic(expected = "5 vs 3")]
    fn test_check_eq_fails() {
        crate::check_eq!(5, 3);
    }

    #[test]
    fn test_check_ne_passes() {
        crate::check_ne!(5, 3);
    }

    #[test]
    #[should_panic(expected = "!=")]
    fn test_check_ne_fails() {
        crate::check_ne!(5, 5);
    }

    #[test]
    fn test_check_lt_passes() {
        crate::check_lt!(3, 5);
    }

    #[test]
    #[should_panic(expected = "<")]
    fn test_check_lt_fails() {
        crate::check_lt!(5, 3);
    }

    #[test]
    fn test_check_le_passes() {
        crate::check_le!(3, 5);
        crate::check_le!(5, 5);
    }

    #[test]
    fn test_check_gt_passes() {
        crate::check_gt!(5, 3);
    }

    #[test]
    fn test_check_ge_passes() {
        crate::check_ge!(5, 3);
        crate::check_ge!(5, 5);
    }

    #[test]
    fn test_check_streq_passes() {
        crate::check_streq!("hello", "hello");
    }

    #[test]
    #[should_panic(expected = "hello vs world")]
    fn test_check_streq_fails() {
        crate::check_streq!("hello", "world");
    }

    #[test]
    fn test_check_some_passes() {
        crate::check_some!(Some(42));
    }

    #[test]
    #[should_panic(expected = "is_some()")]
    fn test_check_some_fails() {
        let opt: Option<i32> = None;
        crate::check_some!(opt);
    }

    #[test]
    fn test_check_ok_passes() {
        let result: Result<i32, &str> = Ok(42);
        crate::check_ok!(result);
    }

    #[test]
    #[should_panic(expected = "is_ok()")]
    fn test_check_ok_fails() {
        let result: Result<i32, &str> = Err("error");
        crate::check_ok!(result);
    }

    #[test]
    fn test_dcheck_passes() {
        crate::dcheck!(true);
        crate::dcheck_eq!(5, 5);
    }

    // In debug mode, dcheck should panic on false
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Check failed")]
    fn test_dcheck_fails_in_debug() {
        crate::dcheck!(false);
    }
}
