//! Type-level constants and basic types.

/// The uninhabited type - equivalent to C++'s `void` or Rust's `!`.
///
/// This type cannot be instantiated and is used to indicate that
/// a function never returns (diverges).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::Void;
///
/// fn never_returns() -> Void {
///     panic!("This function never returns!");
/// }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Void {}

/// Type-level boolean constant.
///
/// Allows using booleans at the type level for compile-time computation.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_constants::Bool;
///
/// type True = Bool<true>;
/// type False = Bool<false>;
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Bool<const B: bool>;

impl<const B: bool> Bool<B> {
    /// Returns the runtime value of the boolean constant.
    pub const VALUE: bool = B;
}

/// Type-level signed integer constant.
///
/// Allows using integers at the type level for compile-time computation.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_constants::Int;
///
/// type One = Int<1>;
/// type MinusOne = Int<-1>;
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Int<const N: isize>;

impl<const N: isize> Int<N> {
    /// Returns the runtime value of the integer constant.
    pub const VALUE: isize = N;
}

/// Type-level unsigned integer constant.
///
/// Allows using unsigned integers at the type level.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_constants::UInt;
///
/// type One = UInt<1>;
/// type Hundred = UInt<100>;
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct UInt<const N: usize>;

impl<const N: usize> UInt<N> {
    /// Returns the runtime value of the unsigned integer constant.
    pub const VALUE: usize = N;
}

/// A type-level list for compile-time type operations.
///
/// This allows operating on lists of types at compile time.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_constants::TypeList;
///
/// type Numbers = TypeList!(i32, i64, u32, u64);
/// ```
pub struct TypeList<T> {
    _phantom: core::marker::PhantomData<T>,
}

impl<T> TypeList<T> {
    /// Creates a new type list marker.
    pub const fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T> Default for TypeList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Copy for TypeList<T> {}

impl<T> Clone for TypeList<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Macro to create a TypeList from a comma-separated list of types.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::type_constants::TypeList;
///
/// type Numbers = TypeList!(i32, i64, u32, u64);
/// ```
#[macro_export]
macro_rules! TypeList {
    () => {
        $crate::absl_meta::type_constants::TypeList::<()>
    };
    ($($t:ty),+ $(,)?) => {
        $crate::absl_meta::type_constants::TypeList::<($($t),*)>
    };
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_type_list_macro() {
        let _list = TypeList!();
        let _nums: TypeList<(i32, i64)> = TypeList!(i32, i64);
    }
}
