//! DIE_IF_NULL macro for null pointer checks.
//!
//! This macro provides a convenient way to check for null values
//! and terminate the program with a clear error message if found.

use std::fmt;
use std::io::{self, Write};

/// Checks that the value is not null, terminating if it is.
///
/// This is useful for checking pointers, references, and Option values.
///
/// # Syntax
///
/// ```ignore
/// DIE_IF_NULL(value)
/// DIE_IF_NULL(value, "custom message")
/// ```
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::DIE_IF_NULL;
/// # fn process_data(data: Option<Vec<u8>>) {
/// let _data = DIE_IF_NULL!(data).expect("DIE_IF_NULL should have caught this");
/// // data is guaranteed to be Some here
/// # }
/// # }
/// ```
///
/// The macro returns the original value if it's not null/None, allowing
/// for fluent usage:
///
/// ```ignore
/// # fn main() {
/// use abseil::DIE_IF_NULL;
/// # fn example(optional_value: Option<i32>) -> i32 {
/// *DIE_IF_NULL!(optional_value)
/// # }
/// # }
/// ```
#[macro_export]
macro_rules! DIE_IF_NULL {
    ($expr:expr) => {{
        let value = $expr;
        $crate::absl_log::die_if_null::check_not_null(value, stringify!($expr), file!(), line!())
    }};
    ($expr:expr, $($arg:tt)*) => {{
        let value = $expr;
        $crate::absl_log::die_if_null::check_not_null_msg(
            value,
            stringify!($expr),
            file!(),
            line!(),
            format_args!($($arg)*)
        )
    }};
}

/// DIE_IF_NULL for pointer-like types.
///
/// This version is specifically for raw pointers and references.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::DIE_IF_NULL_PTR;
/// # unsafe fn example(ptr: *const i32) {
/// # let value = 42;
/// # let ptr = &value as *const i32;
/// DIE_IF_NULL_PTR!(ptr);
/// // ptr is guaranteed to be non-null here
/// println!("Value: {}", *ptr);
/// # }
/// # example(&42 as *const i32);
/// # }
/// ```
#[macro_export]
macro_rules! DIE_IF_NULL_PTR {
    ($expr:expr) => {{
        let ptr = $expr;
        if ptr.is_null() {
            $crate::absl_log::die_if_null::die_null(
                stringify!($expr),
                file!(),
                line!(),
                format_args!("{}", "Pointer was null")
            );
        }
        ptr
    }};
    ($expr:expr, $($arg:tt)*) => {{
        let ptr = $expr;
        if ptr.is_null() {
            $crate::absl_log::die_if_null::die_null(
                stringify!($expr),
                file!(),
                line!(),
                format_args!($($arg)*)
            );
        }
        ptr
    }};
}

/// Internal check function for Option types.
pub fn check_not_null<T>(value: Option<T>, expr: &str, file: &str, line: u32) -> T {
    match value {
        Some(v) => v,
        None => {
            die_null(expr, file, line, format_args!("Value is None"));
        }
    }
}

/// Internal check function with custom message.
pub fn check_not_null_msg<T>(
    value: Option<T>,
    expr: &str,
    file: &str,
    line: u32,
    msg: fmt::Arguments<'_>,
) -> T {
    match value {
        Some(v) => v,
        None => {
            die_null(expr, file, line, msg);
        }
    }
}

/// Terminates the program with a null pointer error message.
pub fn die_null(expr: &str, file: &str, line: u32, msg: fmt::Arguments<'_>) -> ! {
    let stderr = io::stderr();
    let mut stderr_lock = stderr.lock();

    let _ = writeln!(
        stderr_lock,
        "NULL pointer dereference: {} at {}:{}: {}",
        expr, file, line, msg
    );
    let _ = stderr_lock.flush();

    #[cfg(feature = "std")]
    std::process::abort();

    #[cfg(not(feature = "std"))]
    loop {
        core::hint::spin_loop();
    }
}

/// Checks that a reference is not null (for FFI contexts).
///
/// # Safety
///
/// The caller must ensure the reference is valid.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::die_if_null::check_ref_not_null;
/// # fn with_option(opt: Option<&i32>) -> &i32 {
/// unsafe { check_ref_not_null(opt, "value") }
/// # }
/// # }
/// ```
#[inline]
pub unsafe fn check_ref_not_null<'a, T>(opt: Option<&'a T>, expr: &str) -> &'a T {
    match opt {
        Some(r) => r,
        None => {
            die_null(expr, file!(), line!(), format_args!("Reference is None"));
        }
    }
}

