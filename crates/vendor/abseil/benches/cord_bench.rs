// Comprehensive benchmarks for Cord operations
// Tests performance of incremental comparison, hashing, and fast paths
#![cfg(bench)]

use abseil::absl_strings::cord::Cord;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use test::{black_box, Bencher};

// ========== Test Data ==========

fn small_cord() -> Cord {
    Cord::from("Hello, world!")
}

fn medium_cord() -> Cord {
    let mut cord = Cord::new();
    for i in 0..10 {
        cord.append(&format!("Chunk number {} ", i));
    }
    cord
}

fn large_cord() -> Cord {
    let mut cord = Cord::new();
    for i in 0..100 {
        cord.append(&format!("Chunk number {} with some content ", i));
    }
    cord
}

fn xlarge_cord() -> Cord {
    let mut cord = Cord::new();
    for i in 0..1000 {
        cord.append(&format!("Part {} ", i));
    }
    cord
}

fn single_chunk_cord_small() -> Cord {
    Cord::from("This is a single chunk string")
}

fn single_chunk_cord_large() -> Cord {
    Cord::from(&"A".repeat(10000))
}

fn multi_chunk_cord_small() -> Cord {
    let mut cord = Cord::new();
    cord.append("Hello");
    cord.append(" ");
    cord.append("world");
    cord.append("!");
    cord
}

fn multi_chunk_cord_medium() -> Cord {
    let mut cord = Cord::new();
    for i in 0..50 {
        cord.append(&format!("Chunk{}", i));
    }
    cord
}

fn multi_chunk_cord_large() -> Cord {
    let mut cord = Cord::new();
    for i in 0..500 {
        cord.append(&format!("Chunk{}", i));
    }
    cord
}

// ========== Construction benchmarks ==========

#[bench]
fn bench_cord_new(b: &mut Bencher) {
    b.iter(|| {
        let cord = Cord::new();
        black_box(cord);
    });
}

#[bench]
fn bench_cord_from_str_small(b: &mut Bencher) {
    let s = "Hello, world!";
    b.iter(|| {
        let cord = Cord::from(black_box(s));
        black_box(cord);
    });
}

#[bench]
fn bench_cord_from_str_large(b: &mut Bencher) {
    let s = "A".repeat(10000);
    b.iter(|| {
        let cord = Cord::from(black_box(&s));
        black_box(cord);
    });
}

#[bench]
fn bench_cord_append_small(b: &mut Bencher) {
    b.iter(|| {
        let mut cord = Cord::new();
        for i in 0..5 {
            cord.append(&format!("Part{}", i));
        }
        black_box(cord);
    });
}

#[bench]
fn bench_cord_append_medium(b: &mut Bencher) {
    b.iter(|| {
        let mut cord = Cord::new();
        for i in 0..50 {
            cord.append(&format!("Part{}", i));
        }
        black_box(cord);
    });
}

#[bench]
fn bench_cord_append_large(b: &mut Bencher) {
    b.iter(|| {
        let mut cord = Cord::new();
        for i in 0..500 {
            cord.append(&format!("Part{}", i));
        }
        black_box(cord);
    });
}

#[bench]
fn bench_cord_prepend_small(b: &mut Bencher) {
    b.iter(|| {
        let mut cord = Cord::new();
        for i in 0..5 {
            cord.prepend(&format!("Part{}", i));
        }
        black_box(cord);
    });
}

#[bench]
fn bench_cord_append_string(b: &mut Bencher) {
    let strings: Vec<String> = (0..100).map(|i| format!("String{}", i)).collect();
    b.iter(|| {
        let mut cord = Cord::new();
        for s in &strings {
            cord.append(s);
        }
        black_box(cord);
    });
}

#[bench]
fn bench_cord_append_cord_small(b: &mut Bencher) {
    let cord1 = Cord::from("Hello");
    let cord2 = Cord::from(" World");
    b.iter(|| {
        let mut cord = Cord::from("Hello");
        let other = Cord::from(" World");
        cord.append_cord(other);
        black_box(cord);
    });
}

