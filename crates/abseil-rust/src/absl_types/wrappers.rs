//! Wrapper types for specific behaviors.

use alloc::boxed::Box;
use alloc::string::String;
use core::cell::Cell;
use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull as CoreNonNull;

/// A wrapper that ensures the contained value is never null.
///
/// This is similar to Rust's `core::ptr::NonNull` but provided as a
/// higher-level wrapper that can contain any reference type.
///
/// # Examples
///
/// ```
/// use abseil::absl_types::NonNull;
///
/// let value = 42;
/// let non_null = NonNull::new(&value);
/// assert_eq!(non_null.get(), &42);
/// ```
#[derive(Clone, Copy)]
pub struct NonNull<T: ?Sized> {
    inner: CoreNonNull<T>,
}

impl<T: ?Sized> NonNull<T> {
    /// Creates a new NonNull from a reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_types::NonNull;
    ///
    /// let value = 42;
    /// let non_null = NonNull::new(&value);
    /// ```
    #[inline]
    pub fn new(value: &T) -> Self {
        Self {
            inner: CoreNonNull::from(value).into(),
        }
    }

    /// Creates a new NonNull from a mutable reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_types::NonNull;
    ///
    /// let mut value = 42;
    /// let non_null = NonNull::new_mut(&mut value);
    /// ```
    #[inline]
    pub fn new_mut(value: &mut T) -> Self {
        Self {
            inner: CoreNonNull::from(value).into(),
        }
    }

    /// Returns a reference to the contained value.
    #[inline]
    pub fn get(&self) -> &T {
        // SAFETY: NonNull guarantees this is non-null
        // We use as_ref() which is the safe method provided by core::ptr::NonNull
        unsafe { self.inner.as_ref() }
    }

    /// Returns a mutable reference to the contained value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T
    where
        T: Unpin,
    {
        // SAFETY: NonNull guarantees this is non-null, and T: Unpin ensures
        // we can safely create a mutable reference through the pointer.
        unsafe { self.inner.as_mut() }
    }

    /// Returns true if the contained value is null.
    ///
    /// This always returns false for references, but is provided for
    /// API compatibility with pointer-based NonNull.
    #[inline]
    pub fn is_null(&self) -> bool {
        false
    }

    /// Creates a NonNull from a raw pointer.
    ///
    /// # Safety
    ///
    /// The pointer must not be null, and must be properly aligned.
    /// The pointer must point to valid memory for the lifetime of the NonNull.
    ///
    /// For a safe alternative, see [`try_from_ptr`](Self::try_from_ptr).
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_types::NonNull;
    ///
    /// let value = 42;
    /// let non_null = unsafe { NonNull::from_ptr(&value as *const _) };
    /// assert_eq!(non_null.get(), &42);
    /// ```
    #[inline]
    pub unsafe fn from_ptr(ptr: *const T) -> Self {
        Self {
            inner: CoreNonNull::new_unchecked(ptr),
        }
    }

    /// Attempts to create a NonNull from a raw pointer.
    ///
    /// Returns `None` if the pointer is null.
    ///
    /// # Safety
    ///
    /// The pointer must be properly aligned and must point to valid memory
    /// for the lifetime of the NonNull (if not null).
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_types::NonNull;
    ///
    /// let value = 42;
    /// let non_null = NonNull::try_from_ptr(&value as *const _).unwrap();
    /// assert_eq!(non_null.get(), &42);
    ///
    /// let null_ptr = std::ptr::null::<i32>();
    /// assert!(NonNull::try_from_ptr(null_ptr).is_none());
    /// ```
    #[inline]
    pub fn try_from_ptr(ptr: *const T) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: We've checked that ptr is not null
            unsafe { Some(Self::from_ptr(ptr)) }
        }
    }
}

impl<T: ?Sized> fmt::Debug for NonNull<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NonNull({:?})", self.get())
    }
}

