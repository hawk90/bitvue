//! ValueHash - wrapper type that hashes by value rather than by reference.

use core::hash::{Hash, Hasher};

use super::hash::hash_of;

/// A wrapper type that hashes by value rather than by reference.
///
/// This is useful when you want to use value-based hashing in containers
/// that would otherwise use reference hashing.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::ValueHash;
/// use std::collections::HashMap;
///
/// let mut map = HashMap::new();
/// map.insert(ValueHash::new(vec![1, 2, 3]), "value");
/// ```
#[derive(Clone, Debug)]
pub struct ValueHash<T> {
    inner: T,
    cached_hash: Option<u64>,
}

impl<T> ValueHash<T> {
    /// Creates a new ValueHash wrapper.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_hash::ValueHash;
    ///
    /// let wrapper = ValueHash::new(vec![1, 2, 3]);
    /// ```
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            cached_hash: None,
        }
    }

    /// Returns a reference to the inner value.
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the inner value.
    pub fn get_mut(&mut self) -> &mut T {
        self.cached_hash = None; // Invalidate cache
        &mut self.inner
    }

    /// Consumes the wrapper and returns the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Hash> ValueHash<T> {
    /// Returns the cached or computed hash value.
    pub fn hash_value(&mut self) -> u64 {
        match self.cached_hash {
            Some(hash) => hash,
            None => {
                let hash = hash_of(&self.inner);
                self.cached_hash = Some(hash);
                hash
            }
        }
    }
}

impl<T: PartialEq> PartialEq for ValueHash<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Eq> Eq for ValueHash<T> {}

impl<T: Hash> Hash for ValueHash<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.cached_hash {
            Some(hash) => hash.hash(state),
            None => self.inner.hash(state),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_hash() {
        let wrapper = ValueHash::new(vec![1, 2, 3]);
        assert_eq!(wrapper.get(), &[1, 2, 3]);
    }

    #[test]
    fn test_value_hash_mut() {
        let mut wrapper = ValueHash::new(vec![1, 2, 3]);
        wrapper.get_mut().push(4);
        assert_eq!(wrapper.get(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_value_hash_into_inner() {
        let wrapper = ValueHash::new(vec![1, 2, 3]);
        let vec = wrapper.into_inner();
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_value_hash_cached() {
        let mut wrapper = ValueHash::new(vec![1, 2, 3]);
        let hash1 = wrapper.hash_value();
        let hash2 = wrapper.hash_value();
        assert_eq!(hash1, hash2);
    }
}
