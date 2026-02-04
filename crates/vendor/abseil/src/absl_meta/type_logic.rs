//! Type-level logical operations and comparisons.

use super::type_constants::{Bool, Int, UInt};

/// Compile-time conditional type selection.
///
/// Selects between two types based on a boolean condition at compile time.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_logic::IfThenElse;
///
/// // Select signed or unsigned based on condition
/// type SignedIf<COND> = IfThenElse<COND, i32, u32>;
/// ```
pub type IfThenElse<Cond, Then, Else> = <Cond as If<Then, Else>>::Output;

/// Trait for compile-time conditional type selection.
pub trait If<Then, Else> {
    /// The selected type based on the condition.
    type Output;
}

// True selects the first type
impl<Then, Else> If<Then, Else> for Bool<true> {
    type Output = Then;
}

// False selects the second type
impl<Then, Else> If<Then, Else> for Bool<false> {
    type Output = Else;
}

/// Type-level negation.
pub trait Not {
    /// The negated value.
    const VALUE: bool;
}

impl<const B: bool> Not for Bool<B> {
    const VALUE: bool = !B;
}

/// Type-level logical AND.
pub trait And<R> {
    /// The result of A AND B.
    const VALUE: bool;
}

impl<const A: bool, const B: bool> And<Bool<B>> for Bool<A> {
    const VALUE: bool = A && B;
}

/// Type-level logical OR.
pub trait Or<R> {
    /// The result of A OR B.
    const VALUE: bool;
}

impl<const A: bool, const B: bool> Or<Bool<B>> for Bool<A> {
    const VALUE: bool = A || B;
}

/// Type-level XOR.
pub trait Xor<R> {
    /// The result of A XOR B.
    const VALUE: bool;
}

impl<const A: bool, const B: bool> Xor<Bool<B>> for Bool<A> {
    const VALUE: bool = A != B;
}

/// Type-level equality comparison.
pub trait Equal<R> {
    /// True if the values are equal.
    const VALUE: bool;
}

impl<const A: isize, const B: isize> Equal<Int<B>> for Int<A> {
    const VALUE: bool = A == B;
}

impl<const A: usize, const B: usize> Equal<UInt<B>> for UInt<A> {
    const VALUE: bool = A == B;
}

impl<const A: bool, const B: bool> Equal<Bool<B>> for Bool<A> {
    const VALUE: bool = A == B;
}

/// Type-level less-than comparison.
pub trait Less<R> {
    /// True if Self < R.
    const VALUE: bool;
}

impl<const A: isize, const B: isize> Less<Int<B>> for Int<A> {
    const VALUE: bool = A < B;
}

impl<const A: usize, const B: usize> Less<UInt<B>> for UInt<A> {
    const VALUE: bool = A < B;
}

/// Type-level greater-than comparison.
pub trait Greater<R> {
    /// True if Self > R.
    const VALUE: bool;
}

impl<const A: isize, const B: isize> Greater<Int<B>> for Int<A> {
    const VALUE: bool = A > B;
}

impl<const A: usize, const B: usize> Greater<UInt<B>> for UInt<A> {
    const VALUE: bool = A > B;
}

/// Trait for computing the minimum of two type-level integers.
pub trait Min<R> {
    /// The minimum of the two values.
    type Output;
}

impl<const A: isize, const B: isize> Min<Int<B>> for Int<A> {
    type Output = Int<{ if A < B { A } else { B } }>;
}

/// Trait for computing the maximum of two type-level integers.
pub trait Max<R> {
    /// The maximum of the two values.
    type Output;
}

impl<const A: isize, const B: isize> Max<Int<B>> for Int<A> {
    type Output = Int<{ if A > B { A } else { B } }>;
}

/// Enable_if utility for conditional compilation.
///
/// This is similar to C++'s `std::enable_if`.
pub type EnableIf<Cond, T> = <Cond as EnableIfImpl<T>>::Output;

pub trait EnableIfImpl<T> {
    type Output;
}

impl<T> EnableIfImpl<T> for Bool<true> {
    type Output = T;
}

/// Conditional type based on a boolean constant.
pub type Conditional<const Cond: bool, Then, Else> =
    IfThenElse<Bool<Cond>, Then, Else>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not() {
        assert_eq!(Not::<Bool<true>>::VALUE, false);
        assert_eq!(Not::<Bool<false>>::VALUE, true);
    }

    #[test]
    fn test_and() {
        assert_eq!(And::<Bool<true>, Bool<true>>::VALUE, true);
        assert_eq!(And::<Bool<true>, Bool<false>>::VALUE, false);
        assert_eq!(And::<Bool<false>, Bool<false>>::VALUE, false);
    }

    #[test]
    fn test_or() {
        assert_eq!(Or::<Bool<true>, Bool<false>>::VALUE, true);
        assert_eq!(Or::<Bool<false>, Bool<false>>::VALUE, false);
        assert_eq!(Or::<Bool<true>, Bool<true>>::VALUE, true);
    }

    #[test]
    fn test_xor() {
        assert_eq!(Xor::<Bool<true>, Bool<false>>::VALUE, true);
        assert_eq!(Xor::<Bool<true>, Bool<true>>::VALUE, false);
        assert_eq!(Xor::<Bool<false>, Bool<false>>::VALUE, false);
    }

    #[test]
    fn test_equal_int() {
        assert_eq!(Equal::<Int<5>, Int<5>>::VALUE, true);
        assert_eq!(Equal::<Int<5>, Int<3>>::VALUE, false);
    }

    #[test]
    fn test_equal_uint() {
        assert_eq!(Equal::<UInt<5>, UInt<5>>::VALUE, true);
        assert_eq!(Equal::<UInt<5>, UInt<3>>::VALUE, false);
    }

    #[test]
    fn test_equal_bool() {
        assert_eq!(Equal::<Bool<true>, Bool<true>>::VALUE, true);
        assert_eq!(Equal::<Bool<true>, Bool<false>>::VALUE, false);
    }

    #[test]
    fn test_less_int() {
        assert_eq!(Less::<Int<3>, Int<5>>::VALUE, true);
        assert_eq!(Less::<Int<5>, Int<3>>::VALUE, false);
        assert_eq!(Less::<Int<5>, Int<5>>::VALUE, false);
    }

    #[test]
    fn test_less_uint() {
        assert_eq!(Less::<UInt<3>, UInt<5>>::VALUE, true);
        assert_eq!(Less::<UInt<5>, UInt<3>>::VALUE, false);
    }

    #[test]
    fn test_greater_int() {
        assert_eq!(Greater::<Int<5>, Int<3>>::VALUE, true);
        assert_eq!(Greater::<Int<3>, Int<5>>::VALUE, false);
        assert_eq!(Greater::<Int<5>, Int<5>>::VALUE, false);
    }

    #[test]
    fn test_greater_uint() {
        assert_eq!(Greater::<UInt<5>, UInt<3>>::VALUE, true);
        assert_eq!(Greater::<UInt<3>, UInt<5>>::VALUE, false);
    }
}
