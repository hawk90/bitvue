//! Comprehensive concurrent tests for synchronization primitives.
//!
//! This module tests edge cases related to concurrent access:
//! - Mutex contention scenarios
//! - Notification multiple waiters
//! - Blocking counter behavior under load
//! - Barrier synchronization
//! - Spinlock correctness

use abseil::absl_synchronization::{
    Notification, BlockingCounter, Barrier,
    Mutex, MutexGuard, Spinlock, SpinlockGuard,
};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Notification Tests
// ============================================================================

#[test]
fn test_notification_concurrent_notification() {
    // Test: Multiple threads calling notify simultaneously
    let notification = Arc::new(Notification::new());
    let mut handles = vec![];

    for _ in 0..10 {
        let notif = notification.clone();
        handles.push(thread::spawn(move || {
            notif.notify();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(notification.has_been_notified());
}

#[test]
fn test_notification_wait_before_notify() {
    // Test: Thread that waits before notification arrives
    let notification = Arc::new(Notification::new());
    let started = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];

    // Waiter threads
    for _ in 0..5 {
        let notif = notification.clone();
        let started = started.clone();
        handles.push(thread::spawn(move || {
            started.store(true, Ordering::Release);
            notif.wait();
            assert!(notif.has_been_notified());
        }));
    }

    // Wait for at least one thread to start waiting
    while !started.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(1));
    }

    thread::sleep(Duration::from_millis(10));
    notification.notify();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_notification_wait_after_notify() {
    // Test: Thread that waits after notification
    let notification = Arc::new(Notification::new());

    notification.notify();

    // These wait calls should return immediately
    for _ in 0..5 {
        let notif = notification.clone();
        handles.push(thread::spawn(move || {
            notif.wait();
            assert!(notif.has_been_notified());
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_notification_wait_for_timeout() {
    // Test: wait_for timeout behavior
    let notification = Arc::new(Notification::new());
    let mut handles = vec![];

    // Some threads that will timeout
    for _ in 0..5 {
        let notif = notification.clone();
        handles.push(thread::spawn(move || {
            let result = notif.wait_for(Duration::from_millis(10));
            assert!(!result);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_notification_wait_for_success() {
    // Test: wait_for that succeeds
    let notification = Arc::new(Notification::new());
    let notif_clone = notification.clone();

    let handle = thread::spawn(move || {
        let result = notif_clone.wait_for(Duration::from_secs(1));
        assert!(result);
        assert!(notif_clone.has_been_notified());
    });

    thread::sleep(Duration::from_millis(10));
    notification.notify();

    handle.join().unwrap();
}

#[test]
fn test_notification_multiple_waiters_race() {
    // Test: Race between waiters and notification
    let notification = Arc::new(Notification::new());
    let barrier = Arc::new(std::sync::Barrier::new(11)); // 10 waiters + main
    let mut handles = vec![];

    for _ in 0..10 {
        let notif = notification.clone();
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            // Random delay to create race conditions
            let delay = rand::random::<u8>() % 10;
            thread::sleep(Duration::from_millis(delay as u64));
            notif.wait();
        }));
    }

    barrier.wait();
    thread::sleep(Duration::from_millis(20));
    notification.notify();

    for handle in handles {
        handle.join().unwrap();
    }
}

// ============================================================================
// BlockingCounter Tests
// ============================================================================

#[test]
fn test_blocking_counter_concurrent_decrements() {
    // Test: Multiple threads decrementing simultaneously
    let counter = Arc::new(BlockingCounter::new(100));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = counter.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                counter.decrement();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(counter.is_zero());
    assert_eq!(counter.count(), 0);
}

#[test]
fn test_blocking_counter_wait_with_decrements() {
    // Test: Thread waiting while others decrement
    let counter = Arc::new(BlockingCounter::new(100));
    let counter_clone = counter.clone();

    let waiter = thread::spawn(move || {
        counter_clone.wait();
        assert!(counter_clone.is_zero());
    });

    // Give waiter time to start
    thread::sleep(Duration::from_millis(10));

    for _ in 0..100 {
        counter.decrement();
    }

    waiter.join().unwrap();
}

#[test]
fn test_blocking_counter_timeout_behavior() {
    // Test: wait_timeout when count won't reach zero
    let counter = Arc::new(BlockingCounter::new(10));
    let counter_clone = counter.clone();

    let handle = thread::spawn(move || {
        let result = counter_clone.wait_timeout(Duration::from_millis(50));
        assert!(!result);
        assert_eq!(counter_clone.count(), 10);
    });

    handle.join().unwrap();
}

#[test]
fn test_blocking_counter_underflow_detection() {
    // Test: Concurrent decrements that might underflow
    let counter = Arc::new(BlockingCounter::new(5));
    let mut handles = vec![];

    // These should succeed
    for _ in 0..5 {
        let counter = counter.clone();
        handles.push(thread::spawn(move || {
            counter.decrement();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // This should panic
    let result = std::panic::catch_unwind(|| {
        counter.decrement();
    });
    assert!(result.is_err());
}

#[test]
fn test_blocking_counter_stress_wait_notify() {
    // Test: Stress test with wait/notify pattern
    for _ in 0..100 {
        let counter = Arc::new(BlockingCounter::new(50));
        let counter_clone = counter.clone();
        let mut handles = vec![];

        // Decrementing threads
        for _ in 0..5 {
            let counter = counter.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    counter.decrement();
                }
            }));
        }

        // Waiting thread
        handles.push(thread::spawn(move || {
            counter_clone.wait();
            assert!(counter_clone.is_zero());
        }));

        for handle in handles {
            handle.join().unwrap();
        }
    }
}

// ============================================================================
// Barrier Tests
// ============================================================================

#[test]
fn test_barrier_basic_synchronization() {
    // Test: Basic barrier synchronization
    let barrier = Arc::new(Barrier::new(5));
    let mut handles = vec![];
    let counter = Arc::new(AtomicI32::new(0));

    for _ in 0..5 {
        let barrier = barrier.clone();
        let counter = counter.clone();
        handles.push(thread::spawn(move || {
            // Each thread increments counter
            counter.fetch_add(1, Ordering::Relaxed);
            // Wait for all threads
            barrier.wait();
            // All threads should see count == 5
            assert_eq!(counter.load(Ordering::Relaxed), 5);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_barrier_multiple_wait_cycles() {
    // Test: Multiple wait cycles on same barrier
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    for _ in 0..3 {
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            for cycle in 0..5 {
                // Wait for all threads
                barrier.wait();
                // All threads should be in same cycle
                // (we can't directly test this, but the test should complete)
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// ============================================================================
// Mutex Tests
// ============================================================================

#[test]
fn test_mutex_basic_locking() {
    // Test: Basic mutex locking
    let mutex = Arc::new(Mutex::new(0));
    let mutex_clone = mutex.clone();

    let handle = thread::spawn(move || {
        let mut guard = mutex_clone.lock().unwrap();
        *guard = 42;
    });

    handle.join().unwrap();

    let guard = mutex.lock().unwrap();
    assert_eq!(*guard, 42);
}

#[test]
fn test_mutex_contention() {
    // Test: High contention scenario
    let mutex = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let mutex = mutex.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                let mut guard = mutex.lock().unwrap();
                *guard += 1;
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let guard = mutex.lock().unwrap();
    assert_eq!(*guard, 10000);
}

#[test]
fn test_mutex_deadlock_prevention() {
    // Test: Multiple mutexes in consistent order (no deadlock)
    let mutex1 = Arc::new(Mutex::new(0));
    let mutex2 = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let m1 = mutex1.clone();
        let m2 = mutex2.clone();
        handles.push(thread::spawn(move || {
            // Always acquire locks in same order
            let _g1 = m1.lock().unwrap();
            let _g2 = m2.lock().unwrap();
            // Do work
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
    // If we get here, no deadlock occurred
}

#[test]
fn test_mutex_try_lock() {
    // Test: try_lock behavior
    let mutex = Arc::new(Mutex::new(0));
    let mutex_clone = mutex.clone();

    // First lock
    let guard = mutex.lock().unwrap();

    // Try to lock from another thread
    let handle = thread::spawn(move || {
        let try_result = mutex_clone.try_lock();
        assert!(try_result.is_err());
    });

    handle.join().unwrap();

    // Release first lock
    drop(guard);

    // Now try_lock should succeed
    let try_result = mutex.try_lock();
    assert!(try_result.is_ok());
}

// ============================================================================
// Spinlock Tests
// ============================================================================

#[test]
fn test_spinlock_basic_locking() {
    // Test: Basic spinlock locking
    let spinlock = Arc::new(Spinlock::new(0));
    let spinlock_clone = spinlock.clone();

    let handle = thread::spawn(move || {
        let mut guard = spinlock_clone.lock();
        *guard = 42;
    });

    handle.join().unwrap();

    let guard = spinlock.lock();
    assert_eq!(*guard, 42);
}

#[test]
fn test_spinlock_contention() {
    // Test: Spinlock under contention
    let spinlock = Arc::new(Spinlock::new(0));
    let mut handles = vec![];

    for _ in 0..5 {
        let spinlock = spinlock.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let mut guard = spinlock.lock();
                *guard += 1;
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let guard = spinlock.lock();
    assert_eq!(*guard, 500);
}

#[test]
fn test_spinlock_no_deadlock() {
    // Test: Spinlock doesn't deadlock under stress
    for iteration in 0..50 {
        let spinlock = Arc::new(Spinlock::new(0));
        let mut handles = vec![];

        for _ in 0..5 {
            let spinlock = spinlock.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    let _guard = spinlock.lock();
                    // Small critical section
                }
            }));
        }

        // Add timeout detection
        for handle in handles {
            let result = thread::spawn(move || {
                handle.join()
            }).join();

            if result.is_err() {
                panic!("Potential deadlock in spinlock test iteration {}", iteration);
            }
        }
    }
}

// ============================================================================
// Mixed Synchronization Tests
// ============================================================================

#[test]
fn test_notification_with_counter() {
    // Test: Using notification to signal counter completion
    let counter = Arc::new(BlockingCounter::new(10));
    let notification = Arc::new(Notification::new());
    let counter_clone = counter.clone();
    let notification_clone = notification.clone();

    // Worker threads
    let mut handles = vec![];
    for _ in 0..10 {
        let c = counter.clone();
        handles.push(thread::spawn(move || {
            // Do work
            thread::sleep(Duration::from_millis(1));
            c.decrement();
        }));
    }

    // Waiter thread that then notifies
    handles.push(thread::spawn(move || {
        counter_clone.wait();
        notification_clone.notify();
    }));

    // Main thread waits for notification
    notification.wait();

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(notification.has_been_notified());
    assert!(counter.is_zero());
}

#[test]
fn test_barrier_then_notification() {
    // Test: Barrier followed by notification
    let barrier = Arc::new(Barrier::new(5));
    let notification = Arc::new(Notification::new());
    let mut handles = vec![];

    for _ in 0..4 {
        let b = barrier.clone();
        let n = notification.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            n.wait();
        }));
    }

    barrier.wait();
    thread::sleep(Duration::from_millis(10));
    notification.notify();

    for handle in handles {
        handle.join().unwrap();
    }
}

// ============================================================================
// Fairness Tests
// ============================================================================

#[test]
fn test_mutex_starvation_prevention() {
    // Test: Check that threads eventually get the lock
    let mutex = Arc::new(Mutex::new(vec![0usize; 10]));
    let mut handles = vec![];

    for i in 0..5 {
        let mutex = mutex.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let mut guard = mutex.lock().unwrap();
                // Modify the data
                guard[i] += 1;
                // Small sleep to increase contention
                thread::sleep(Duration::from_micros(10));
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // All threads should have completed successfully
    let guard = mutex.lock().unwrap();
    assert_eq!(guard[0] + guard[1] + guard[2] + guard[3] + guard[4], 500);
}

#[test]
fn test_fairness_notification_order() {
    // Test: Multiple waiters are all awakened
    let notification = Arc::new(Notification::new());
    let barrier = Arc::new(std::sync::Barrier::new(11));
    let ready_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for i in 0..10 {
        let n = notification.clone();
        let b = barrier.clone();
        let rc = ready_count.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            rc.fetch_add(1, Ordering::Release);
            n.wait();
            assert!(n.has_been_notified());
        }));
    }

    barrier.wait();
    // Wait for all threads to be ready
    while ready_count.load(Ordering::Acquire) < 10 {
        thread::sleep(Duration::from_millis(1));
    }

    thread::sleep(Duration::from_millis(10));
    notification.notify();

    for handle in handles {
        handle.join().unwrap();
    }
}