/// Checks that a mutable reference is not null (for FFI contexts).
///
/// # Safety
///
/// The caller must ensure the reference is valid.
#[inline]
pub unsafe fn check_mut_ref_not_null<'a, T>(opt: Option<&'a mut T>, expr: &str) -> &'a mut T {
    match opt {
        Some(r) => r,
        None => {
            die_null(expr, file!(), line!(), format_args!("Mutable reference is None"));
        }
    }
}

/// Unwraps an Option, terminating if None.
///
/// This is similar to the DIE_IF_NULL macro but as a function.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::die_if_null::unwrap_or_die;
/// let value = Some(42);
/// let result = unwrap_or_die(value, "value");
/// assert_eq!(result, 42);
/// # }
/// ```
#[inline]
pub fn unwrap_or_die<T>(value: Option<T>, expr: &str) -> T {
    match value {
        Some(v) => v,
        None => {
            die_null(expr, file!(), line!(), format_args!("unwrap_or_die failed"));
        }
    }
}

/// Unwraps a Result, terminating if Err.
///
/// # Example
///
/// ```ignore
/// # fn main() {
/// use abseil::absl_log::die_if_null::unwrap_result_or_die;
/// let result: Result<i32, &str> = Ok(42);
/// let value = unwrap_result_or_die(result, "result");
/// assert_eq!(value, 42);
/// # }
/// ```
#[inline]
pub fn unwrap_result_or_die<T, E: std::fmt::Debug>(value: Result<T, E>, expr: &str) -> T {
    match value {
        Ok(v) => v,
        Err(e) => {
            die_null(expr, file!(), line!(), format_args!("Result was Err: {:?}", e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_die_if_null_some() {
        let value = Some(42);
        let result = DIE_IF_NULL!(value);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_die_if_null_with_custom_message() {
        let value = Some("hello");
        let result = DIE_IF_NULL!(value, "Custom error message");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_die_if_null_pointer_non_null() {
        let x = 42;
        let ptr: *const i32 = &x;
        let result = DIE_IF_NULL_PTR!(ptr);
        unsafe {
            assert_eq!(*result, 42);
        }
    }

    #[test]
    fn test_unwrap_or_die_some() {
        let value = Some(42);
        let result = unwrap_or_die(value, "test_value");
        assert_eq!(result, 42);
    }

    #[test]
    fn test_unwrap_result_or_die_ok() {
        let result: Result<i32, &str> = Ok(42);
        let value = unwrap_result_or_die(result, "test_result");
        assert_eq!(value, 42);
    }

    #[test]
    fn test_unwrap_result_or_die_err_with_msg() {
        let result: Result<i32, String> = Err("error".to_string());
        // This would terminate, so we don't actually call it in tests
        // Just verify it compiles
        let _ = result;
    }

    // Tests for None/null cases are omitted because they would
    // cause the program to abort

    #[test]
    fn test_check_ref_not_null_some() {
        let value = 42;
        let opt: Option<&i32> = Some(&value);
        unsafe {
            let result = check_ref_not_null(opt, "test_ref");
            assert_eq!(*result, 42);
        }
    }

    #[test]
    fn test_check_mut_ref_not_null_some() {
        let mut value = 42;
        let opt: Option<&mut i32> = Some(&mut value);
        unsafe {
            let result = check_mut_ref_not_null(opt, "test_mut_ref");
            assert_eq!(*result, 42);
            *result = 100;
            assert_eq!(value, 100);
        }
    }

    #[test]
    fn test_die_if_null_fluent_usage() {
        // Test that DIE_IF_NULL can be used fluently
        let maybe_value = Some(vec![1, 2, 3]);
        let length = DIE_IF_NULL!(maybe_value).len();
        assert_eq!(length, 3);
    }

    #[test]
    fn test_die_if_null_with_expression() {
        // Test with a more complex expression
        let value = Some(10);
        let result = *DIE_IF_NULL!(value.as_ref());
        assert_eq!(result, 10);
    }
}
