//! Counter utilities for tracking operations.

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A simple counter.
#[derive(Debug)]
pub struct Counter {
    count: Arc<AtomicUsize>,
    label: Option<String>,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(0)),
            label: None,
        }
    }

    pub fn with_label(label: impl Into<String>) -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(0)),
            label: Some(label.into()),
        }
    }

    pub fn increment(&self) -> usize {
        self.count.fetch_add(1, Ordering::Relaxed)
    }

    pub fn add(&self, amount: usize) -> usize {
        self.count.fetch_add(amount, Ordering::Relaxed)
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    pub fn reset(&self) -> usize {
        self.count.swap(0, Ordering::Relaxed)
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        Self {
            count: Arc::clone(&self.count),
            label: self.label.clone(),
        }
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that increments a counter on drop.
pub struct CounterGuard {
    counter: Counter,
}

impl CounterGuard {
    pub fn new(counter: Counter) -> Self {
        Self { counter }
    }
}

impl Drop for CounterGuard {
    fn drop(&mut self) {
        self.counter.increment();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);
        counter.increment();
        assert_eq!(counter.get(), 1);
        counter.add(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_counter_reset() {
        let counter = Counter::new();
        counter.increment();
        assert_eq!(counter.reset(), 1);
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_counter_clone() {
        let counter = Counter::new();
        let counter2 = counter.clone();
        counter.increment();
        assert_eq!(counter2.get(), 1);
    }

    #[test]
    fn test_counter_guard() {
        let counter = Counter::new();
        {
            let _guard = CounterGuard::new(counter.clone());
        }
        assert_eq!(counter.get(), 1);
    }
}