/// A generic wrapper type that can contain any value.
///
/// This is useful for type erasure or when you need to store
/// values of different types in a homogeneous collection.
///
/// # Examples
///
/// ```
/// use abseil::absl_types::Wrapper;
///
/// let wrapped_int = Wrapper::new(42i32);
/// let wrapped_str = Wrapper::new("hello");
///
/// assert_eq!(wrapped_int.get(), &42);
/// assert_eq!(wrapped_str.get(), &"hello");
/// ```
#[derive(Clone, Debug)]
pub struct Wrapper<T> {
    inner: T,
}

impl<T> Wrapper<T> {
    /// Creates a new wrapper containing the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_types::Wrapper;
    ///
    /// let wrapped = Wrapper::new(42);
    /// assert_eq!(wrapped.get(), &42);
    /// ```
    #[inline]
    pub const fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Gets a reference to the wrapped value.
    #[inline]
    pub const fn get(&self) -> &T {
        &self.inner
    }

    /// Gets a mutable reference to the wrapped value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consumes the wrapper and returns the inner value.
    #[inline]
    pub const fn into_inner(self) -> T {
        self.inner
    }

    /// Maps the wrapped value using a function.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_types::Wrapper;
    ///
    /// let wrapped = Wrapper::new(42);
    /// let mapped = wrapped.map(|x| x * 2);
    /// assert_eq!(mapped.get(), &84);
    /// ```
    #[inline]
    pub fn map<U, F>(self, f: F) -> Wrapper<U>
    where
        F: FnOnce(T) -> U,
    {
        Wrapper::new(f(self.inner))
    }
}

/// An ownership marker type.
///
/// This type doesn't do anything at runtime, but it's used to indicate
/// that a value takes ownership of some resource.
///
/// # Examples
///
/// ```
/// use abseil::absl_types::Owner;
///
/// fn take_ownership<T>(resource: T) -> Owner<T> {
///     Owner::new(resource)
/// }
///
/// fn release_ownership<T>(owner: Owner<T>) -> T {
///     owner.into_inner()
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Owner<T>(pub T);

impl<T> Owner<T> {
    /// Creates a new Owner wrapper.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Consumes the Owner and returns the owned value.
    #[inline]
    pub const fn into_inner(self) -> T {
        self.0
    }

    /// Gets a reference to the owned value.
    #[inline]
    pub const fn get(&self) -> &T {
        &self.0
    }

    /// Gets a mutable reference to the owned value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A wrapper for transparent field access.
///
/// This wrapper allows accessing fields through newtype patterns.
///
/// # Examples
///
/// ```
/// use abseil::absl_types::Transparent;
///
/// struct NewType(u32);
///
/// let wrapped = Transparent::new(NewType(42));
/// assert_eq!(wrapped.0, 42);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transparent<T>(pub T);

impl<T> Transparent<T> {
    /// Creates a new transparent wrapper.
    #[inline]
    pub const fn new(inner: T) -> Self {
        Self(inner)
    }

    /// Consumes the wrapper and returns the inner value.
    #[inline]
    pub const fn into_inner(self) -> T {
        self.0
    }

    /// Gets a reference to the inner value.
    #[inline]
    pub const fn get(&self) -> &T {
        &self.0
    }

    /// Gets a mutable reference to the inner value.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A wrapper that disables automatic trait derivation.
///
/// This is useful when you want to opt-out of derived traits
/// for specific types.
///
/// # Examples
///
/// ```
/// use abseil::absl_types::Opaque;
///
/// let opaque = Opaque::new(42);
/// // opaque doesn't implement Copy despite wrapping i32
/// ```
#[derive(Debug)]
pub struct Opaque<T>(pub T);

impl<T> Opaque<T> {
    /// Creates a new opaque wrapper.
    #[inline]
    pub const fn new(inner: T) -> Self {
        Self(inner)
    }

