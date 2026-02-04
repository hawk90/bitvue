//! FlatHashMap placeholder - Placeholder for flat hash map (similar to Abseil's flat_hash_map)

/// Placeholder for flat hash map (similar to Abseil's `flat_hash_map`).
///
/// This is a placeholder that wraps Rust's `std::collections::HashMap`.
/// A true flat hash map implementation would use open addressing with
/// quadratic probing.
pub type FlatHashMap<K, V> = alloc::collections::BTreeMap<K, V>;

/// Creates a new flat hash map.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::flat_hash_map;
///
/// let map = flat_hash_map();
/// assert!(map.is_empty());
/// ```
#[inline]
pub fn flat_hash_map<K, V>() -> FlatHashMap<K, V> {
    FlatHashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_hash_map() {
        let map = flat_hash_map();
        assert!(map.is_empty());
    }
}