#[bench]
fn bench_cord_append_cord_large(b: &mut Bencher) {
    b.iter(|| {
        let mut cord1 = Cord::new();
        let mut cord2 = Cord::new();
        for i in 0..50 {
            cord1.append(&format!("C1-{}", i));
            cord2.append(&format!("C2-{}", i));
        }
        cord1.append_cord(cord2);
        black_box(cord1);
    });
}

// ========== Conversion benchmarks ==========

#[bench]
fn bench_cord_to_string_single_chunk(b: &mut Bencher) {
    let cord = single_chunk_cord_large();
    b.iter(|| {
        let s = cord.to_string_value();
        black_box(s);
    });
}

#[bench]
fn bench_cord_to_string_multi_chunk(b: &mut Bencher) {
    let cord = multi_chunk_cord_medium();
    b.iter(|| {
        let s = cord.to_string_value();
        black_box(s);
    });
}

#[bench]
fn bench_cord_as_str_single_chunk(b: &mut Bencher) {
    let cord = single_chunk_cord_small();
    b.iter(|| {
        let s = cord.as_str();
        black_box(s);
    });
}

#[bench]
fn bench_cord_as_str_multi_chunk(b: &mut Bencher) {
    let cord = multi_chunk_cord_small();
    b.iter(|| {
        let s = cord.as_str();
        black_box(s);
    });
}

#[bench]
fn bench_cord_to_str_single_chunk(b: &mut Bencher) {
    let cord = single_chunk_cord_small();
    b.iter(|| {
        let s = cord.to_str();
        black_box(s);
    });
}

#[bench]
fn bench_cord_to_str_multi_chunk(b: &mut Bencher) {
    let cord = multi_chunk_cord_small();
    b.iter(|| {
        let s = cord.to_str();
        black_box(s);
    });
}

// ========== Equality benchmarks (incremental comparison) ==========

#[bench]
fn bench_cord_eq_single_chunk_equal(b: &mut Bencher) {
    let cord1 = single_chunk_cord_large();
    let cord2 = single_chunk_cord_large();
    b.iter(|| {
        let result = black_box(&cord1) == black_box(&cord2);
        black_box(result);
    });
}

#[bench]
fn bench_cord_eq_single_chunk_not_equal(b: &mut Bencher) {
    let cord1 = single_chunk_cord_large();
    let cord2 = Cord::from(&"B".repeat(10000));
    b.iter(|| {
        let result = black_box(&cord1) == black_box(&cord2);
        black_box(result);
    });
}

#[bench]
fn bench_cord_eq_multi_chunk_equal(b: &mut Bencher) {
    let cord1 = multi_chunk_cord_medium();
    let cord2 = multi_chunk_cord_medium();
    b.iter(|| {
        let result = black_box(&cord1) == black_box(&cord2);
        black_box(result);
    });
}

#[bench]
fn bench_cord_eq_multi_chunk_not_equal(b: &mut Bencher) {
    let cord1 = multi_chunk_cord_medium();
    let cord2 = Cord::from("different content");
    b.iter(|| {
        let result = black_box(&cord1) == black_box(&cord2);
        black_box(result);
    });
}

#[bench]
fn bench_cord_eq_different_sizes(b: &mut Bencher) {
    let cord1 = large_cord();
    let cord2 = small_cord();
    b.iter(|| {
        let result = black_box(&cord1) == black_box(&cord2);
        black_box(result);
    });
}

#[bench]
fn bench_cord_eq_single_vs_multi_chunk(b: &mut Bencher) {
    let cord1 = single_chunk_cord_small();
    let cord2 = multi_chunk_cord_small();
    b.iter(|| {
        let result = black_box(&cord1) == black_box(&cord2);
        black_box(result);
    });
}

