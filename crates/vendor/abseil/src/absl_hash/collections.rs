//! HashMap and HashSet marker types for hash-based collection operations.

use core::marker::PhantomData;

/// A hash map that uses custom hashing.
///
/// This is a simple wrapper for hash-based map operations.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::HashMap;
///
/// let mut map = HashMap::new();
/// map.insert(1, "one");
/// map.insert(2, "two");
/// ```
pub struct HashMap<K, V>(PhantomData<(K, V)>);

impl<K, V> HashMap<K, V> {
    /// Creates a new empty HashMap marker.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<K, V> Default for HashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone, V: Clone> Clone for HashMap<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: Copy, V: Copy> Copy for HashMap<K, V> {}

/// A hash set that uses custom hashing.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::HashSet;
///
/// let mut set = HashSet::new();
/// set.insert(1);
/// set.insert(2);
/// ```
pub struct HashSet<T>(PhantomData<T>);

impl<T> HashSet<T> {
    /// Creates a new empty HashSet marker.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for HashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for HashSet<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Copy> Copy for HashSet<T> {}
