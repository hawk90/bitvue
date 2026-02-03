//! Function chaining utilities - chain, apply_chain

/// Chains multiple functions together.
///
/// Each function receives the result of the previous one.
#[inline]
pub fn chain<A, B>(start: A, functions: &[fn(A) -> B]) -> Vec<B>
where
    B: Clone,
{
    functions.iter().map(|f| f(start)).collect()
}

/// Applies a sequence of functions to a value.
#[inline]
pub fn apply_chain<A>(mut value: A, functions: &[fn(&mut A)]) -> A {
    for f in functions {
        f(&mut value);
    }
    value
}