// ========== Hash benchmarks (incremental hashing) ==========

#[bench]
fn bench_cord_hash_single_chunk_small(b: &mut Bencher) {
    let cord = single_chunk_cord_small();
    b.iter(|| {
        let mut hasher = DefaultHasher::new();
        black_box(&cord).hash(&mut hasher);
        black_box(hasher.finish());
    });
}

#[bench]
fn bench_cord_hash_single_chunk_large(b: &mut Bencher) {
    let cord = single_chunk_cord_large();
    b.iter(|| {
        let mut hasher = DefaultHasher::new();
        black_box(&cord).hash(&mut hasher);
        black_box(hasher.finish());
    });
}

#[bench]
fn bench_cord_hash_multi_chunk_small(b: &mut Bencher) {
    let cord = multi_chunk_cord_small();
    b.iter(|| {
        let mut hasher = DefaultHasher::new();
        black_box(&cord).hash(&mut hasher);
        black_box(hasher.finish());
    });
}

#[bench]
fn bench_cord_hash_multi_chunk_medium(b: &mut Bencher) {
    let cord = multi_chunk_cord_medium();
    b.iter(|| {
        let mut hasher = DefaultHasher::new();
        black_box(&cord).hash(&mut hasher);
        black_box(hasher.finish());
    });
}

#[bench]
fn bench_cord_hash_multi_chunk_large(b: &mut Bencher) {
    let cord = multi_chunk_cord_large();
    b.iter(|| {
        let mut hasher = DefaultHasher::new();
        black_box(&cord).hash(&mut hasher);
        black_box(hasher.finish());
    });
}

// ========== Ordering benchmarks ==========

#[bench]
fn bench_cord_cmp_single_chunk(b: &mut Bencher) {
    let cord1 = single_chunk_cord_small();
    let cord2 = Cord::from("Hello, there!");
    b.iter(|| {
        let result = black_box(&cord1).cmp(black_box(&cord2));
        black_box(result);
    });
}

#[bench]
fn bench_cord_cmp_multi_chunk(b: &mut Bencher) {
    let cord1 = multi_chunk_cord_small();
    let cord2 = Cord::from("Different content");
    b.iter(|| {
        let result = black_box(&cord1).cmp(black_box(&cord2));
        black_box(result);
    });
}

#[bench]
fn bench_cord_partial_cmp(b: &mut Bencher) {
    let cord1 = medium_cord();
    let cord2 = medium_cord();
    b.iter(|| {
        let result = black_box(&cord1).partial_cmp(black_box(&cord2));
        black_box(result);
    });
}

// ========== Size and property benchmarks ==========

#[bench]
fn bench_cord_size_single_chunk(b: &mut Bencher) {
    let cord = single_chunk_cord_large();
    b.iter(|| {
        let size = black_box(&cord).size();
        black_box(size);
    });
}

#[bench]
fn bench_cord_size_multi_chunk(b: &mut Bencher) {
    let cord = multi_chunk_cord_medium();
    b.iter(|| {
        let size = black_box(&cord).size();
        black_box(size);
    });
}

#[bench]
fn bench_cord_is_empty_true(b: &mut Bencher) {
    let cord = Cord::new();
    b.iter(|| {
        let empty = black_box(&cord).is_empty();
        black_box(empty);
    });
}

#[bench]
fn bench_cord_is_empty_false(b: &mut Bencher) {
    let cord = single_chunk_cord_small();
    b.iter(|| {
        let empty = black_box(&cord).is_empty();
        black_box(empty);
    });
}

#[bench]
fn bench_cord_chunk_count(b: &mut Bencher) {
    let cord = multi_chunk_cord_medium();
    b.iter(|| {
        let count = black_box(&cord).chunk_count();
        black_box(count);
    });
}

// ========== Iterator benchmarks ==========

