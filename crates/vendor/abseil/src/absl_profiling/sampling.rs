//! Statistical sampling utilities.

extern crate alloc;

use alloc::vec::Vec;

/// Records samples for statistical analysis.
pub struct SampleRecorder<T> {
    samples: Vec<T>,
    max_samples: usize,
}

impl<T: Clone> SampleRecorder<T> {
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
        }
    }

    pub fn record(&mut self, sample: T) {
        if self.samples.len() < self.max_samples {
            self.samples.push(sample);
        }
    }

    pub fn samples(&self) -> &[T] {
        &self.samples
    }

    pub fn count(&self) -> usize {
        self.samples.len()
    }

    pub fn is_full(&self) -> bool {
        self.samples.len() >= self.max_samples
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

/// Statistical sampling profiler.
pub struct SamplingProfiler {
    interval_us: u64,
}

impl SamplingProfiler {
    pub fn new(interval_us: u64) -> Self {
        Self { interval_us }
    }

    pub fn interval(&self) -> u64 {
        self.interval_us
    }

    pub fn set_interval(&mut self, interval_us: u64) {
        self.interval_us = interval_us;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_recorder() {
        let mut recorder = SampleRecorder::new(10);
        recorder.record(42);
        assert_eq!(recorder.count(), 1);
        assert_eq!(recorder.samples()[0], 42);
    }

    #[test]
    fn test_sample_recorder_full() {
        let mut recorder = SampleRecorder::new(2);
        recorder.record(1);
        recorder.record(2);
        assert!(recorder.is_full());
        recorder.record(3); // Should be dropped
        assert_eq!(recorder.count(), 2);
    }

    #[test]
    fn test_sampling_profiler() {
        let profiler = SamplingProfiler::new(1000);
        assert_eq!(profiler.interval(), 1000);
    }
}
