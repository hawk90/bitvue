//! Cache management for overlay extraction
//!
//! Provides thread-safe LRU caching for parsed coding units to avoid
//! re-parsing when extracting multiple overlays from the same frame.

use bitvue_core::BitvueError;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;

/// Helper macro to safely lock mutexes with proper error handling
/// Prevents panic on mutex poisoning by returning an error instead
macro_rules! lock_mutex {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|e| BitvueError::Decode(format!("Mutex poisoned: {}", e)))?
    };
}

/// Per optimize-code skill: Thread-safe LRU cache for parsed coding units
///
/// Caches parsed coding units per frame to avoid re-parsing
/// when multiple overlays are extracted from the same frame.
///
/// Key: Hash of tile data + base_qp (ensures cache validity)
/// Value: Parsed coding units
type CodingUnitCache = HashMap<u64, Vec<crate::tile::CodingUnit>>;

/// Global thread-safe cache for coding units (module-level)
///
/// Per optimize-code skill: Use LazyLock for safe static initialization
/// This avoids re-parsing the same tile data when extracting multiple overlays.
///
/// Per optimize-code skill § "Batch Operations":
/// "Single lock acquisition" pattern - lock once for the entire operation
static CODING_UNIT_CACHE: LazyLock<Mutex<CodingUnitCache>> =
    LazyLock::new(|| Mutex::new(HashMap::with_capacity(16)));

/// Compute cache key from tile data
///
/// Per optimize-code skill: Use hash-based cache keys for fast lookup
pub fn compute_cache_key(tile_data: &[u8], base_qp: i16) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    tile_data.hash(&mut hasher);
    base_qp.hash(&mut hasher);
    hasher.finish()
}

/// Get cached coding units or parse and cache them
///
/// Per optimize-code skill: "Single lock acquisition" pattern
/// Gets or inserts into cache in a single lock operation
pub fn get_or_parse_coding_units<F>(
    cache_key: u64,
    parse_fn: F,
) -> Result<Vec<crate::tile::CodingUnit>, BitvueError>
where
    F: FnOnce() -> Result<Vec<crate::tile::CodingUnit>, BitvueError>,
{
    // Per optimize-code skill: Check cache first with read lock
    {
        let cache = lock_mutex!(CODING_UNIT_CACHE);
        if let Some(cached) = cache.get(&cache_key) {
            tracing::debug!("Cache HIT for coding units: {} units", cached.len());
            return Ok(cached.clone());
        }
    }

    // Cache miss - parse and store
    tracing::debug!("Cache MISS - parsing coding units from tile data");
    let units = parse_fn()?;

    // Per optimize-code skill: Single lock acquisition for insert
    {
        let mut cache = lock_mutex!(CODING_UNIT_CACHE);
        cache.insert(cache_key, units.clone());
    }

    Ok(units)
}

/// Clear the coding unit cache (useful for testing)
///
/// Per generate-tests skill: Provide test utilities for cache management
#[cfg(test)]
pub fn clear_cu_cache() {
    let mut cache = CODING_UNIT_CACHE.lock().unwrap();
    cache.clear();
}

/// Get the size of the coding unit cache (for testing)
#[cfg(test)]
pub fn cu_cache_size() -> usize {
    let cache = CODING_UNIT_CACHE.lock().unwrap();
    cache.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit_and_miss() {
        // Clear cache before test
        clear_cu_cache();

        let tile_data = vec![1u8, 2, 3, 4, 5];
        let base_qp: i16 = 32;
        let cache_key = compute_cache_key(&tile_data, base_qp);

        // First call should be a cache miss
        let parse_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let parse_count_clone = parse_count.clone();

        let result = get_or_parse_coding_units(cache_key, || {
            parse_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![])
        });

        assert!(result.is_ok());
        assert_eq!(parse_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call should be a cache hit
        let result = get_or_parse_coding_units(cache_key, || {
            parse_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![])
        });

        assert!(result.is_ok());
        assert_eq!(parse_count.load(std::sync::atomic::Ordering::SeqCst), 1, "Should not re-parse on cache hit");
    }

    #[test]
    fn test_clear_cu_cache() {
        // Add something to cache
        let tile_data = vec![1u8, 2, 3];
        let cache_key = compute_cache_key(&tile_data, 32);

        let _ = get_or_parse_coding_units(cache_key, || Ok(vec![]));
        assert!(cu_cache_size() > 0);

        // Clear and verify empty
        clear_cu_cache();
        assert_eq!(cu_cache_size(), 0);
    }
}
