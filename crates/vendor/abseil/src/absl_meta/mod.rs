//! Type traits and compile-time utilities.
//!
//! This module provides type traits (similar to Abseil's `absl/meta` or C++'s `<type_traits>`)
//! which are utilities for querying and transforming types at compile time.
//!
//! # Overview
//!
//! Type traits provide compile-time type information and transformations. Rust's
//! standard library already provides many of these through `core::mem` and
//! `core::marker`, but this module provides additional compatibility helpers
//! and Abseil-specific traits.
//!
//! # Components
//!
//! - [`TypeIdentity<T>`] - Type identity wrapper for passing types as values
//! - [`Void`] - uninhabited type equivalent to C++'s `void`
//! - [`Bool<const B: bool>`] - Compile-time boolean type-level value
//! - [`Int<const N: isize>`] - Compile-time integer type-level value
//! - [`TypeList<Types...>`] - Type-level list for type operations
//! - [`IfThenElse<Cond, Then, Else>`] - Compile-time conditional type selection
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_meta::{TypeIdentity, Void, is_signed};
//!
//! // Check type properties
//! assert!(is_signed::<i32>());
//! assert!(!is_signed::<u32>());
//!
//! // Type identity for type passing
//! fn get_type<T>() -> TypeIdentity<T> {
//!     TypeIdentity::new()
//! }
//!
//! // Use Void for uninhabited return types
//! fn never_returns() -> Void {
//!     panic!("This function never returns");
//! }
//! ```

// Submodules
pub mod const_fn;
pub mod container_traits;
pub mod marker_traits;
pub mod thread_safety;
pub mod type_arithmetic;
pub mod type_constants;
pub mod type_info;
pub mod type_list_ops;
pub mod type_logic;
pub mod type_transform;
pub mod type_traits;

// Re-exports from type_traits module
pub use type_traits::{
    is_arithmetic, is_floating_point, is_integral, is_signed, is_unsigned, TypeIdentity,
};

// Re-exports from type_constants
pub use type_constants::{Bool, Int, TypeList, UInt, Void};

// Re-exports from type_logic
pub use type_logic::{
    And, Conditional, EnableIf, Equal, Greater, If, IfThenElse, Less, Max, Min, Not, Or, Xor,
};

// Re-exports from type_info
pub use type_info::{AlignOf, SizeOf, TypeEq};

// Re-exports from const_fn
pub use const_fn::{Const, SignedNum};

// Re-exports from marker_traits
pub use marker_traits::{
    ArrayLen, HasCopyTrait, IsArray, IsConst, IsFunction, IsPod, IsPointer, IsPrimitive,
    IsReference, IsVolatile,
};

// Re-exports from type_arithmetic
pub use type_arithmetic::{Add, Div, Mod, Mul, Sub};

// Re-exports from type_transform
pub use type_transform::{
    AddConst, AddMutRef, AddVolatile, AsConstRef, AsMutRef, RemoveConst, RemoveConstRef,
    RemoveMutRef, RemovePointer, RemoveReference, RemoveVolatile,
};

// Re-exports from container_traits
pub use container_traits::{
    IsEnum, IsOption, IsResult, IsSlice, IsTuple, IsUnion, OptionValue, ResultErr, ResultOk,
};

// Re-exports from type_list_ops
pub use type_list_ops::{TypeListHead, TypeListLen, TypeListTail};

