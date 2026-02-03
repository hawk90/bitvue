//! AnyMap - Type-keyed storage.

use alloc::collections::BTreeMap;
use core::any::TypeId;
use crate::absl_any::any_box::AnyBox;

/// A map keyed by type, storing one value per type.
///
/// This is useful for storing heterogeneous collections where each type
/// appears at most once.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::AnyMap;
///
/// let mut map = AnyMap::new();
/// map.insert(42i32);
/// map.insert("hello");
///
/// assert_eq!(map.get::<i32>(), Some(&42));
/// assert_eq!(map.get::<&str>(), Some(&"hello"));
/// ```
#[derive(Default)]
pub struct AnyMap {
    data: BTreeMap<TypeId, AnyBox>,
}

impl AnyMap {
    /// Creates a new empty AnyMap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a value into the map.
    ///
    /// Returns the old value if one existed for this type.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyMap;
    ///
    /// let mut map = AnyMap::new();
    /// map.insert(42i32);
    /// map.insert(100i32); // Replaces the previous value
    /// ```
    pub fn insert<T: 'static>(&mut self, value: T) -> Option<AnyBox> {
        self.data.insert(TypeId::of::<T>(), AnyBox::new(value))
    }

    /// Gets a reference to the value of type T.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyMap;
    ///
    /// let mut map = AnyMap::new();
    /// map.insert(42i32);
    /// assert_eq!(map.get::<i32>(), Some(&42));
    /// assert_eq!(map.get::<i64>(), None);
    /// ```
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.data.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Gets a mutable reference to the value of type T.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.data.get_mut(&TypeId::of::<T>())?.downcast_mut::<T>()
    }

    /// Removes the value of type T from the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyMap;
    ///
    /// let mut map = AnyMap::new();
    /// map.insert(42i32);
    /// assert_eq!(map.remove::<i32>(), Some(42));
    /// assert_eq!(map.remove::<i32>(), None);
    /// ```
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.data.remove(&TypeId::of::<T>())?.downcast::<T>().ok()
    }

    /// Returns true if the map contains a value of type T.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_any::AnyMap;
    ///
    /// let mut map = AnyMap::new();
    /// map.insert(42i32);
    /// assert!(map.contains::<i32>());
    /// assert!(!map.contains::<i64>());
    /// ```
    pub fn contains<T: 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Returns the number of types stored in the map.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clears the map, removing all values.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Iterates over all stored type IDs.
    pub fn type_ids(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.data.keys().copied()
    }

    /// Gets the type name of each stored value.
    pub fn type_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.data.values().map(|v| v.type_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_map_new() {
        let map = AnyMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_any_map_insert() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        assert_eq!(map.get::<i32>(), Some(&42));
    }

    #[test]
    fn test_any_map_replace() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        map.insert(100i32);
        assert_eq!(map.get::<i32>(), Some(&100));
    }

    #[test]
    fn test_any_map_get() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        map.insert("hello");
        assert_eq!(map.get::<i32>(), Some(&42));
        assert_eq!(map.get::<&str>(), Some(&"hello"));
    }

    #[test]
    fn test_any_map_get_mut() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        if let Some(v) = map.get_mut::<i32>() {
            *v = 100;
        }
        assert_eq!(map.get::<i32>(), Some(&100));
    }

    #[test]
    fn test_any_map_remove() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        assert_eq!(map.remove::<i32>(), Some(42));
        assert_eq!(map.remove::<i32>(), None);
    }

    #[test]
    fn test_any_map_contains() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        assert!(map.contains::<i32>());
        assert!(!map.contains::<i64>());
    }

    #[test]
    fn test_any_map_clear() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        map.insert("hello");
        map.clear();
        assert!(map.is_empty());
    }

    #[test]
    fn test_any_map_type_ids() {
        let mut map = AnyMap::new();
        map.insert(42i32);
        map.insert("hello");
        let ids: Vec<_> = map.type_ids().collect();
        assert_eq!(ids.len(), 2);
    }
}
