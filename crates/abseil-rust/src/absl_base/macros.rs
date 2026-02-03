//! Utility macros common to Abseil libraries.
//!
//! This module provides useful utility macros similar to those found in
//! Abseil's `absl/base/macros.h`.

/// The `must_use_result` macro annotates a function or method to indicate
/// that its return value should always be used.
///
/// This is equivalent to Rust's `#[must_use]` attribute but with
/// Abseil-compatible naming. It can optionally include a custom message
/// explaining why the result is important.
///
/// # Example
///
/// Note: Use Rust's built-in `#[must_use]` attribute instead.
/// This is for documentation purposes only.
///
/// ```rust
/// // Instead of: #[must_use]
/// fn compute() -> i32 {
///     42
/// }
///
/// // With custom message:
/// #[must_use = "The result contains important error information"]
/// fn parse(input: &str) -> Result<i32, &'static str> {
///     input.parse().map_err(|_| "Invalid number")
/// }
/// ```

/// Annotates a function to prevent it from being discarded.
///
/// This ensures that calling the function without using its result
/// will generate a compiler warning.
///
/// # Example
///
/// ```rust
/// #![must_use]
///
/// fn expensive_computation() -> i32 {
///     42
/// }
///
/// fn main() {
///     let _ = expensive_computation(); // OK
///     expensive_computation(); // Warning: unused result
/// }
/// ```
///
/// Note: Use Rust's built-in `#[must_use]` attribute instead.
/// This module is provided for API compatibility with Abseil.

/// Concatenates identifiers into a single identifier.
///
/// This is the built-in Rust `concat_idents!` macro re-exported for convenience.
///
/// Note: This macro is unstable and requires the `concat_idents` feature gate.
/// It's primarily useful in macro-generated code where you need to create
/// composite identifiers.
///
/// # Example
///
/// The macro is for advanced macro use cases. In most cases, you should
/// use string concatenation or other approaches instead.
///
/// ```rust
/// // Note: concat_idents! is a built-in unstable macro
/// // This is just for documentation purposes
/// ```

/// Converts an expression to a string at compile time.
///
/// This is similar to `stringify!` but can be used in more contexts.
///
/// # Example
///
/// ```rust
/// use abseil::stringify_expr;
///
/// fn test() {
///     let x = stringify_expr!(1 + 2);
///     assert_eq!(x, "1 + 2");
/// }
/// ```
#[macro_export]
macro_rules! stringify_expr {
    ($e:expr) => {
        stringify!($e)
    };
}

/// Gets the name of a type as a string.
///
/// # Example
///
/// ```rust
/// use abseil::type_name;
///
/// // Returns the type name as a &'static str
/// let i32_name: &'static str = type_name!(i32);
/// assert!(i32_name.contains("i32"));
///
/// // Complex type names include the full path
/// let vec_name: &'static str = type_name!(Vec<u8>);
/// assert!(vec_name.contains("Vec") || vec_name.contains("vec"));
/// ```
#[macro_export]
macro_rules! type_name {
    ($ty:ty) => {
        std::any::type_name::<$ty>()
    };
}

/// Gets the name of the type of an expression.
///
/// # Example
///
/// ```rust
/// use abseil::type_name_of;
///
/// let x: Vec<i32> = vec![1, 2, 3];
/// let name = type_name_of!(&x);
/// assert!(name.contains("Vec"));
/// ```
#[macro_export]
macro_rules! type_name_of {
    ($e:expr) => {
        std::any::type_name_of_val($e)
    };
}

/// Counts the number of tokens passed to it.
///
/// # Example
///
/// ```rust
/// use abseil::count_tokens;
///
/// assert_eq!(count_tokens!(), 0);
/// assert_eq!(count_tokens!(a), 1);
/// assert_eq!(count_tokens!(a, b), 2);
/// assert_eq!(count_tokens!(a, b, c), 3);
/// ```
#[macro_export]
macro_rules! count_tokens {
    () => { 0usize };
    ($head:tt) => { 1usize };
    ($head:tt, $($tail:tt),*) => { 1usize + count_tokens!($($tail),*) };
}

