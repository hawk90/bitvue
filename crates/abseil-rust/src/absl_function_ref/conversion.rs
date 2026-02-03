//! Conversion utilities - to_fn, to_fn_mut

/// Converts a function pointer to a trait object.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::to_fn;
///
/// fn add_one(x: i32) -> i32 { x + 1 }
/// let f: Box<dyn Fn(i32) -> i32> = to_fn(add_one);
/// assert_eq!(f(5), 6);
/// ```
#[inline]
pub fn to_fn<A, B, F>(f: F) -> Box<dyn Fn(A) -> B>
where
    F: Fn(A) -> B + 'static,
{
    Box::new(f)
}

/// Converts a mutable function pointer to a trait object.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_function_ref::to_fn_mut;
///
/// let mut counter = 0;
/// let mut inc = move |x: i32| { counter += x; counter };
/// let mut f: Box<dyn FnMut(i32) -> i32> = to_fn_mut(inc);
/// assert_eq!(f(5), 5);
/// assert_eq!(f(3), 8);
/// ```
#[inline]
pub fn to_fn_mut<A, B, F>(f: F) -> Box<dyn FnMut(A) -> B>
where
    F: FnMut(A) -> B + 'static,
{
    Box::new(f)
}
