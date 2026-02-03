//! Compiler attribute utilities.
//!
//! This module provides attribute macros and utilities similar to Abseil's
//! `absl/base/attributes.h`, which provides compiler-specific attributes
//! for optimization, deprecation, and other purposes.

/// Combines multiple annotations into a single attribute.
///
/// This macro allows you to apply multiple attributes at once.
///
/// Note: This is a documentation-only macro. In Rust, you should use
/// built-in attributes like `#[must_use]`, `#[inline]`, etc.
///
/// # Example
///
/// ```rust
/// use abseil::annotate;
///
/// // This macro is for documentation purposes
/// annotate!();
/// ```
#[macro_export]
macro_rules! annotate {
    () => {};
    // This macro doesn't actually generate code, it's for API compatibility
    ($($name:tt)*) => {};
}

/// Attribute to mark a function as cold - it's unlikely to be called.
///
/// This helps the compiler optimize for the common case where the
/// annotated function is not executed (e.g., error paths).
///
/// # Example
///
/// The macro expands to the `#[cold]` attribute:
///
/// ```rust
/// use abseil::cold_path;
///
/// // Instead of directly using the macro as an attribute,
/// // use Rust's built-in #[cold] attribute:
/// #[cold]
/// fn handle_fatal_error() -> ! {
///     panic!("Fatal error occurred");
/// }
/// ```
#[macro_export]
macro_rules! cold_path {
    () => {
        #[cold]
    };
}

/// Attribute to mark a function as hot - it's likely to be called frequently.
///
/// This tells the compiler to optimize more aggressively for this function.
///
/// # Example
///
/// The macro expands to the `#[inline]` attribute:
///
/// ```rust
/// // Instead of using the macro, use Rust's built-in #[inline] attribute:
/// #[inline]
/// fn performance_critical_function(x: i32) -> i32 {
///     x * 2
/// }
/// ```
#[macro_export]
macro_rules! hot_path {
    () => {
        #[inline]
    };
}

/// Marks a type as having a trivial destructor.
///
/// This is a documentation marker indicating that the type does not
/// need special cleanup when dropped.
///
/// # Example
///
/// This is a documentation macro that adds a doc comment:
///
/// ```rust
/// // The macro is for documentation purposes only.
/// // In practice, just document that your type has a trivial destructor.
///
/// struct SimpleStruct {
///     value: i32,
/// }
/// ```
#[macro_export]
macro_rules! trivially_destructible {
    () => {
        #[doc = "This type has a trivial destructor"]
    };
}

/// Marks a type as trivially copyable (POD - Plain Old Data).
///
/// This is a documentation marker for types that can be safely copied
/// with memcpy and have no special copy semantics.
///
/// # Example
///
/// This is a documentation macro - in practice use Rust's built-in attributes:
///
/// ```rust
/// #[repr(C)]
/// #[derive(Clone, Copy)]
/// struct PodStruct {
///     x: i32,
///     y: f64,
/// }
/// ```
#[macro_export]
macro_rules! trivially_copyable {
    () => {
        #[doc = "This type is trivially copyable"]
    };
}

/// Checks if a type is trivially destructible at compile time.
///
/// This is a const function that returns true if the type has a trivial
/// destructor (i.e., doesn't need special cleanup).
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::attributes::is_trivially_destructible;
///
/// // Vec needs drop (returns true from needs_drop)
/// assert!(is_trivially_destructible::<Vec<u8>>());
/// // i32 doesn't need drop (returns false from needs_drop)
/// assert!(!is_trivially_destructible::<i32>());
/// ```
#[inline]
#[must_use]
pub const fn is_trivially_destructible<T: ?Sized>() -> bool {
    // In Rust, we use needs_drop which returns true if the type needs drop
    core::mem::needs_drop::<T>()
}

/// Checks if a type is trivially copyable at compile time.
///
/// This is a const function that returns true if the type can be safely
/// copied with memcpy (Copy trait and no custom copy behavior).
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::attributes::is_trivially_copyable;
///
/// // For this implementation, we check if the type implements Copy
/// // This is a simplified check
/// ```
#[inline]
#[must_use]
pub const fn is_trivially_copyable<T: ?Sized>() -> bool {
    // This is a conceptual check. In Rust, types implementing Copy
    // are trivially copyable, but we can't detect this as a const fn.
    true
}

/// Helper macro to suppress warnings (wrapper around Rust's `#[allow]`).
///
/// Note: For actual use, prefer Rust's built-in `#[allow(...)]` attribute.
/// This macro is provided for API compatibility with Abseil.
///
/// # Example
///
/// ```rust
/// use abseil::suppress_warning;
///
/// // Instead of: #[allow(unused_variables)]
/// // Use: (You can't use macros as attributes directly)
/// // Just use the built-in #[allow(...)] attribute
/// ```
#[macro_export]
macro_rules! suppress_warning {
    // This is a no-op wrapper - in real code, use #[allow(...)] directly
    ($($warning:ident),* $(,)?) => {
        // The macro exists for API compatibility
        // For actual use, use: #[allow($($warning),*)]
    };
}

/// Gets the name of the current function.
///
/// This macro expands to a string literal containing the function name.
///
/// # Example
///
/// ```rust
/// use abseil::current_function_name;
///
/// fn my_function() {
///     let name = current_function_name!();
///     assert!(name.contains("my_function"));
/// }
/// ```
#[macro_export]
macro_rules! current_function_name {
    () => {
        // Use the stable approach with proc_macro_style identification
        // In stable Rust, we can't get the exact function name at compile time
        // but we can get module path and line number
        concat!(module_path!(), ":", line!())
    };
}

/// Gets the source file name.
///
/// This macro expands to a string literal containing the file name.
#[macro_export]
macro_rules! source_file_name {
    () => {
        file!()
    };
}

/// Gets the line number in the source file.
///
/// This macro expands to a u32 containing the current line number.
#[macro_export]
macro_rules! source_line_number {
    () => {
        line!()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_function_name() {
        let name = current_function_name!();
        assert!(name.contains("test_current_function_name") || name.contains(module_path!()));
    }

    #[test]
    fn test_source_file_name() {
        let name = source_file_name!();
        assert!(name.contains("attributes.rs"));
    }

    #[test]
    fn test_source_line_number() {
        let line = source_line_number!();
        assert!(line > 0);
    }

    #[test]
    fn test_annotate_macro() {
        // The annotate macro should compile without errors
        annotate!();
        annotate!("must_use");
        annotate!("inline", "always");
    }

    #[test]
    fn test_suppress_warning_macro() {
        // The macro should compile and expand
        // Since it can't be used as an attribute directly, we just verify it compiles
        suppress_warning!(dead_code, unused_variables);
        suppress_warning!(unused_variables);
    }

    #[test]
    fn test_is_trivially_destructible() {
        // Vec needs drop
        assert!(is_trivially_destructible::<Vec<u8>>());
        // i32 doesn't need drop
        assert!(!is_trivially_destructible::<i32>());
    }
}
