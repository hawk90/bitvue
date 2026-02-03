//! Type-level list operations.

use super::type_constants::TypeList;

/// Type-level list operations: Length.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::{TypeList, TypeListLen};
///
/// type List = TypeList!(i32, i64, u32);
/// // TypeListLen::<List>::VALUE would be 3
/// ```
pub trait TypeListLen {
    /// The number of types in the list.
    const VALUE: usize = 0;
}

impl TypeListLen for TypeList<()> {
    const VALUE: usize = 0;
}

impl TypeListLen for TypeList<(i8,)> {
    const VALUE: usize = 1;
}

impl TypeListLen for TypeList<(i8, i16)> {
    const VALUE: usize = 2;
}

impl TypeListLen for TypeList<(i8, i16, i32)> {
    const VALUE: usize = 3;
}

impl TypeListLen for TypeList<(i8, i16, i32, i64)> {
    const VALUE: usize = 4;
}

impl TypeListLen for TypeList<(i8, i16, i32, i64, i128)> {
    const VALUE: usize = 5;
}

/// Type-level list operations: Head (first element).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::{TypeList, TypeListHead};
///
/// type List = TypeList!(i32, i64, u32);
/// type First = TypeListHead::<List>;
/// // First is i32
/// ```
pub trait TypeListHead {
    /// The first type in the list.
    type Output;
}

impl<T, Rest> TypeListHead for TypeList<(T, Rest)> {
    type Output = T;
}

/// Type-level list operations: Tail (rest of elements).
pub trait TypeListTail {
    /// The rest of the list.
    type Output;
}

impl<T, Rest> TypeListTail for TypeList<(T, Rest)> {
    type Output = Rest;
}
