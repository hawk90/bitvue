//! BTreeMap/BTreeSet utilities - Wrapper around BTreeMap/BTreeSet with additional utilities

/// Wrapper around BTreeMap with additional utilities.
pub type BTreeMap<K, V> = alloc::collections::BTreeMap<K, V>;

/// Wrapper around BTreeSet with additional utilities.
pub type BTreeSet<T> = alloc::collections::BTreeSet<T>;

/// Creates a new BTreeMap.
#[inline]
pub fn btree_map<K, V>() -> BTreeMap<K, V>
where
    K: core::cmp::Ord,
{
    BTreeMap::new()
}

/// Creates a new BTreeSet.
#[inline]
pub fn btree_set<T>() -> BTreeSet<T>
where
    T: core::cmp::Ord,
{
    BTreeSet::new()
}

/// Gets a value from a map or inserts a default.
///
/// This is a convenience wrapper that uses the entry API internally.
#[inline]
pub fn get_or_insert<K, V: Default>(map: &mut BTreeMap<K, V>, key: K) -> &V
where
    K: core::cmp::Ord,
{
    // Note: BTreeMap doesn't have entry API like HashMap,
    // so we need to use contains_key + get pattern
    if !map.contains_key(&key) {
        map.insert(key, V::default());
    }
    // Safe: we just ensured the key exists
    match map.get(&key) {
        Some(v) => v,
        None => unsafe {
            // This should never happen because we just inserted the key
            core::hint::unreachable_unchecked()
        },
    }
}

/// Gets a value from a map or computes it with a function.
#[inline]
pub fn get_or_insert_with<K, V, F>(map: &mut BTreeMap<K, V>, key: K, f: F) -> &V
where
    K: core::cmp::Ord,
    F: FnOnce() -> V,
{
    if !map.contains_key(&key) {
        map.insert(key, f());
    }
    // Safe: we just ensured the key exists
    match map.get(&key) {
        Some(v) => v,
        None => unsafe {
            // This should never happen because we just inserted the key
            core::hint::unreachable_unchecked()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_map() {
        let mut map = btree_map();
        map.insert("key1", "value1");
        map.insert("key2", "value2");

        assert_eq!(map.get("key1"), Some(&"value1"));
        assert!(map.contains_key("key2"));
    }

    #[test]
    fn test_btree_set() {
        let mut set = btree_set();
        set.insert(1);
        set.insert(2);
        set.insert(3);

        assert!(set.contains(&2));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_get_or_insert() {
        let mut map: BTreeMap<&str, i32> = btree_map();
        let value = get_or_insert(&mut map, "count");
        assert_eq!(*value, 0);
        assert_eq!(map.get("count"), Some(&0));
    }

    #[test]
    fn test_get_or_insert_with() {
        let mut map: BTreeMap<&str, i32> = btree_map();
        let value = get_or_insert_with(&mut map, "count", || 42);
        assert_eq!(*value, 42);
        assert_eq!(map.get("count"), Some(&42));
    }
}