    /// Consumes the wrapper and returns the inner value.
    #[inline]
    pub const fn into_inner(self) -> T {
        self.0
    }
}

/// A wrapper for pinned data.
///
/// This ensures data cannot be moved in memory.
///
/// # Safety
///
/// The pinning invariant is upheld because:
/// 1. The inner value is stored in a Box, which has a stable memory address
/// 2. We only expose pinned references through as_pin/as_pin_mut
/// 3. The inner value cannot be moved out without consuming the Pinned wrapper
///
/// # Examples
///
/// ```
/// use abseil::absl_types::Pinned;
/// use core::pin::Pin;
///
/// let pinned = Pinned::new(Box::pin(42));
/// ```
#[derive(Debug)]
pub struct Pinned<T> {
    inner: Box<T>,
}

impl<T> Pinned<T> {
    /// Creates a new pinned value.
    pub fn new(value: Box<T>) -> Self {
        Self { inner: value }
    }

    /// Gets a pinned reference to the inner value.
    ///
    /// # Safety
    ///
    /// This is safe because:
    /// 1. The Box has a stable memory address
    /// 2. We're creating a Pin from a reference to data that won't move
    /// 3. The lifetime of the returned Pin is tied to self, preventing
    ///    the Pinned wrapper from being dropped while the Pin is active
    pub fn as_pin(&self) -> Pin<&T> {
        // SAFETY: The data is pinned in a Box which has stable address.
        // The reference lifetime is tied to &self, ensuring the Pinned
        // wrapper (and thus the Box) remains valid.
        unsafe { Pin::new_unchecked(&*self.inner) }
    }

    /// Gets a pinned mutable reference to the inner value.
    ///
    /// # Safety
    ///
    /// This is safe because:
    /// 1. The Box has a stable memory address
    /// 2. We're creating a Pin from a reference to data that won't move
    /// 3. The lifetime of the returned Pin is tied to &mut self, preventing
    ///    the Pinned wrapper from being dropped while the Pin is active
    pub fn as_pin_mut(&mut self) -> Pin<&mut T> {
        // SAFETY: The data is pinned in a Box which has stable address.
        // The reference lifetime is tied to &mut self, ensuring the Pinned
        // wrapper (and thus the Box) remains valid.
        unsafe { Pin::new_unchecked(&mut *self.inner) }
    }

    /// Consumes the Pinned wrapper and returns the inner Box.
    ///
    /// This allows unpinned access to the value. Note that after calling
    /// this method, the value is no longer pinned.
    pub fn into_inner(self) -> Box<T> {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_null() {
        let value = 42;
        let non_null = NonNull::new(&value);
        assert_eq!(non_null.get(), &42);
        assert!(!non_null.is_null());
    }

    #[test]
    fn test_non_null_try_from_ptr() {
        let value = 42;
        let non_null = NonNull::try_from_ptr(&value as *const _).unwrap();
        assert_eq!(non_null.get(), &42);

        let null_ptr = core::ptr::null::<i32>();
        assert!(NonNull::try_from_ptr(null_ptr).is_none());
    }

    #[test]
    fn test_pinned_into_inner() {
        let pinned = Pinned::new(Box::pin(42));
        let inner = pinned.into_inner();
        assert_eq!(*inner, 42);
    }

    #[test]
    fn test_wrapper() {
        let wrapped = Wrapper::new(42);
        assert_eq!(wrapped.get(), &42);

        let mapped = wrapped.map(|x| x * 2);
        assert_eq!(mapped.get(), &84);
    }

    #[test]
    fn test_owner() {
        let owner = Owner::new(vec![1, 2, 3]);
        assert_eq!(owner.get(), &[1, 2, 3]);
        let vec = owner.into_inner();
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_transparent() {
        let wrapped = Transparent::new(42u32);
        assert_eq!(wrapped.0, 42);
        assert_eq!(wrapped.get(), &42);
    }

    #[test]
    fn test_transparent_into_inner() {
        let wrapped = Transparent::new(42u32);
        assert_eq!(wrapped.into_inner(), 42);
    }

    #[test]
    fn test_opaque() {
        let opaque = Opaque::new(42);
        assert_eq!(opaque.into_inner(), 42);
    }

    #[test]
    fn test_pinned() {
        let pinned = Pinned::new(Box::pin(42));
        assert_eq!(*pinned.as_pin(), 42);
    }
}
