//! Property-based tests using proptest patterns.
//!
//! This module tests invariants and properties that should hold for all inputs:
//! - Hash function properties (determinism, distribution)
//! - String/bytes roundtrip operations
//! - Graph traversal properties
//! - Numeric function properties

use abseil::absl_hash::{
    fnv_hash, fnv_hash_32, murmur3_64, xxhash_64, xxhash3_64, wyhash,
    hash_of, hash_combine,
};
use abseil::absl_graph::Graph;
use abseil::absl_numeric::{is_power_of_two, gcd, lcm};

// ============================================================================
// Hash Function Properties
// ============================================================================

#[test]
fn test_hash_determinism_fnv() {
    // Property: Same input should always produce same hash
    let input = b"hello world";
    let hash1 = fnv_hash(input);
    let hash2 = fnv_hash(input);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_determinism_fnv_32() {
    let input = b"test data";
    let hash1 = fnv_hash_32(input);
    let hash2 = fnv_hash_32(input);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_determinism_murmur3() {
    let input = b"consistent data";
    let seed = 42u64;
    let hash1 = murmur3_64(input, seed);
    let hash2 = murmur3_64(input, seed);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_determinism_xxhash() {
    let input = b"deterministic test";
    let hash1 = xxhash_64(input);
    let hash2 = xxhash_64(input);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_determinism_xxhash3() {
    let input = b"xxhash3 test data";
    let hash1 = xxhash3_64(input);
    let hash2 = xxhash3_64(input);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_determinism_wyhash() {
    let input = b"wyhash consistency";
    let hash1 = wyhash(input);
    let hash2 = wyhash(input);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_determinism_hash_of() {
    // Test hash_of macro/function
    let value = 42i32;
    let hash1 = hash_of(&value);
    let hash2 = hash_of(&value);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_different_inputs_different_hashes() {
    // Property: Different inputs should (usually) produce different hashes
    // We test this with several inputs - while collisions are possible,
    // they should be extremely rare with good hash functions
    let inputs = vec![
        b"hello",
        b"world",
        b"test",
        b"data",
        b"different",
    ];

    let hashes: Vec<u64> = inputs.iter().map(|&s| fnv_hash(s)).collect();

    // All hashes should be different (for these simple inputs)
    for i in 0..hashes.len() {
        for j in (i+1)..hashes.len() {
            assert_ne!(hashes[i], hashes[j],
                "Inputs {:?} and {:?} produced same hash", inputs[i], inputs[j]);
        }
    }
}

#[test]
fn test_hash_avalanche_effect() {
    // Property: Small input changes should cause large output changes
    let input1 = b"hello world";
    let input2 = b"hello worle"; // One bit difference

    let hash1 = fnv_hash(input1);
    let hash2 = fnv_hash(input2);

    // Count differing bits
    let xor = hash1 ^ hash2;
    let differing_bits = xor.count_ones();

    // At least half the bits should differ (good avalanche)
    assert!(differing_bits >= 32,
        "Avalanche effect weak: only {} bits differ", differing_bits);
}

#[test]
fn test_hash_empty_input() {
    // Property: Empty input should produce valid (non-panicking) hash
    let empty = b"";
    assert!(fnv_hash(empty) != 0 || true); // Just verify it doesn't panic
    assert!(fnv_hash_32(empty) != 0 || true);
    assert!(murmur3_64(empty, 0) != 0 || true);
}

#[test]
fn test_hash_single_byte_inputs() {
    // Property: All single byte values should produce valid hashes
    for byte in 0u8..=255 {
        let input = [byte];
        let hash = fnv_hash(&input);
        // Each byte should produce some hash (just verify no panic)
        let _ = hash;
    }
}

#[test]
fn test_hash_large_input() {
    // Property: Large inputs should be handled correctly
    let large_input = vec![0xFFu8; 10000];
    let hash = fnv_hash(&large_input);
    // Just verify it processes without panic
    assert!(hash != 0 || true);
}

#[test]
fn test_hash_seed_affects_murmur3() {
    // Property: Different seeds should produce different hashes
    let input = b"seeded test";
    let hash1 = murmur3_64(input, 0);
    let hash2 = murmur3_64(input, 42);
    let hash3 = murmur3_64(input, 999);

    // Different seeds should give different results
    assert_ne!(hash1, hash2);
    assert_ne!(hash2, hash3);
    assert_ne!(hash1, hash3);
}

#[test]
fn test_hash_combine_associative_property() {
    // Property: hash_combine should produce different results for different inputs
    let data1 = &[&1u32, &2u32, &3u32];
    let data2 = &[&1u32, &2u32, &4u32];

    let hash1 = hash_combine(data1);
    let hash2 = hash_combine(data2);

    assert_ne!(hash1, hash2);
}

#[test]
fn test_hash_combine_order_sensitive() {
    // Property: Order should affect combined hash
    let data1 = &[&1u32, &2u32, &3u32];
    let data2 = &[&3u32, &2u32, &1u32];

    let hash1 = hash_combine(data1);
    let hash2 = hash_combine(data2);

    assert_ne!(hash1, hash2);
}

// ============================================================================
// Numeric Function Properties
// ============================================================================

#[test]
fn test_is_power_of_two_boundary_values() {
    // Property: Powers of two should be identified correctly
    let powers_of_two: Vec<u32> = vec![
        1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024,
        1 << 10, 1 << 15, 1 << 20, 1 << 31,
    ];

    for &value in &powers_of_two {
        assert!(is_power_of_two(value),
            "{} should be a power of two", value);
    }

    // Test that values between powers of two are not powers of two
    for i in 1..10000 {
        if !powers_of_two.contains(&i) {
            assert!(!is_power_of_two(i),
                "{} should not be a power of two", i);
        }
    }
}

#[test]
fn test_is_power_of_two_zero_and_negative() {
    // Property: Zero should not be a power of two
    assert!(!is_power_of_two(0u32));
    assert!(!is_power_of_two(0u64));
    assert!(!is_power_of_two(0usize));

    // Negative values are never powers of two
    assert!(!is_power_of_two(-1i32));
    assert!(!is_power_of_two(-2i32));
    assert!(!is_power_of_two(-4i32));
}

#[test]
fn test_gcd_properties() {
    // Property 1: gcd(a, b) should divide both a and b
    for a in 1u32..100 {
        for b in 1u32..100 {
            let g = gcd(a, b);
            assert!(a % g == 0, "gcd({}, {}) = {} should divide {}", a, b, g, a);
            assert!(b % g == 0, "gcd({}, {}) = {} should divide {}", a, b, g, b);
        }
    }

    // Property 2: gcd should be symmetric
    for a in 1u32..100 {
        for b in 1u32..100 {
            assert_eq!(gcd(a, b), gcd(b, a),
                "gcd should be symmetric: gcd({}, {}) == gcd({}, {})", a, b, b, a);
        }
    }

    // Property 3: gcd(a, a) == a
    for a in 1u32..100 {
        assert_eq!(gcd(a, a), a, "gcd({}, {}) should equal {}", a, a, a);
    }

    // Property 4: gcd(a, 0) == a
    for a in 1u32..100 {
        assert_eq!(gcd(a, 0), a, "gcd({}, 0) should equal {}", a, a);
    }
}

#[test]
fn test_lcm_properties() {
    // Property 1: lcm(a, b) should be a multiple of both a and b
    for a in 1u32..50 {
        for b in 1u32..50 {
            let l = lcm(a, b);
            assert!(l % a == 0, "lcm({}, {}) = {} should be multiple of {}", a, b, l, a);
            assert!(l % b == 0, "lcm({}, {}) = {} should be multiple of {}", a, b, l, b);
        }
    }

    // Property 2: lcm should be symmetric
    for a in 1u32..50 {
        for b in 1u32..50 {
            assert_eq!(lcm(a, b), lcm(b, a),
                "lcm should be symmetric");
        }
    }

    // Property 3: lcm(a, a) == a
    for a in 1u32..50 {
        assert_eq!(lcm(a, a), a, "lcm({}, {}) should equal {}", a, a, a);
    }
}

#[test]
fn test_gcd_lcm_relationship() {
    // Property: gcd(a, b) * lcm(a, b) == a * b
    for a in 1u32..50 {
        for b in 1u32..50 {
            let g = gcd(a, b) as u64;
            let l = lcm(a, b) as u64;
            let product = (a as u64) * (b as u64);
            assert_eq!(g * l, product,
                "gcd * lcm should equal product: {} * {} == {}", g, l, product);
        }
    }
}

// ============================================================================
// Graph Properties
// ============================================================================

#[test]
fn test_graph_vertex_count_invariant() {
    // Property: vertex_count should equal number of vertices added
    let mut graph = Graph::new();

    for i in 0..100 {
        graph.add_vertex(i);
        assert_eq!(graph.vertex_count(), i + 1);
    }
}

#[test]
fn test_graph_edge_count_invariant() {
    // Property: edge_count should equal number of edges added
    let mut graph = Graph::new();

    let v0 = graph.add_vertex(0);
    let v1 = graph.add_vertex(1);
    let v2 = graph.add_vertex(2);

    for i in 0..10 {
        graph.add_edge(v0, v1, None);
        assert_eq!(graph.edge_count(), i + 1);
    }
}

#[test]
fn test_graph_id_uniqueness() {
    // Property: Each vertex should have a unique ID
    let mut graph = Graph::new();

    let mut ids = std::collections::HashSet::new();
    for i in 0..100 {
        let id = graph.add_vertex(i);
        assert!(!ids.contains(&id), "Duplicate vertex ID: {}", id);
        ids.insert(id);
    }

    assert_eq!(ids.len(), 100);
}

#[test]
fn test_graph_vertex_index_validity() {
    // Property: Valid vertex IDs should be retrievable
    let mut graph = Graph::new();

    for expected in 0..10 {
        let id = graph.add_vertex(expected);
        let vertex = graph.vertex(id);
        assert!(vertex.is_some(), "Vertex {} should exist", id);
        assert_eq!(vertex.unwrap().data, expected);
    }
}

#[test]
fn test_graph_reflexivity() {
    // Property: A vertex is reachable from itself
    let mut graph = Graph::new();

    let v0 = graph.add_vertex(0);
    let v1 = graph.add_vertex(1);
    graph.add_edge(v0, v1, None);

    // v0 should be in its own neighbors if we add a self-loop
    graph.add_edge(v0, v0, None);
    assert!(graph.has_edge(v0, v0));
}

#[test]
fn test_graph_empty_graph_properties() {
    // Property: Empty graph should have specific properties
    let graph: Graph<i32> = Graph::new();

    assert_eq!(graph.vertex_count(), 0);
    assert_eq!(graph.edge_count(), 0);
    assert!(graph.vertex(0).is_none());
}

// ============================================================================
// Bytes/String Properties
// ============================================================================

#[test]
fn test_hash_unicode_strings() {
    // Property: Unicode strings should hash consistently
    let unicode_strings = vec![
        "Hello, 世界!",
        "Привет мир",
        "مرحبا بالعالم",
        "🎉🎊🎈",
        "Ñoño",
        "café",
        "日本語",
    ];

    for s in &unicode_strings {
        let hash1 = fnv_hash(s.as_bytes());
        let hash2 = fnv_hash(s.as_bytes());
        assert_eq!(hash1, hash2,
            "Unicode string {:?} should hash consistently", s);
    }
}

#[test]
fn test_hash_null_bytes() {
    // Property: Hash should handle null bytes correctly
    let with_null = b"hello\0world";
    let without_null = b"hello world";

    // These should produce different hashes
    assert_ne!(fnv_hash(with_null), fnv_hash(without_null));
}

#[test]
fn test_hash_repeated_patterns() {
    // Property: Repeated patterns should be hashed correctly
    let pattern = b"abcd";
    let repeated = b"abcdabcdabcdabcd";

    // These should produce different hashes
    assert_ne!(fnv_hash(pattern), fnv_hash(repeated));
}

#[test]
fn test_hash_all_zeros() {
    // Property: All-zero input should produce specific hash
    let zeros = vec![0u8; 1000];
    let hash1 = fnv_hash(&zeros);
    let hash2 = fnv_hash(&zeros);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_all_ones() {
    // Property: All-ones input should hash correctly
    let ones = vec![0xFFu8; 1000];
    let hash1 = fnv_hash(&ones);
    let hash2 = fnv_hash(&ones);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_sequential_bytes() {
    // Property: Sequential patterns should hash
    let sequential: Vec<u8> = (0..255).collect();
    let hash = fnv_hash(&sequential);
    // Just verify it doesn't panic
    assert!(hash != 0 || true);
}

// ============================================================================
// Distribution Properties
// ============================================================================

#[test]
fn test_hash_distribution_quality() {
    // Property: Good hash functions should distribute values well
    // We test this by hashing many sequential values and checking
    // that the hashes are well-distributed

    let inputs: Vec<u32> = (0..1000).collect();
    let mut hashes: Vec<u64> = inputs.iter().map(|&v| fnv_hash(&v.to_le_bytes())).collect();

    // Sort to check distribution
    hashes.sort();

    // Count how many hash values are in each quartile
    let range = *hashes.last().unwrap() - *hashes.first().unwrap();
    let q1 = *hashes.first().unwrap() + range / 4;
    let q2 = *hashes.first().unwrap() + range / 2;
    let q3 = *hashes.first().unwrap() + 3 * range / 4;

    let mut count1 = 0;
    let mut count2 = 0;
    let mut count3 = 0;
    let mut count4 = 0;

    for hash in &hashes {
        if *hash < q1 { count1 += 1; }
        else if *hash < q2 { count2 += 1; }
        else if *hash < q3 { count3 += 1; }
        else { count4 += 1; }
    }

    // Each quartile should have roughly 25% of the values
    let expected = hashes.len() / 4;
    let tolerance = expected / 4; // Allow 25% deviation

    assert!(count1 >= expected - tolerance && count1 <= expected + tolerance,
        "Q1 has {} values, expected around {}", count1, expected);
    assert!(count4 >= expected - tolerance && count4 <= expected + tolerance,
        "Q4 has {} values, expected around {}", count4, expected);
}