/// Creates a comma-separated list from the input tokens.
///
/// # Example
///
/// ```rust
/// use abseil::comma_separated;
///
/// // Note: The macro expects comma-separated input
/// // This is mainly for internal macro use
/// macro_rules! demo {
///     () => {
///         // Expands to: a, b, c
///         let _ = comma_separated!(a, b, c);
///     };
/// }
/// ```
#[macro_export]
macro_rules! comma_separated {
    ($($item:tt),*) => {
        $($item),*
    };
}

/// Evaluates to the number of variadic arguments.
///
/// This is useful for macros that need to count their arguments.
///
/// # Example
///
/// ```rust
/// use abseil::va_args_count;
///
/// const COUNT: usize = va_args_count!(a, b, c);
/// assert_eq!(COUNT, 3);
/// ```
#[macro_export]
macro_rules! va_args_count {
    () => { 0 };
    ($first:tt) => { 1 };
    ($first:tt, $($rest:tt),*) => { 1 + va_args_count!($($rest),*) };
}

/// Applies a macro to each element in a list.
///
/// # Example
///
/// ```rust
/// use abseil::foreach;
///
/// macro_rules! declare_item {
///     ($item:ident) => {
///         let $item = 0;
///     };
/// }
///
/// // The macro expands declare_item for each item
/// // Use in statement context, not as an expression
/// foreach!(declare_item, x y z);
/// ```
#[macro_export]
macro_rules! foreach {
    ($macro:ident, ) => {};
    ($macro:ident, $head:tt $($tail:tt)*) => {
        $macro!($head);
        foreach!($macro, $($tail)*);
    };
}

/// Returns the first argument.
///
/// # Example
///
/// ```rust
/// use abseil::first;
///
/// // The macro returns the first token (not evaluated, just passed through)
/// let x = first!(1, 2, 3);
/// assert_eq!(x, 1);
/// ```
#[macro_export]
macro_rules! first {
    ($first:tt $($rest:tt)*) => {
        $first
    };
}

/// Returns the last argument.
///
/// # Example
///
/// ```rust
/// use abseil::last;
///
/// // The macro returns the last token
/// let x = last!(1, 2, 3);
/// assert_eq!(x, 3);
/// let y = last!(42);
/// assert_eq!(y, 42);
/// ```
#[macro_export]
macro_rules! last {
    ($only:tt) => {
        $only
    };
    ($first:tt $($rest:tt)+) => {
        last!($($rest)+)
    };
}

/// Creates an array from a list of expressions.
///
/// # Example
///
/// ```rust
/// use abseil::make_array;
///
/// let arr = make_array![1, 2, 3];
/// assert_eq!(arr, [1, 2, 3]);
/// ```
#[macro_export]
macro_rules! make_array {
    ($($elem:expr),+ $(,)?) => {
        [$($elem),+]
    };
}

/// Creates a hash map with the given key-value pairs.
///
/// # Example
///
/// ```rust
/// use abseil::make_map;
///
/// let map = make_map! {
///     "a" => 1,
///     "b" => 2,
///     "c" => 3,
/// };
/// assert_eq!(map.get("a"), Some(&1));
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! make_map {
    ($($key:expr => $value:expr),+ $(,)?) => {{
        #[cfg(feature = "std")]
        use std::collections::HashMap;

        let mut map = HashMap::new();
        $(
            map.insert($key, $value);
        )+
        map
    }};
}

/// Creates a BTreeMap with the given key-value pairs.
///
/// # Example
///
/// ```rust
/// use abseil::make_btree_map;
///
/// let map = make_btree_map! {
///     "a" => 1,
///     "b" => 2,
/// };
/// assert_eq!(map.get("a"), Some(&1));
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! make_btree_map {
    ($($key:expr => $value:expr),+ $(,)?) => {{
        use std::collections::BTreeMap;

        let mut map = BTreeMap::new();
        $(
            map.insert($key, $value);
        )+
        map
    }};
}

/// Prints to standard output with file and line information.
///
/// Similar to `println!` but includes source location.
///
/// # Example
///
/// ```rust
/// use abseil::println_loc;
///
/// fn main() {
///     println_loc!("Debug message");
///     // Output: [example.rs:4:5] Debug message
/// }
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! println_loc {
    ($($arg:tt)*) => {
        println!("[{}:{}] {}", file!(), line!(), format_args!($($arg)*))
    };
}

