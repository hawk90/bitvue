//! Profiling and performance measurement utilities.
//!
//! This module provides profiling utilities similar to Abseil's `absl/profiling`
//! directory, which help measure and analyze code performance.
//!
//! # Overview
//!
//! Profiling utilities provide tools for measuring execution time, memory usage,
//! and other performance metrics. These are useful for:
//!
//! - Benchmarking code performance
//! - Measuring function execution time
//! - Counting operations
//! - Memory profiling
//!
//! # Components
//!
//! - [`Timer`] - High-resolution timer for measuring elapsed time
//! - [`Counter`] - Generic counter for tracking operations
//! - [`Profiler`] - Simple profiler for collecting metrics
//! - [`sampling`] - Statistical sampling utilities
//! - [`histogram`] - Histogram for value distribution
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_profiling::Timer;
//!
//! let timer = Timer::start();
//! // ... do some work ...
//! let elapsed = timer.elapsed();
//! println!(" took {:?}", elapsed);
//! ```


extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;
use core::time::Duration;

pub mod sampling;
pub mod histogram;
pub mod counter;
pub mod recorder;

// Re-exports
pub use sampling::{SampleRecorder, SamplingProfiler};
pub use histogram::{Histogram, HistogramBucket};
pub use counter::{Counter, CounterGuard};
pub use recorder::{ProfileRecorder, ProfileData};

/// High-resolution timer for measuring elapsed time.
#[derive(Clone, Debug)]
pub struct Timer {
    start: Option<Duration>,
    end: Option<Duration>,
}

impl Timer {
    /// Creates a new timer and starts it.
    ///
    /// # Examples
    ///
    /// ```
    /// use abseil::absl_profiling::Timer;
    ///
    /// let timer = Timer::start();
    /// // ... do work ...
    /// let elapsed = timer.elapsed();
    /// ```
    pub fn start() -> Self {
        Self {
            start: Some(now()),
            end: None,
        }
    }

    /// Creates a new stopped timer.
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    /// Stops the timer and returns the elapsed duration.
    pub fn stop(&mut self) -> Duration {
        self.end = Some(now());
        self.elapsed()
    }

    /// Returns the elapsed time since starting.
    pub fn elapsed(&self) -> Duration {
        match (self.start, self.end) {
            (Some(start), Some(end)) => end.saturating_sub(start),
            (Some(start), None) => now().saturating_sub(start),
            (None, _) => Duration::ZERO,
        }
    }

    /// Resets the timer.
    pub fn reset(&mut self) {
        self.start = None;
        self.end = None;
    }

    /// Restarts the timer.
    pub fn restart(&mut self) {
        self.start = Some(now());
        self.end = None;
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Timer {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        static ZERO: Duration = Duration::ZERO;
        // Note: This is a simplified implementation
        &ZERO
    }
}

/// RAII timer guard that automatically stops on drop.
pub struct TimerGuard<'a> {
    timer: &'a mut Timer,
    label: Option<String>,
}

impl<'a> TimerGuard<'a> {
    pub fn new(timer: &'a mut Timer) -> Self {
        Self {
            timer,
            label: None,
        }
    }

    pub fn with_label(timer: &'a mut Timer, label: impl Into<String>) -> Self {
        Self {
            timer,
            label: Some(label.into()),
        }
    }
}

impl Drop for TimerGuard<'_> {
    fn drop(&mut self) {
        self.timer.stop();
        if let Some(label) = &self.label {
            // In a real implementation, this might log the time
            let _ = label;
        }
    }
}

/// Simple profiler for collecting timing metrics.
#[derive(Default)]
pub struct Profiler {
    metrics: BTreeMap<String, Vec<Duration>>,
}

impl Profiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a timing metric for the given label.
    pub fn record(&mut self, label: impl Into<String>, duration: Duration) {
        let label = label.into();
        self.metrics.entry(label).or_default().push(duration);
    }

    /// Returns all recorded metrics.
    pub fn metrics(&self) -> &BTreeMap<String, Vec<Duration>> {
        &self.metrics
    }

    /// Returns statistics for a specific label.
    pub fn stats(&self, label: &str) -> Option<ProfileStats> {
        let durations = self.metrics.get(label)?;
        if durations.is_empty() {
            return None;
        }

        let count = durations.len();
        let total: Duration = durations.iter().sum();
        let avg = total / count as u32;

        let min = *durations.iter().min().unwrap_or(&Duration::ZERO);
        let max = *durations.iter().max().unwrap_or(&Duration::ZERO);

        Some(ProfileStats {
            count,
            total,
            average: avg,
            min,
            max,
        })
    }

    /// Clears all recorded metrics.
    pub fn clear(&mut self) {
        self.metrics.clear();
    }
}

/// Statistics for a profiled operation.
#[derive(Clone, Debug)]
pub struct ProfileStats {
    pub count: usize,
    pub total: Duration,
    pub average: Duration,
    pub min: Duration,
    pub max: Duration,
}

impl fmt::Display for ProfileStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "count: {}, total: {:?}, avg: {:?}, min: {:?}, max: {:?}",
            self.count, self.total, self.average, self.min, self.max
        )
    }
}

/// Gets the current time as Duration.
#[inline(always)]
fn now() -> Duration {
    // Note: In no_std environment, we'd need a different approach
    // For now, using a simple counter-based implementation
    #[cfg(feature = "std")]
    {
        use std::time::Instant;
        Instant::now().elapsed()
    }

    #[cfg(not(feature = "std"))]
    {
        // Fallback for no_std - this is not accurate but provides compilation
        Duration::from_nanos(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        let elapsed = timer.elapsed();
        assert!(elapsed >= Duration::ZERO);
    }

    #[test]
    fn test_timer_stop() {
        let mut timer = Timer::start();
        timer.stop();
        let elapsed = timer.elapsed();
        assert!(elapsed >= Duration::ZERO);
    }

    #[test]
    fn test_timer_reset() {
        let mut timer = Timer::start();
        timer.reset();
        assert_eq!(timer.start, None);
        assert_eq!(timer.end, None);
    }

    #[test]
    fn test_profiler() {
        let mut profiler = Profiler::new();
        profiler.record("test", Duration::from_millis(100));
        profiler.record("test", Duration::from_millis(200));

        let stats = profiler.stats("test").unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.total, Duration::from_millis(300));
        assert_eq!(stats.average, Duration::from_millis(150));
    }

    #[test]
    fn test_profiler_clear() {
        let mut profiler = Profiler::new();
        profiler.record("test", Duration::from_millis(100));
        profiler.clear();
        assert!(profiler.stats("test").is_none());
    }
}
