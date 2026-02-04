//! ResourceGuard - guard for managing resource lifetimes with access.

use core::cell::Cell;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

/// A guard that manages a resource and automatically cleans it up.
///
/// Unlike `Cleanup` which just runs a function, `ResourceGuard` owns
/// the resource and provides access to it during its lifetime.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_cleanup::ResourceGuard;
///
/// fn use_resource() -> ResourceGuard<Vec<i32>> {
///     let data = vec![1, 2, 3, 4, 5];
///     ResourceGuard::new(data, |vec| {
///         println!("Cleaning up {} elements", vec.len());
///     })
/// }
///
/// {
///     let guard = use_resource();
///     println!("First element: {}", guard[0]); // Access the resource
/// } // cleanup runs here
/// ```
pub struct ResourceGuard<T, F: FnOnce(&mut T)> {
    resource: ManuallyDrop<T>,
    cleanup_fn: ManuallyDrop<F>,
    dismissed: Cell<bool>,
}

impl<T, F: FnOnce(&mut T)> ResourceGuard<T, F> {
    /// Creates a new resource guard.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ResourceGuard;
    ///
    /// let guard = ResourceGuard::new(vec![1, 2, 3], |v| {
    ///     println!("Cleaning up vector with {} elements", v.len());
    /// });
    /// ```
    pub fn new(resource: T, cleanup_fn: F) -> Self {
        Self {
            resource: ManuallyDrop::new(resource),
            cleanup_fn: ManuallyDrop::new(cleanup_fn),
            dismissed: Cell::new(false),
        }
    }

    /// Dismisses the cleanup, preventing it from running.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ResourceGuard;
    ///
    /// let mut guard = ResourceGuard::new(vec![1, 2, 3], |_| {});
    /// guard.dismiss();
    /// ```
    pub fn dismiss(&mut self) {
        self.dismissed.set(true);
    }

    /// Returns true if this guard has been dismissed.
    pub fn is_dismissed(&self) -> bool {
        self.dismissed.get()
    }

    /// Returns a reference to the managed resource.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ResourceGuard;
    ///
    /// let guard = ResourceGuard::new(vec![1, 2, 3], |_| {});
    /// assert_eq!(guard.get(), &[1, 2, 3]);
    /// ```
    pub fn get(&self) -> &T {
        &self.resource
    }

    /// Returns a mutable reference to the managed resource.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ResourceGuard;
    ///
    /// let mut guard = ResourceGuard::new(vec![1, 2, 3], |_| {});
    /// guard.get_mut().push(4);
    /// assert_eq!(guard.get(), &[1, 2, 3, 4]);
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.resource
    }

    /// Consumes the guard and returns the resource without running cleanup.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ResourceGuard;
    ///
    /// let guard = ResourceGuard::new(vec![1, 2, 3], |_| {});
    /// let vec = guard.into_inner();
    /// // cleanup does NOT run
    /// ```
    pub fn into_inner(mut self) -> T {
        self.dismissed.set(true);
        // SAFETY: We've marked as dismissed, so Drop won't run
        unsafe { ManuallyDrop::take(&mut self.resource) }
    }

    /// Executes the cleanup function early with a mutable reference.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_cleanup::ResourceGuard;
    ///
    /// let mut guard = ResourceGuard::new(vec![1, 2, 3], |v| {
    ///     v.clear();
    /// });
    /// guard.cleanup(); // vector is cleared now
    /// assert_eq!(guard.get(), &[]);
    /// ```
    pub fn cleanup(&mut self) {
        if !self.dismissed.get() {
            self.dismissed.set(true);
            let resource = unsafe { &mut *self.resource as *mut T };
            let cleanup_fn = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
            // SAFETY: We have exclusive access and cleanup won't run again
            unsafe { cleanup_fn(&mut *resource) };
        }
    }
}

impl<T, F: FnOnce(&mut T)> Deref for ResourceGuard<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T, F: FnOnce(&mut T)> DerefMut for ResourceGuard<T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T, F: FnOnce(&mut T)> Drop for ResourceGuard<T, F> {
    fn drop(&mut self) {
        if !self.dismissed.get() {
            let resource = unsafe { &mut *self.resource as *mut T };
            let cleanup_fn = unsafe { ManuallyDrop::take(&mut self.cleanup_fn) };
            // SAFETY: We're in drop, have exclusive access
            unsafe { cleanup_fn(&mut *resource) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_guard_access() {
        let guard = ResourceGuard::new(vec![1, 2, 3], |_| {});
        assert_eq!(guard.get(), &[1, 2, 3]);
        assert_eq!(guard[0], 1);
    }

    #[test]
    fn test_resource_guard_mut_access() {
        let mut guard = ResourceGuard::new(vec![1, 2, 3], |_| {});
        guard.get_mut().push(4);
        assert_eq!(guard.get(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_resource_guard_cleanup_runs() {
        let cleaned = crate::absl_cleanup::tests::TestCounter::new();
        {
            let _guard = ResourceGuard::new(vec![1, 2, 3], |_| cleaned.inc());
        }
        assert_eq!(cleaned.get(), 1);
    }

    #[test]
    fn test_resource_guard_dismiss() {
        let cleaned = crate::absl_cleanup::tests::TestCounter::new();
        {
            let mut guard = ResourceGuard::new(vec![1, 2, 3], |_| cleaned.inc());
            guard.dismiss();
            assert!(guard.is_dismissed());
        }
        assert_eq!(cleaned.get(), 0);
    }

    #[test]
    fn test_resource_guard_into_inner() {
        let guard = ResourceGuard::new(vec![1, 2, 3], |_| panic!("Should not run"));
        let vec = guard.into_inner();
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_resource_guard_cleanup() {
        let cleaned = crate::absl_cleanup::tests::TestCounter::new();
        let mut guard = ResourceGuard::new(vec![1, 2, 3], |v| {
            v.clear();
            cleaned.inc();
        });
        guard.cleanup();
        assert_eq!(guard.get(), &[]);
        assert_eq!(cleaned.get(), 1);
    }

    #[test]
    fn test_multiple_resource_guards() {
        let count1 = crate::absl_cleanup::tests::TestCounter::new();
        let count2 = crate::absl_cleanup::tests::TestCounter::new();

        {
            let _g1 = ResourceGuard::new(1, |_| count1.inc());
            let _g2 = ResourceGuard::new(2, |_| count2.inc());
        }

        assert_eq!(count1.get(), 1);
        assert_eq!(count2.get(), 1);
    }
}