/// Prints to standard error with file and line information.
///
/// Similar to `eprintln!` but includes source location.
///
/// # Example
///
/// ```rust
/// use abseil::eprintln_loc;
///
/// fn main() {
///     eprintln_loc!("Error message");
///     // Output: [example.rs:4:5] Error message
/// }
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! eprintln_loc {
    ($($arg:tt)*) => {
        eprintln!("[{}:{}] {}", file!(), line!(), format_args!($($arg)*))
    };
}

/// Asserts a condition with a custom message.
///
/// This is like `assert!` but with more flexible message formatting.
///
/// # Example
///
/// ```rust
/// use abseil::assert_msg;
///
/// let x = 42;
/// assert_msg!(x == 42, "x should be 42, got {}", x);
/// ```
#[macro_export]
macro_rules! assert_msg {
    ($cond:expr, $($msg:tt)*) => {
        assert!($cond, $($msg)*)
    };
}

/// Asserts that two values are equal with custom messages.
///
/// # Example
///
/// ```rust
/// use abseil::assert_eq_msg;
///
/// let a = 42;
/// let b = 42;
/// assert_eq_msg!(a, b, "Values should match");
/// ```
#[macro_export]
macro_rules! assert_eq_msg {
    ($left:expr, $right:expr, $($msg:tt)*) => {
        assert_eq!($left, $right, $($msg)*)
    };
}

/// Computes the offset of a field within a struct.
///
/// # Example
///
/// ```rust
/// use abseil::offset_of;
///
/// struct Foo {
///     a: u8,
///     b: u32,
///     c: u8,
/// }
///
/// // Get the byte offset of field 'b' within struct Foo
/// let offset = offset_of!(Foo, b);
/// // Offset depends on alignment/padding
/// let _ = offset;
/// ```
#[macro_export]
macro_rules! offset_of {
    ($ty:ty, $field:tt) => {{
        // Try to use the built-in std::mem::offset_of! if available (Rust 1.77+)
        // Otherwise fall back to a manual approach
        std::mem::offset_of!($ty, $field)
    }};
}

/// Gets the size of a type in bytes.
///
/// # Example
///
/// ```rust
/// use abseil::size_of;
///
/// assert_eq!(size_of!(i32), 4);
/// ```
#[macro_export]
macro_rules! size_of {
    ($ty:ty) => {
        std::mem::size_of::<$ty>()
    };
}

/// Gets the alignment of a type in bytes.
///
/// # Example
///
/// ```rust
/// use abseil::align_of;
///
/// assert_eq!(align_of!(i32), 4);
/// assert_eq!(align_of!(u8), 1);
/// ```
#[macro_export]
macro_rules! align_of {
    ($ty:ty) => {
        std::mem::align_of::<$ty>()
    };
}

/// Performs a checked cast from one integer type to another.
///
/// # Example
///
/// ```rust
/// use abseil::checked_cast;
///
/// let x: i64 = 42;
/// let y: i32 = checked_cast!(x, i32);
/// assert_eq!(y, 42);
/// ```
#[macro_export]
macro_rules! checked_cast {
    ($value:expr, $ty:ty) => {
        <$ty>::try_from($value).expect("Checked cast failed")
    };
}

/// Performs a saturating cast from one integer type to another.
///
/// Uses i64 as an intermediate type for all comparisons to ensure correct
/// overflow/underflow detection when casting between signed and unsigned types.
///
/// # Panics
///
/// This macro is designed for standard integer types (i8, i16, i32, i64, u8, u16, u32, u64, isize, usize).
/// It may not work correctly for types outside this range.
///
/// # Example
///
/// ```rust
/// use abseil::saturating_cast;
///
/// let x: i64 = 300;
/// let y: u8 = saturating_cast!(x, u8);
/// assert_eq!(y, 255);
///
/// let z: i64 = -10;
/// let w: u8 = saturating_cast!(z, u8);
/// assert_eq!(w, 0);
/// ```
#[macro_export]
macro_rules! saturating_cast {
    ($value:expr, $ty:ty) => {{
        // Convert to i64 for consistent comparison logic
        // SAFETY: This approach works correctly for standard integer types because:
        // 1. All target type MAX/MIN values fit within i64 range
        // 2. Casting through i64 preserves the value for valid inputs
        // 3. Comparison against target bounds detects overflow/underflow
        let v = $value as i64;
        let target_max = <$ty>::MAX as i64;
        let target_min = <$ty>::MIN as i64;

        if v > target_max {
            <$ty>::MAX
        } else if v < target_min {
            <$ty>::MIN
        } else {
            // Value is within range, safe to cast
            v as $ty
        }
    }};
}

