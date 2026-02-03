//! Adapters and wrappers - map_error, map_result, unwrap_or, unwrap_or_else

/// Wraps a function to convert its error type.
#[inline]
pub fn map_error<T, E1, E2, F>(mut f: F, mut mapper: impl FnMut(E1) -> E2) -> impl FnMut() -> Result<T, E2>
where
    F: FnMut() -> Result<T, E1>,
{
    move || f().map_err(|e| mapper(e))
}

/// Wraps a function to convert its output type.
#[inline]
pub fn map_result<T1, T2, E, F>(mut f: F, mut mapper: impl FnMut(T1) -> T2) -> impl FnMut() -> Result<T2, E>
where
    F: FnMut() -> Result<T1, E>,
{
    move || f().map(|v| mapper(v))
}

/// Wraps a function to provide default value on error.
#[inline]
pub fn unwrap_or<T, E, F>(mut f: F, default: T) -> impl FnMut() -> T
where
    F: FnMut() -> Result<T, E>,
    T: Clone,
{
    move || f().unwrap_or_else(|_| default.clone())
}

/// Wraps a function to provide a default value computed by a function.
#[inline]
pub fn unwrap_or_else<T, E, F, D>(mut f: F, mut default: D) -> impl FnMut() -> T
where
    F: FnMut() -> Result<T, E>,
    D: FnMut(E) -> T,
{
    move || f().unwrap_or_else(|e| default(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_result() {
        let f = || Ok::<i32, &str>(5);
        let mapped = map_result(f, |x| x * 2);
        assert_eq!(mapped(), Ok(10));
    }

    #[test]
    fn test_unwrap_or() {
        let f = || Ok::<i32, &str>(5);
        let unwrap_or_f = unwrap_or(f, 0);
        assert_eq!(unwrap_or_f(), 5);

        let f2 = || Err::<i32, &str>("failed");
        let unwrap_or_f2 = unwrap_or(f2, 0);
        assert_eq!(unwrap_or_f2(), 0);
    }

    #[test]
    fn test_unwrap_or_else() {
        let f = || Err::<(), &str>("error");
        let unwrap_f = unwrap_or_else(f, |e| e.len());
        assert_eq!(unwrap_f(), 5);
    }
}