#[bench]
fn bench_cord_chunks_iter(b: &mut Bencher) {
    let cord = multi_chunk_cord_medium();
    b.iter(|| {
        let count = black_box(&cord).chunks().count();
        black_box(count);
    });
}

#[bench]
fn bench_cord_chars_iter_small(b: &mut Bencher) {
    let cord = single_chunk_cord_small();
    b.iter(|| {
        let chars: Vec<char> = black_box(&cord).chars().collect();
        black_box(chars);
    });
}

#[bench]
fn bench_cord_chars_iter_large(b: &mut Bencher) {
    let cord = single_chunk_cord_large();
    b.iter(|| {
        let count = black_box(&cord).chars().count();
        black_box(count);
    });
}

#[bench]
fn bench_cord_chars_iter_multi_chunk(b: &mut Bencher) {
    let cord = multi_chunk_cord_medium();
    b.iter(|| {
        let count = black_box(&cord).chars().count();
        black_box(count);
    });
}

#[bench]
fn bench_cord_bytes_iter_small(b: &mut Bencher) {
    let cord = single_chunk_cord_small();
    b.iter(|| {
        let bytes: Vec<u8> = black_box(&cord).bytes().collect();
        black_box(bytes);
    });
}

#[bench]
fn bench_cord_bytes_iter_large(b: &mut Bencher) {
    let cord = single_chunk_cord_large();
    b.iter(|| {
        let count = black_box(&cord).bytes().count();
        black_box(count);
    });
}

// ========== Subview and split benchmarks ==========

#[bench]
fn bench_cord_subview_start(b: &mut Bencher) {
    let cord = large_cord();
    b.iter(|| {
        let subcord = black_box(&cord).subview(0, 100);
        black_box(subcord);
    });
}

#[bench]
fn bench_cord_subview_middle(b: &mut Bencher) {
    let cord = large_cord();
    b.iter(|| {
        let subcord = black_box(&cord).subview(500, 1000);
        black_box(subcord);
    });
}

#[bench]
fn bench_cord_subview_end(b: &mut Bencher) {
    let cord = large_cord();
    b.iter(|| {
        let subcord = black_box(&cord).subview(cord.size() - 100, cord.size());
        black_box(subcord);
    });
}

#[bench]
fn bench_cord_split_small(b: &mut Bencher) {
    let cord = medium_cord();
    b.iter(|| {
        let (left, right) = black_box(&cord).split(500);
        black_box((left, right));
    });
}

#[bench]
fn bench_cord_split_large(b: &mut Bencher) {
    let cord = xlarge_cord();
    let pos = cord.size() / 2;
    b.iter(|| {
        let (left, right) = black_box(&cord).split(pos);
        black_box((left, right));
    });
}

// ========== Clone benchmarks ==========

#[bench]
fn bench_cord_clone_single_chunk(b: &mut Bencher) {
    let cord = single_chunk_cord_large();
    b.iter(|| {
        let cloned = black_box(&cord).clone();
        black_box(cloned);
    });
}

#[bench]
fn bench_cord_clone_multi_chunk(b: &mut Bencher) {
    let cord = multi_chunk_cord_medium();
    b.iter(|| {
        let cloned = black_box(&cord).clone();
        black_box(cloned);
    });
}

// ========== Display formatting ==========

#[bench]
fn bench_cord_display_single_chunk(b: &mut Bencher) {
    let cord = single_chunk_cord_small();
    b.iter(|| {
        let s = format!("{}", black_box(&cord));
        black_box(s);
    });
}

#[bench]
fn bench_cord_display_multi_chunk(b: &mut Bencher) {
    let cord = multi_chunk_cord_small();
    b.iter(|| {
        let s = format!("{}", black_box(&cord));
        black_box(s);
    });
}

// ========== Clear benchmark ==========

#[bench]
fn bench_cord_clear(b: &mut Bencher) {
    b.iter(|| {
        let mut cord = large_cord();
        cord.clear();
        black_box(&cord);
    });
}