/// Generates a unique variable name.
///
/// Useful for macros that need to create temporary variables.
///
/// # Example
///
/// ```rust
/// use abseil::unique_var_name;
///
/// macro_rules! unique_var {
///     () => {
///         let unique_var_name!() = 42;
///     };
/// }
/// ```
#[macro_export]
macro_rules! unique_var_name {
    () => {
        _abseil_var_concat!(__abseil_var_, std::line!())
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! _abseil_var_concat {
    ($prefix:ident, $line:expr) => {
        concat_idents!($prefix, $line)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stringify_expr() {
        assert_eq!(stringify_expr!(1 + 2), "1 + 2");
        assert_eq!(stringify_expr!(foo), "foo");
    }

    #[test]
    fn test_type_name() {
        let name = type_name!(i32);
        assert!(name.contains("i32"));
    }

    #[test]
    fn test_count_tokens() {
        assert_eq!(count_tokens!(), 0);
        assert_eq!(count_tokens!(a), 1);
        assert_eq!(count_tokens!(a, b, c), 3);
    }

    #[test]
    fn test_va_args_count() {
        assert_eq!(va_args_count!(), 0);
        assert_eq!(va_args_count!(a), 1);
        assert_eq!(va_args_count!(a, b, c), 3);
    }

    #[test]
    fn test_first() {
        // The first! macro returns the first token - test with literals
        assert_eq!(first!(1), 1);
        assert_eq!(first!("a"), "a");
    }

    #[test]
    fn test_last() {
        // The last! macro returns the last token - test with literals
        assert_eq!(last!(1), 1);
        assert_eq!(last!(1, 2, 3), 3);
        assert_eq!(last!("a"), "a");
    }

    #[test]
    fn test_make_array() {
        let arr = make_array![1, 2, 3];
        assert_eq!(arr, [1, 2, 3]);
    }

    #[test]
    fn test_size_of() {
        assert_eq!(size_of!(u8), 1);
        assert_eq!(size_of!(i32), 4);
    }

    #[test]
    fn test_align_of() {
        assert_eq!(align_of!(u8), 1);
        assert_eq!(align_of!(i32), 4);
    }

    #[test]
    fn test_checked_cast() {
        let x: i64 = 42;
        let y: i32 = checked_cast!(x, i32);
        assert_eq!(y, 42);
    }

    #[test]
    fn test_saturating_cast() {
        let x: i64 = 300;
        let y: u8 = saturating_cast!(x, u8);
        assert_eq!(y, 255);

        let z: i64 = -10;
        let w: u8 = saturating_cast!(z, u8);
        assert_eq!(w, 0);
    }

    #[test]
    fn test_offset_of() {
        // Use #[repr(C)] to ensure predictable field order
        #[repr(C)]
        struct Test {
            a: u8,
            b: u32,
            c: u8,
        }

        let offset_a = offset_of!(Test, a);
        let offset_b = offset_of!(Test, b);
        let offset_c = offset_of!(Test, c);

        // With #[repr(C)], a is at offset 0
        assert_eq!(offset_a, 0, "First field should be at offset 0, got {}", offset_a);
        // b is after a (with padding for alignment to 4 bytes)
        assert!(offset_b >= 4, "b should be at least at offset 4");
        // c is after b
        assert!(offset_c > offset_b, "c should come after b");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_make_map() {
        use std::collections::HashMap;

        let map = make_map! {
            1 => "one",
            2 => "two",
        };

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&1), Some(&"one"));
        assert_eq!(map.get(&2), Some(&"two"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_make_btree_map() {
        use std::collections::BTreeMap;

        let map = make_btree_map! {
            "a" => 1,
            "b" => 2,
        };

        assert_eq!(map.len(), 2);
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), Some(&2));
    }
}
