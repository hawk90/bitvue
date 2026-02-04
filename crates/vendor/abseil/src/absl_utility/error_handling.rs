//! Error handling utilities.

use core::fmt;

/// Converts a `Result` to an `Option`, discarding the error.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::error_handling::ok_or_none;
///
/// assert_eq!(ok_or_none(Ok(42)), Some(42));
/// assert_eq!(ok_or_none::<i32, _>(Err("error")), None);
/// ```
#[inline]
pub fn ok_or_none<T, E>(result: Result<T, E>) -> Option<T> {
    result.ok()
}

/// Converts a `Result` to an `Option`, discarding the value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::error_handling::err_or_none;
///
/// assert_eq!(err_or_none::<(), _>(Ok(42)), None);
/// assert_eq!(err_or_none(Err("error")), Some("error"));
/// ```
#[inline]
pub fn err_or_none<T, E>(result: Result<T, E>) -> Option<E> {
    result.err()
}

/// Returns the value from a `Result`, or a default if it's an error.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::error_handling::unwrap_or_default;
///
/// assert_eq!(unwrap_or_default(Ok(42)), 42);
/// assert_eq!(unwrap_or_default::<i32, _>(Err("error")), 0);
/// ```
#[inline]
pub fn unwrap_or_default<T: Default>(result: Result<T, impl fmt::Display>) -> T {
    result.unwrap_or_default()
}

/// Returns the value from a `Result`, or computes a default if it's an error.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::error_handling::unwrap_or_else;
///
/// assert_eq!(unwrap_or_else(Ok(42), |_| 0), 42);
/// assert_eq!(unwrap_or_else::<i32, _>(Err("error"), |e| e.len() as i32), 7);
/// ```
#[inline]
pub fn unwrap_or_else<T, E, F>(result: Result<T, E>, default: F) -> T
where
    F: FnOnce(E) -> T,
{
    result.unwrap_or_else(default)
}

/// Maps a `Result`'s value using a function, or returns a default error.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::error_handling::map_ok;
///
/// assert_eq!(map_ok(Ok(42), |x| x * 2), Ok(84));
/// assert_eq!(map_ok::<i32, _, _, _>(Err("error"), |x| x * 2), Err("error"));
/// ```
#[inline]
pub fn map_ok<T, E, U, F>(result: Result<T, E>, f: F) -> Result<U, E>
where
    F: FnOnce(T) -> U,
{
    result.map(f)
}

/// Maps a `Result`'s error using a function, or returns a default value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::error_handling::map_err;
///
/// assert_eq!(map_err(Ok(42), |_| "new error"), Ok(42));
/// assert_eq!(map_err::<_, _, _>(Err("error"), |_| "new error"), Err("new error"));
/// ```
#[inline]
pub fn map_err<T, E, F, G>(result: Result<T, E>, f: F) -> Result<T, G>
where
    F: FnOnce(E) -> G,
{
    result.map_err(f)
}

/// Flattens a `Result` of `Result` into a single `Result`.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_utility::error_handling::flatten_result;
///
/// assert_eq!(flatten_result(Ok(Ok(42))), Ok(42));
/// assert_eq!(flatten_result::<_, i32>(Ok(Err("error"))), Err("error"));
/// ```
#[inline]
pub fn flatten_result<T, E>(result: Result<Result<T, E>, E>) -> Result<T, E> {
    result.and_then(|x| x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_utils() {
        assert_eq!(ok_or_none(Ok(42)), Some(42));
        assert_eq!(ok_or_none::<i32, _>(Err("error")), None);
        assert_eq!(err_or_none::<(), _>(Ok(42)), None);
        assert_eq!(err_or_none(Err("error")), Some("error"));
        assert_eq!(unwrap_or_default(Ok(42)), 42);
        assert_eq!(unwrap_or_default::<i32, _>(Err("error")), 0);
    }

    #[test]
    fn test_map_ok() {
        assert_eq!(map_ok(Ok(42), |x| x * 2), Ok(84));
        assert_eq!(map_ok::<i32, _, _, _>(Err("error"), |x| x * 2), Err("error"));
    }

    #[test]
    fn test_map_err() {
        assert_eq!(map_err(Ok(42), |_| "new error"), Ok(42));
        assert_eq!(map_err::<_, _, _>(Err("error"), |_| "new error"), Err("new error"));
    }

    #[test]
    fn test_flatten_result() {
        assert_eq!(flatten_result(Ok(Ok(42))), Ok(42));
        assert_eq!(flatten_result::<_, i32>(Ok(Err("error"))), Err("error"));
    }
}
