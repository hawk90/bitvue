//! NodeHashMap placeholder - Placeholder for node hash map (similar to Abseil's node_hash_map)

/// Placeholder for node hash map (similar to Abseil's `node_hash_map`).
///
/// This is a placeholder that wraps Rust's `std::collections::HashMap`.
/// A true node hash map implementation would use separate chaining.
pub type NodeHashMap<K, V> = alloc::collections::BTreeMap<K, V>;

/// Creates a new node hash map.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_container::node_hash_map;
///
/// let map = node_hash_map();
/// assert!(map.is_empty());
/// ```
#[inline]
pub fn node_hash_map<K, V>() -> NodeHashMap<K, V> {
    NodeHashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_hash_map() {
        let map = node_hash_map();
        assert!(map.is_empty());
    }
}
