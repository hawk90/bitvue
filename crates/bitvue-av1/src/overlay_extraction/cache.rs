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

/// Maximum number of coding unit entries to cache
/// Prevents unbounded memory growth from processing many different frames
const MAX_CACHE_ENTRIES: usize = 64;

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
/// Uses "single lock acquisition" pattern to prevent TOCTOU race condition.
/// Lock is held for entire operation (check, parse, insert) ensuring thread safety.
pub fn get_or_parse_coding_units<F>(
    cache_key: u64,
    parse_fn: F,
) -> Result<Vec<crate::tile::CodingUnit>, BitvueError>
where
    F: FnOnce() -> Result<Vec<crate::tile::CodingUnit>, BitvueError>,
{
    let mut cache = lock_mutex!(CODING_UNIT_CACHE);

    // Check if already cached (still holding lock)
    if let Some(cached) = cache.get(&cache_key) {
        tracing::debug!("Cache HIT for coding units: {} units", cached.len());
        return Ok(cached.clone());
    }

    // Cache miss - parse and insert (still holding lock)
    // This prevents other threads from simultaneously parsing the same key
    tracing::debug!("Cache MISS - parsing coding units from tile data");
    let units = parse_fn()?;

    // Enforce cache size limit to prevent unbounded growth
    // If cache is full, evict 25% of entries (pseudo-random eviction)
    if cache.len() >= MAX_CACHE_ENTRIES {
        let remove_count = MAX_CACHE_ENTRIES / 4;
        tracing::debug!(
            "Cache full ({} entries), evicting {} entries",
            cache.len(),
            remove_count
        );

        // Pseudo-random eviction: remove first N keys from iterator
        // HashMap doesn't preserve order, so this gives a random sampling
        let keys_to_remove: Vec<_> = cache.keys().take(remove_count).copied().collect();
        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    cache.insert(cache_key, units.clone());

    Ok(units)
}

/// Clear the coding unit cache (useful for testing)
///
/// Per generate-tests skill: Provide test utilities for cache management
#[cfg(test)]
pub fn clear_cu_cache() {
    let mut cache = CODING_UNIT_CACHE.lock().unwrap();
    cache.clear();
    // Ensure the cache is actually empty
    assert!(cache.is_empty());
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

    #[test]
    fn test_cache_size_limit() {
        // Note: This test uses shared static cache state.
        // Run with --test-threads=1 if this test flakes in parallel execution.
        clear_cu_cache();

        // Add entries up to limit (use unique data with sufficient entropy)
        let mut added = 0;
        for i in 0..=MAX_CACHE_ENTRIES {
            // Use a simple counter that won't wrap (each entry unique)
            let tile_data = vec![1u8, 2u8, 3u8, 4u8, 5u8, i as u8];
            let cache_key = compute_cache_key(&tile_data, 32);
            let _ = get_or_parse_coding_units(cache_key, || {
                added += 1;
                Ok(vec![])
            });
            if added == MAX_CACHE_ENTRIES {
                break;
            }
        }

        let size_at_limit = cu_cache_size();
        assert_eq!(size_at_limit, MAX_CACHE_ENTRIES, "Should reach cache limit");

        // Add one more entry - should trigger eviction
        let tile_data = vec![9u8, 9u8, 9u8, 9u8, 9u8, 9u8];
        let cache_key = compute_cache_key(&tile_data, 33);
        let _ = get_or_parse_coding_units(cache_key, || Ok(vec![]));

        // Cache should be smaller due to eviction
        let size_after = cu_cache_size();
        assert!(
            size_after < size_at_limit,
            "Cache should shrink after eviction: {} < {}",
            size_after,
            size_at_limit
        );
        // Should have roughly MAX_CACHE_ENTRIES - 25% + 1 entries
        assert!(
            size_after >= MAX_CACHE_ENTRIES * 3 / 4,
            "Cache should retain most entries after eviction: {} >= {}",
            size_after,
            MAX_CACHE_ENTRIES * 3 / 4
        );
    }
}