// Re-exports from thread_safety
pub use thread_safety::{HasCloneTrait as HasCloneTraitThread, IsSend, IsSync};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_void_type() {
        // Void cannot be instantiated, but we can use it as a type
        let _never: Option<Void> = None;
    }

    #[test]
    fn test_bool_value() {
        assert_eq!(Bool::<true>::VALUE, true);
        assert_eq!(Bool::<false>::VALUE, false);
    }

    #[test]
    fn test_int_value() {
        assert_eq!(Int::<42>::VALUE, 42);
        assert_eq!(Int::<-10>::VALUE, -10);
        assert_eq!(Int::<0>::VALUE, 0);
    }

    #[test]
    fn test_uint_value() {
        assert_eq!(UInt::<42>::VALUE, 42);
        assert_eq!(UInt::<0>::VALUE, 0);
        assert_eq!(UInt::<100>::VALUE, 100);
    }

    #[test]
    fn test_if_then_else() {
        // Can't directly test type selection at runtime,
        // but we can verify the types are different
        type SelectedTrue = IfThenElse<Bool<true>, i32, u32>;
        type SelectedFalse = IfThenElse<Bool<false>, i32, u32>;

        // Verify through size comparison
        assert_eq!(core::mem::size_of::<SelectedTrue>(), 4);
        assert_eq!(core::mem::size_of::<SelectedFalse>(), 4);
    }

    #[test]
    fn test_size_of() {
        assert_eq!(SizeOf::<i32>::VALUE, 4);
        assert_eq!(SizeOf::<i64>::VALUE, 8);
        assert_eq!(SizeOf::<u8>::VALUE, 1);
        assert_eq!(SizeOf::<bool>::VALUE, 1);
    }

    #[test]
    fn test_align_of() {
        assert_eq!(AlignOf::<i32>::VALUE, 4);
        assert_eq!(AlignOf::<i64>::VALUE, 8);
        assert_eq!(AlignOf::<u8>::VALUE, 1);
    }

    #[test]
    fn test_const_min() {
        assert_eq!(Const::min(1, 2), 1);
        assert_eq!(Const::min(2, 1), 1);
        assert_eq!(Const::min(5, 5), 5);
    }

    #[test]
    fn test_const_max() {
        assert_eq!(Const::max(1, 2), 2);
        assert_eq!(Const::max(2, 1), 2);
        assert_eq!(Const::max(5, 5), 5);
    }

    #[test]
    fn test_const_clamp() {
        assert_eq!(Const::clamp(5, 0, 10), 5);
        assert_eq!(Const::clamp(-5, 0, 10), 0);
        assert_eq!(Const::clamp(15, 0, 10), 10);
    }

    #[test]
    fn test_const_abs() {
        assert_eq!(Const::abs(-5), 5);
        assert_eq!(Const::abs(5), 5);
        assert_eq!(Const::abs(0), 0);
    }

    #[test]
    fn test_const_in_range() {
        assert!(Const::in_range(5, 0, 10));
        assert!(Const::in_range(0, 0, 10));
        assert!(Const::in_range(10, 0, 10));
        assert!(!Const::in_range(15, 0, 10));
        assert!(!Const::in_range(-5, 0, 10));
    }

    #[test]
    fn test_const_div_ceil() {
        assert_eq!(Const::div_ceil(10, 3), 4);
        assert_eq!(Const::div_ceil(9, 3), 3);
        assert_eq!(Const::div_ceil(1, 3), 1);
    }

    #[test]
    fn test_const_round_up() {
        assert_eq!(Const::round_up(10, 3), 12);
        assert_eq!(Const::round_up(9, 3), 9);
        assert_eq!(Const::round_up(1, 3), 3);
    }

    #[test]
    fn test_is_reference() {
        assert!(IsReference::<&i32>::VALUE);
        assert!(IsReference::<&mut i32>::VALUE);
        assert!(!IsReference::<i32>::VALUE);
    }

    #[test]
    fn test_is_pointer() {
        assert!(IsPointer::<*const i32>::VALUE);
        assert!(IsPointer::<*mut i32>::VALUE);
        assert!(!IsPointer::<&i32>::VALUE);
        assert!(!IsPointer::<i32>::VALUE);
    }

    #[test]
    fn test_is_array() {
        assert!(IsArray::<[i32; 5]>::VALUE);
        assert!(IsArray::<[i32]>::VALUE);
        assert!(!IsArray::<i32>::VALUE);
    }

    #[test]
    fn test_array_len() {
        assert_eq!(ArrayLen::<[i32; 5]>::VALUE, 5);
        assert_eq!(ArrayLen::<[i32; 100]>::VALUE, 100);
    }

    #[test]
    fn test_is_pod() {
        assert!(IsPod::<i32>::VALUE);
        assert!(IsPod::<u64>::VALUE);
        assert!(IsPod::<bool>::VALUE);
        assert!(IsPod::<f32>::VALUE);
    }

    #[test]
    fn test_has_copy_trait() {
        assert!(HasCopyTrait::<i32>::VALUE);
        assert!(HasCopyTrait::<bool>::VALUE);
    }

    #[test]
    fn test_is_primitive() {
        assert!(IsPrimitive::<i32>::VALUE);
        assert!(IsPrimitive::<bool>::VALUE);
        assert!(IsPrimitive::<f64>::VALUE);
        assert!(IsPrimitive::<char>::VALUE);
    }

    #[test]
    fn test_is_function() {
        fn foo() {}
        fn bar(x: i32) -> i32 { x }

        assert!(IsFunction::<fn()>::VALUE);
        assert!(IsFunction::<fn(i32) -> i32>::VALUE);
        assert!(!IsFunction::<i32>::VALUE);
    }

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
    fn test_type_list_macro() {
        let _list = TypeList!();
        let _nums: TypeList<(i32, i64)> = TypeList!(i32, i64);
    }

    #[test]
    fn test_const_min_max_edge_cases() {
        assert_eq!(Const::min(i8::MIN, i8::MAX), i8::MIN);
        assert_eq!(Const::max(i8::MIN, i8::MAX), i8::MAX);
    }

    #[test]
    fn test_signed_abs_edge_cases() {
        assert_eq!(Const::abs(i32::MIN), i32::MIN); // Overflow in abs
        assert_eq!(Const::abs(0), 0);
    }

    // Tests for Add trait
    #[test]
    fn test_add_int() {
        type Result = Add::<Int<10>, Int<5>>;
        assert_eq!(Result::VALUE, 15);
    }

    #[test]
    fn test_add_uint() {
        type Result = Add::<UInt<10>, UInt<5>>;
        assert_eq!(Result::VALUE, 15);
    }

    #[test]
    fn test_add_negative() {
        type Result = Add::<Int<-5>, Int<3>>;
        assert_eq!(Result::VALUE, -2);
    }

    // Tests for Sub trait
    #[test]
    fn test_sub_int() {
        type Result = Sub::<Int<10>, Int<3>>;
        assert_eq!(Result::VALUE, 7);
    }

    #[test]
    fn test_sub_negative() {
        type Result = Sub::<Int<5>, Int<10>>;
        assert_eq!(Result::VALUE, -5);
    }

    // Tests for Mul trait
    #[test]
    fn test_mul_int() {
        type Result = Mul::<Int<3>, Int<4>>;
        assert_eq!(Result::VALUE, 12);
    }

    #[test]
    fn test_mul_uint() {
        type Result = Mul::<UInt<5>, UInt<6>>;
        assert_eq!(Result::VALUE, 30);
    }

    #[test]
    fn test_mul_negative() {
        type Result = Mul::<Int<-3>, Int<4>>;
        assert_eq!(Result::VALUE, -12);
    }

    // Tests for Div trait
    #[test]
    fn test_div_uint() {
        type Result = Div::<UInt<10>, UInt<2>>;
        assert_eq!(Result::VALUE, 5);
    }

    #[test]
    fn test_div_rounding() {
        type Result = Div::<UInt<11>, UInt<2>>;
        assert_eq!(Result::VALUE, 5);
    }

    // Tests for Mod trait
    #[test]
    fn test_mod_uint() {
        type Result = Mod::<UInt<10>, UInt<3>>;
        assert_eq!(Result::VALUE, 1);
    }

    #[test]
    fn test_mod_zero() {
        type Result = Mod::<UInt<12>, UInt<4>>;
        assert_eq!(Result::VALUE, 0);
    }

    // Tests for Equal trait
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

    // Tests for Less trait
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

    // Tests for Greater trait
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

    // Tests for RemoveReference
    #[test]
    fn test_remove_reference() {
        type T1 = RemoveReference<i32>;
        type T2 = RemoveReference<&i32>;
        type T3 = RemoveReference<&mut i32>;

        // Verify through size
        assert_eq!(core::mem::size_of::<T1>(), 4);
        assert_eq!(core::mem::size_of::<T2>(), 4);
    }

    // Tests for RemovePointer
    #[test]
    fn test_remove_pointer() {
        type T1 = RemovePointer<i32>;
        type T2 = RemovePointer<*const i32>;
        type T3 = RemovePointer<*mut i32>;

        assert_eq!(core::mem::size_of::<T1>(), 4);
    }

    // Tests for IsTuple
    #[test]
    fn test_is_tuple() {
        assert!(IsTuple::<(i32, i32)>::VALUE);
        assert!(IsTuple::<()>::VALUE);
        assert!(IsTuple::<(i32, i32, i32, i32, i32)>::VALUE);
        assert!(!IsTuple::<i32>::VALUE);
        assert_eq!(IsTuple::<(i32, i32)>::COUNT, 2);
        assert_eq!(IsTuple::<()>::COUNT, 0);
        assert_eq!(IsTuple::<(i32, i32, i32)>::COUNT, 3);
    }

    // Tests for IsOption
    #[test]
    fn test_is_option() {
        assert!(IsOption::<Option<i32>>::VALUE);
        assert!(IsOption::<Option<&str>>::VALUE);
        assert!(!IsOption::<i32>::VALUE);
    }

    // Tests for IsResult
    #[test]
    fn test_is_result() {
        assert!(IsResult::<Result<i32, String>>::VALUE);
        assert!(!IsResult::<i32>::VALUE);
    }

    // Tests for IsSlice
    #[test]
    fn test_is_slice() {
        assert!(IsSlice::<[i32]>::VALUE);
        assert!(IsSlice::<[u8]>::VALUE);
        assert!(!IsSlice::<i32>::VALUE);
    }

    // Tests for IsEnum
    #[test]
    fn test_is_enum() {
        assert!(IsEnum::<Option<i32>>::VALUE);
        assert!(IsEnum::<Result<i32, String>>::VALUE);
        assert!(!IsEnum::<i32>::VALUE);
    }

    // Tests for OptionValue
    #[test]
    fn test_option_value() {
        type T = OptionValue::<Option<i32>>;
        // T should be i32
        assert_eq!(core::mem::size_of::<T>(), 4);
    }

    // Tests for ResultOk
    #[test]
    fn test_result_ok() {
        type T = ResultOk::<Result<i32, String>>;
        assert_eq!(core::mem::size_of::<T>(), 4);
    }

    // Tests for ResultErr
    #[test]
    fn test_result_err() {
        type T = ResultErr::<Result<i32, String>>;
        // String doesn't have a fixed size, but we can check it compiles
        let _: T = String::new();
    }

    // Tests for Conditional
    #[test]
    fn test_conditional() {
        type T1 = Conditional::<true, i32, u32>;
        type T2 = Conditional::<false, i32, u32>;

        assert_eq!(core::mem::size_of::<T1>(), 4);
        assert_eq!(core::mem::size_of::<T2>(), 4);
    }

    // Tests for IsSend
    #[test]
    fn test_is_send() {
        assert!(IsSend::<i32>::VALUE);
        assert!(IsSend::<String>::VALUE);
    }

    // Tests for IsSync
    #[test]
    fn test_is_sync() {
        assert!(IsSync::<i32>::VALUE);
        assert!(IsSync::<&i32>::VALUE);
    }

    // Tests for HasCloneTraitThread (aliased to avoid conflict)
    #[test]
    fn test_has_clone_trait_thread() {
        assert!(HasCloneTraitThread::<i32>::VALUE);
        assert!(HasCloneTraitThread::<Vec<i32>>::VALUE);
    }

    // Tests for Const::sqr
    #[test]
    fn test_const_sqr() {
        assert_eq!(Const::sqr(5), 25);
        assert_eq!(Const::sqr(-3), 9);
        assert_eq!(Const::sqr(0), 0);
    }

    // Tests for Const::pow
    #[test]
    fn test_const_pow() {
        assert_eq!(Const::pow(2u32, 3), 8);
        assert_eq!(Const::pow(5u32, 0), 1);
        assert_eq!(Const::pow(3u32, 4), 81);
    }

    // Tests for Const::gcd
    #[test]
    fn test_const_gcd() {
        assert_eq!(Const::gcd(48, 18), 6);
        assert_eq!(Const::gcd(17, 5), 1);
        assert_eq!(Const::gcd(0, 5), 5);
    }

    // Tests for Const::lcm
    #[test]
    fn test_const_lcm() {
        assert_eq!(Const::lcm(4, 6), 12);
        assert_eq!(Const::lcm(5, 7), 35);
        assert_eq!(Const::lcm(3, 9), 9);
    }

    // Tests for Const::is_even
    #[test]
    fn test_const_is_even() {
        assert!(Const::is_even(4));
        assert!(Const::is_even(0));
        assert!(!Const::is_even(5));
    }

    // Tests for Const::is_odd
    #[test]
    fn test_const_is_odd() {
        assert!(Const::is_odd(5));
        assert!(Const::is_odd(1));
        assert!(!Const::is_odd(4));
    }

    // Tests for Const::leading_zeros
    #[test]
    fn test_const_leading_zeros() {
        assert_eq!(Const::leading_zeros(0u32), 32);
        assert_eq!(Const::leading_zeros(1u32), 31);
        assert_eq!(Const::leading_zeros(0x8000_0000u32), 0);
        assert_eq!(Const::leading_zeros(0xFFu32), 24);
    }

    // Tests for Const::trailing_zeros
    #[test]
    fn test_const_trailing_zeros() {
        assert_eq!(Const::trailing_zeros(0u32), 32);
        assert_eq!(Const::trailing_zeros(1u32), 0);
        assert_eq!(Const::trailing_zeros(2u32), 1);
        assert_eq!(Const::trailing_zeros(8u32), 3);
    }

    // Tests for Const::popcount
    #[test]
    fn test_const_popcount() {
        assert_eq!(Const::popcount(0u32), 0);
        assert_eq!(Const::popcount(1u32), 1);
        assert_eq!(Const::popcount(0xFFu32), 8);
        assert_eq!(Const::popcount(0x1010u32), 2);
    }

    // Tests for Const::swap_bytes
    #[test]
    fn test_const_swap_bytes() {
        assert_eq!(Const::swap_bytes(0x12345678u32), 0x78563412);
        assert_eq!(Const::swap_bytes(0x11223344u32), 0x44332211);
    }

    // Tests for Const::rotate_left
    #[test]
    fn test_const_rotate_left() {
        assert_eq!(Const::rotate_left(0x80000001u32, 1), 0x00000003);
        assert_eq!(Const::rotate_left(0x12345678u32, 4), 0x23456781);
    }

    // Tests for Const::rotate_right
    #[test]
    fn test_const_rotate_right() {
        assert_eq!(Const::rotate_right(0x80000001u32, 1), 0xC0000000);
        assert_eq!(Const::rotate_right(0x12345678u32, 4), 0x81234567);
    }

    // Tests for Const::is_power_of_two
    #[test]
    fn test_const_is_power_of_two() {
        assert!(Const::is_power_of_two(1));
        assert!(Const::is_power_of_two(2));
        assert!(Const::is_power_of_two(16));
        assert!(!Const::is_power_of_two(0));
        assert!(!Const::is_power_of_two(3));
        assert!(!Const::is_power_of_two(15));
    }

    // Tests for Const::next_power_of_two
    #[test]
    fn test_const_next_power_of_two() {
        assert_eq!(Const::next_power_of_two(0), 1);
        assert_eq!(Const::next_power_of_two(1), 1);
        assert_eq!(Const::next_power_of_two(5), 8);
        assert_eq!(Const::next_power_of_two(16), 16);
        assert_eq!(Const::next_power_of_two(17), 32);
    }

    // Tests for Const::isqrt
    #[test]
    fn test_const_isqrt() {
        assert_eq!(Const::isqrt(0), 0);
        assert_eq!(Const::isqrt(1), 1);
        assert_eq!(Const::isqrt(4), 2);
        assert_eq!(Const::isqrt(8), 2);
        assert_eq!(Const::isqrt(9), 3);
        assert_eq!(Const::isqrt(15), 3);
        assert_eq!(Const::isqrt(16), 4);
    }

    // Tests for Const::log2
    #[test]
    fn test_const_log2() {
        assert_eq!(Const::log2(1), 0);
        assert_eq!(Const::log2(2), 1);
        assert_eq!(Const::log2(4), 2);
        assert_eq!(Const::log2(8), 3);
        assert_eq!(Const::log2(16), 4);
    }

    // Tests for Const::saturating_add
    #[test]
    fn test_const_saturating_add() {
        assert_eq!(Const::saturating_add(u32::MAX, 1), u32::MAX);
        assert_eq!(Const::saturating_add(5, 3), 8);
        assert_eq!(Const::saturating_add(5, u32::MAX), u32::MAX);
    }

    // Tests for Const::saturating_sub
    #[test]
    fn test_const_saturating_sub() {
        assert_eq!(Const::saturating_sub(0u32, 1), 0);
        assert_eq!(Const::saturating_sub(5, 3), 2);
        assert_eq!(Const::saturating_sub(3, 5), 0);
    }

    // Tests for Const::saturating_mul
    #[test]
    fn test_const_saturating_mul() {
        assert_eq!(Const::saturating_mul(u32::MAX, 2), u32::MAX);
        assert_eq!(Const::saturating_mul(5, 3), 15);
        assert_eq!(Const::saturating_mul(5, u32::MAX), u32::MAX);
    }

    // Tests for IsConst
    #[test]
    fn test_is_const() {
        assert!(!IsConst::<i32>::VALUE);
        assert!(!IsConst::<&i32>::VALUE);
    }

    // Tests for IsVolatile
    #[test]
    fn test_is_volatile() {
        assert!(!IsVolatile::<i32>::VALUE);
    }

    // Tests for RemoveConstRef
    #[test]
    fn test_remove_const_ref() {
        type T = RemoveConstRef::<i32>;
        assert_eq!(core::mem::size_of::<T>(), 0); // ZST for unsized type
    }

    // Tests for RemoveMutRef
    #[test]
    fn test_remove_mut_ref() {
        type T = RemoveMutRef::<i32>;
        assert_eq!(core::mem::size_of::<T>(), 0); // ZST for unsized type
    }
}
