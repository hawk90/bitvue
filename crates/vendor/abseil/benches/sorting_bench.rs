#![feature(test)]
extern crate test;

use abseil::absl_sorting::{
    bubble_sort, heapsort, insertion_sort, introsort, mergesort, quicksort, radix_sort, timsort,
};
use test::{black_box, Bencher};

// Small data (10 elements)
fn small_data() -> Vec<i32> {
    vec![64, 34, 25, 12, 22, 11, 90, 88, 45, 33]
}

// Small data for radix sort (unsigned)
fn small_data_u32() -> Vec<u32> {
    vec![64, 34, 25, 12, 22, 11, 90, 88, 45, 33]
}

// Medium data (100 elements)
fn medium_data() -> Vec<i32> {
    (0..100).rev().collect()
}

// Medium data for radix sort (unsigned)
fn medium_data_u32() -> Vec<u32> {
    (0..100).rev().collect()
}

// Large data (1000 elements)
fn large_data() -> Vec<i32> {
    (0..1000).rev().collect()
}

// Large data for radix sort (unsigned)
fn large_data_u32() -> Vec<u32> {
    (0..1000).rev().collect()
}

// Extra large data (10000 elements)
fn xlarge_data() -> Vec<i32> {
    (0..10000).rev().collect()
}

// Extra large data for radix sort (unsigned)
fn xlarge_data_u32() -> Vec<u32> {
    (0..10000).rev().collect()
}

// MERGE SORT BENCHMARKS
#[bench]
fn bench_mergesort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        mergesort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_mergesort_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let mut data = data.clone();
        mergesort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_mergesort_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let mut data = data.clone();
        mergesort(&mut data);
        black_box(data);
    });
}

// QUICK SORT BENCHMARKS
#[bench]
fn bench_quicksort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        quicksort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_quicksort_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let mut data = data.clone();
        quicksort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_quicksort_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let mut data = data.clone();
        quicksort(&mut data);
        black_box(data);
    });
}

// HEAP SORT BENCHMARKS
#[bench]
fn bench_heapsort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        heapsort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_heapsort_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let mut data = data.clone();
        heapsort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_heapsort_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let mut data = data.clone();
        heapsort(&mut data);
        black_box(data);
    });
}

// RADIX SORT BENCHMARKS
#[bench]
fn bench_radix_sort_small(b: &mut Bencher) {
    let data = small_data_u32();
    b.iter(|| {
        let mut data = data.clone();
        radix_sort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_radix_sort_medium(b: &mut Bencher) {
    let data = medium_data_u32();
    b.iter(|| {
        let mut data = data.clone();
        radix_sort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_radix_sort_large(b: &mut Bencher) {
    let data = large_data_u32();
    b.iter(|| {
        let mut data = data.clone();
        radix_sort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_radix_sort_xlarge(b: &mut Bencher) {
    let data = xlarge_data_u32();
    b.iter(|| {
        let mut data = data.clone();
        radix_sort(&mut data);
        black_box(data);
    });
}

// INTROSORT BENCHMARKS
#[bench]
fn bench_introsort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        introsort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_introsort_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let mut data = data.clone();
        introsort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_introsort_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let mut data = data.clone();
        introsort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_introsort_xlarge(b: &mut Bencher) {
    let data = xlarge_data();
    b.iter(|| {
        let mut data = data.clone();
        introsort(&mut data);
        black_box(data);
    });
}

// TIMSORT BENCHMARKS
#[bench]
fn bench_timsort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        timsort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_timsort_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let mut data = data.clone();
        timsort(&mut data);
        black_box(data);
    });
}

#[bench]
fn bench_timsort_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let mut data = data.clone();
        timsort(&mut data);
        black_box(data);
    });
}

// BUBBLE SORT (only small - intentionally slow)
#[bench]
fn bench_bubble_sort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        bubble_sort(&mut data);
        black_box(data);
    });
}

// INSERTION SORT (only small - O(n²))
#[bench]
fn bench_insertion_sort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        insertion_sort(&mut data);
        black_box(data);
    });
}

// STANDARD LIBRARY SORT (for comparison)
#[bench]
fn bench_std_sort_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort();
        black_box(data);
    });
}

#[bench]
fn bench_std_sort_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort();
        black_box(data);
    });
}

#[bench]
fn bench_std_sort_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort();
        black_box(data);
    });
}

#[bench]
fn bench_std_sort_xlarge(b: &mut Bencher) {
    let data = xlarge_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort();
        black_box(data);
    });
}

// STANDARD LIBRARY UNSTABLE SORT (for comparison)
#[bench]
fn bench_std_sort_unstable_small(b: &mut Bencher) {
    let data = small_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort_unstable();
        black_box(data);
    });
}

#[bench]
fn bench_std_sort_unstable_medium(b: &mut Bencher) {
    let data = medium_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort_unstable();
        black_box(data);
    });
}

#[bench]
fn bench_std_sort_unstable_large(b: &mut Bencher) {
    let data = large_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort_unstable();
        black_box(data);
    });
}

#[bench]
fn bench_std_sort_unstable_xlarge(b: &mut Bencher) {
    let data = xlarge_data();
    b.iter(|| {
        let mut data = data.clone();
        data.sort_unstable();
        black_box(data);
    });
}
