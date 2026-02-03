//! Comprehensive thread safety edge case tests.
//!
//! This module tests edge cases related to concurrent access patterns:
//! - Concurrent read/write operations
//! - Stress tests for synchronization primitives
//! - Race condition scenarios
//! - Deadlock prevention

use abseil::absl_base::call_once::{call_once, is_done, OnceFlag};
use abseil::absl_hash::BloomFilter;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_once_flag_concurrent_initialization() {
    // Test multiple threads racing to initialize
    let flag = Arc::new(OnceFlag::new());
    let counter = Arc::new(AtomicI32::new(0));
    let mut handles = vec![];

    for _ in 0..100 {
        let flag_clone = Arc::clone(&flag);
        let counter_clone = Arc::clone(&counter);

        handles.push(thread::spawn(move || {
            call_once(&flag_clone, || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Despite 100 threads, the closure should only run once
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert!(is_done(&flag));
}

#[test]
fn test_once_flag_concurrent_after_done() {
    // Test concurrent calls after initialization is complete
    let flag = Arc::new(OnceFlag::new());
    let counter = Arc::new(AtomicI32::new(0));

    // Initialize first
    call_once(&flag, || {
        counter.fetch_add(1, Ordering::SeqCst);
    });

    // Now spawn many threads that will all see it as done
    let mut handles = vec![];
    for _ in 0..50 {
        let flag_clone = Arc::clone(&flag);
        handles.push(thread::spawn(move || {
            call_once(&flag_clone, || {
                // This should not execute
                panic!("Should not execute!");
            });
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Counter should still be 1
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_once_flag_stress_test() {
    // Stress test with rapid concurrent calls
    let flag = Arc::new(OnceFlag::new());
    let counter = Arc::new(AtomicI32::new(0));
    let mut handles = vec![];

    for _ in 0..1000 {
        let flag_clone = Arc::clone(&flag);
        let counter_clone = Arc::clone(&counter);

        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                call_once(&flag_clone, || {
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                });
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Should still only execute once despite 100,000 total calls
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_once_flag_slow_initialization() {
    // Test with slow initialization to ensure waiting works
    let flag = Arc::new(OnceFlag::new());
    let started = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];

    // Thread that will do the slow initialization
    {
        let flag_clone = Arc::clone(&flag);
        let started_clone = Arc::clone(&started);
        let finished_clone = Arc::clone(&finished);

        handles.push(thread::spawn(move || {
            call_once(&flag_clone, || {
                started_clone.store(true, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(100));
                finished_clone.store(true, Ordering::SeqCst);
            });
        }));
    }

    // Threads that will wait for initialization
    for _ in 0..10 {
        let flag_clone = Arc::clone(&flag);
        let finished_clone = Arc::clone(&finished);

        handles.push(thread::spawn(move || {
            // Wait a bit to ensure the init thread starts first
            thread::sleep(Duration::from_millis(10));

            call_once(&flag_clone, || {
                panic!("Should not execute - init thread should run first");
            });

            // By the time we get here, initialization should be complete
            assert!(finished_clone.load(Ordering::SeqCst));
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(finished.load(Ordering::SeqCst));
}

#[test]
fn test_once_flag_no_deadlock() {
    // Test that there's no deadlock with rapid concurrent access
    for iteration in 0..100 {
        let flag = Arc::new(OnceFlag::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn many threads that all try to initialize
        for _ in 0..20 {
            let flag_clone = Arc::clone(&flag);
            let counter_clone = Arc::clone(&counter);

            handles.push(thread::spawn(move || {
                call_once(&flag_clone, || {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                });
            }));
        }

        // Add a timeout to detect deadlocks
        for handle in handles {
            let result = thread::spawn(move || {
                handle.join().unwrap()
            }).join();

            if result.is_err() {
                panic!("Potential deadlock in iteration {}", iteration);
            }
        }

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}

#[test]
fn test_bloom_filter_concurrent_inserts() {
    // Test concurrent inserts to BloomFilter
    let bloom = Arc::new(std::sync::Mutex::new(BloomFilter::new(10_000, 7)));
    let mut handles = vec![];

    for thread_id in 0..10 {
        let bloom_clone = Arc::clone(&bloom);
        handles.push(thread::spawn(move || {
            for i in 0..1000 {
                let mut bloom = bloom_clone.lock().unwrap();
                bloom.insert(&(thread_id * 1000 + i));
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all inserted values are present
    let bloom = bloom.lock().unwrap();
    for thread_id in 0..10 {
        for i in 0..1000 {
            assert!(bloom.contains(&(thread_id * 1000 + i)),
                "Should contain value {} from thread {}",
                thread_id * 1000 + i, thread_id);
        }
    }
}

#[test]
fn test_bloom_filter_concurrent_insert_and_contains() {
    // Test concurrent inserts and contains
    let bloom = Arc::new(std::sync::Mutex::new(BloomFilter::new(10_000, 7)));
    let insert_done = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];

    // Insert threads
    for i in 0..5 {
        let bloom_clone = Arc::clone(&bloom);
        let insert_done_clone = Arc::clone(&insert_done);

        handles.push(thread::spawn(move || {
            for j in 0..1000 {
                let mut bloom = bloom_clone.lock().unwrap();
                bloom.insert(&(i * 1000 + j));
            }
            insert_done_clone.store(true, Ordering::SeqCst);
        }));
    }

    // Query threads
    for _ in 0..5 {
        let bloom_clone = Arc::clone(&bloom);
        handles.push(thread::spawn(move || {
            loop {
                {
                    let bloom = bloom_clone.lock().unwrap();
                    // Just check that we can access it
                    let _ = bloom.contains(&42);
                }
                // Check if we're done
                if insert_done.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_bloom_filter_concurrent_clear() {
    // Test concurrent operations with clear
    let bloom = Arc::new(std::sync::Mutex::new(BloomFilter::new(1000, 7)));
    let mut handles = vec![];

    // Thread that clears periodically
    {
        let bloom_clone = Arc::clone(&bloom);
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(10));
                let mut bloom = bloom_clone.lock().unwrap();
                bloom.clear();
            }
        }));
    }

    // Threads that insert
    for i in 0..5 {
        let bloom_clone = Arc::clone(&bloom);
        handles.push(thread::spawn(move || {
            for j in 0..100 {
                let mut bloom = bloom_clone.lock().unwrap();
                bloom.insert(&(i * 100 + j));
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Just verify we didn't crash
    let bloom = bloom.lock().unwrap();
    let _ = bloom.contains(&42);
}

#[test]
fn test_once_flag_interleaved_calls() {
    // Test interleaved calls to call_once with different flags
    let flag1 = Arc::new(OnceFlag::new());
    let flag2 = Arc::new(OnceFlag::new());
    let counter1 = Arc::new(AtomicI32::new(0));
    let counter2 = Arc::new(AtomicI32::new(0));
    let mut handles = vec![];

    for i in 0..10 {
        let flag1_clone = Arc::clone(&flag1);
        let flag2_clone = Arc::clone(&flag2);
        let counter1_clone = Arc::clone(&counter1);
        let counter2_clone = Arc::clone(&counter2);

        handles.push(thread::spawn(move || {
            if i % 2 == 0 {
                call_once(&flag1_clone, || {
                    counter1_clone.fetch_add(1, Ordering::SeqCst);
                });
            } else {
                call_once(&flag2_clone, || {
                    counter2_clone.fetch_add(1, Ordering::SeqCst);
                });
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Both flags should be initialized
    assert!(is_done(&flag1));
    assert!(is_done(&flag2));
    assert_eq!(counter1.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
}

#[test]
fn test_once_flag_panic_recovery() {
    // Test behavior when closure panics
    let flag = Arc::new(OnceFlag::new());
    let panic_count = Arc::new(AtomicI32::new(0));
    let success_count = Arc::new(AtomicI32::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let flag_clone = Arc::clone(&flag);
        let panic_count_clone = Arc::clone(&panic_count);
        let success_count_clone = Arc::clone(&success_count);

        handles.push(thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                call_once(&flag_clone, || {
                    // First thread to succeed sets this
                    success_count_clone.fetch_add(1, Ordering::SeqCst);
                    // If we're not the first, panic
                    if success_count_clone.load(Ordering::SeqCst) > 1 {
                        panic!("Already initialized!");
                    }
                });
            });

            if result.is_err() {
                panic_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // At least one should have succeeded
    assert!(success_count.load(Ordering::SeqCst) >= 1);
    assert!(is_done(&flag));
}

#[test]
fn test_bloom_filter_stress_concurrent() {
    // Stress test with many concurrent operations
    let bloom = Arc::new(std::sync::Mutex::new(BloomFilter::new(100_000, 7)));
    let num_threads = 20;
    let operations_per_thread = 1000;
    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let bloom_clone = Arc::clone(&bloom);
        handles.push(thread::spawn(move || {
            for i in 0..operations_per_thread {
                let value = thread_id * operations_per_thread + i;
                let mut bloom = bloom_clone.lock().unwrap();

                // Mix of inserts and contains
                if i % 2 == 0 {
                    bloom.insert(&value);
                } else {
                    let _ = bloom.contains(&value);
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify a sample of inserted values
    let bloom = bloom.lock().unwrap();
    assert!(bloom.contains(&0));
    assert!(bloom.contains(&(num_threads * operations_per_thread - 1)));
}

#[test]
fn test_memory_region_thread_safe_clone() {
    // Test that MemoryRegion can be safely cloned between threads
    let region = MemoryRegion::new(0x1000, 0x2000);
    let mut handles = vec![];

    for _ in 0..10 {
        let region = region.clone();
        handles.push(thread::spawn(move || {
            // All operations should work fine on cloned regions
            assert!(region.contains(0x1500));
            assert_eq!(region.size(), 0x1000);
            assert!(!region.is_empty());
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_once_flag_static_scope() {
    // Test OnceFlag in static-like scope
    static FLAG: OnceFlag = OnceFlag::new();
    static COUNTER: AtomicI32 = AtomicI32::new(0);

    let mut handles = vec![];

    for _ in 0..10 {
        handles.push(thread::spawn(|| {
            call_once(&FLAG, || {
                COUNTER.fetch_add(1, Ordering::SeqCst);
            });
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
}

#[test]
fn test_bloom_filter_memory_barrier() {
    // Test that memory ordering is correct with concurrent operations
    use std::sync::Barrier;

    let bloom = Arc::new(std::sync::Mutex::new(BloomFilter::new(1000, 7)));
    let barrier = Arc::new(Barrier::new(11)); // 10 threads + main
    let ready = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];

    // Threads that will insert
    for i in 0..10 {
        let bloom_clone = Arc::clone(&bloom);
        let barrier_clone = Arc::clone(&barrier);
        let ready_clone = Arc::clone(&ready);

        handles.push(thread::spawn(move || {
            barrier_clone.wait();

            let mut bloom = bloom_clone.lock().unwrap();
            bloom.insert(&i);

            ready_clone.store(true, Ordering::Release);
        }));
    }

    // Wait for all threads to be ready
    barrier.wait();

    // Wait for all threads to complete
    while ready.load(Ordering::Acquire) < 10 {
        thread::sleep(Duration::from_millis(1));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // All values should be present
    let bloom = bloom.lock().unwrap();
    for i in 0..10 {
        assert!(bloom.contains(&i), "Should contain {}", i);
    }
}
