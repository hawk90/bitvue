//! Histogram for value distribution analysis.

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;
use core::ops::RangeInclusive;

/// Error type for Histogram operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HistogramError {
    /// Number of buckets must be at least 1.
    InvalidNumBuckets,
    /// Minimum value must be less than maximum value.
    InvalidRange,
}

impl fmt::Display for HistogramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HistogramError::InvalidNumBuckets => {
                write!(f, "num_buckets must be at least 1")
            }
            HistogramError::InvalidRange => {
                write!(f, "min must be less than max")
            }
        }
    }
}

/// A histogram bucket.
#[derive(Clone, Debug)]
pub struct HistogramBucket {
    pub range: RangeInclusive<f64>,
    pub count: usize,
}

/// A histogram for tracking value distributions.
#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<HistogramBucket>,
    out_of_range: usize,
}

impl Histogram {
    /// Creates a new histogram with uniform bucket sizes.
    ///
    /// # Panics
    ///
    /// Panics if `num_buckets` is 0 or `min >= max`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_profiling::histogram::Histogram;
    ///
    /// let mut hist = Histogram::new(0.0, 10.0, 5);
    /// hist.add(2.5);
    /// ```
    pub fn new(min: f64, max: f64, num_buckets: usize) -> Self {
        Self::try_new(min, max, num_buckets).unwrap()
    }

    /// Creates a new histogram with validation.
    ///
    /// Returns `Err` if `num_buckets` is 0 or `min >= max`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_profiling::histogram::Histogram;
    ///
    /// assert!(Histogram::try_new(0.0, 10.0, 5).is_ok());
    /// assert!(Histogram::try_new(10.0, 10.0, 5).is_err()); // min >= max
    /// assert!(Histogram::try_new(0.0, 10.0, 0).is_err()); // num_buckets == 0
    /// ```
    pub fn try_new(min: f64, max: f64, num_buckets: usize) -> Result<Self, HistogramError> {
        if num_buckets == 0 {
            return Err(HistogramError::InvalidNumBuckets);
        }
        if min >= max {
            return Err(HistogramError::InvalidRange);
        }

        let bucket_size = (max - min) / num_buckets as f64;
        let mut buckets = Vec::with_capacity(num_buckets);

        for i in 0..num_buckets {
            let bucket_min = min + i as f64 * bucket_size;
            let bucket_max = if i == num_buckets - 1 {
                max
            } else {
                min + (i + 1) as f64 * bucket_size
            };
            buckets.push(HistogramBucket {
                range: RangeInclusive::new(bucket_min, bucket_max),
                count: 0,
            });
        }

        Ok(Self {
            buckets,
            out_of_range: 0,
        })
    }

    /// Adds a value to the histogram.
    pub fn add(&mut self, value: f64) {
        for bucket in &mut self.buckets {
            if bucket.range.contains(&value) {
                bucket.count += 1;
                return;
            }
        }
        self.out_of_range += 1;
    }

    /// Returns all buckets.
    pub fn buckets(&self) -> &[HistogramBucket] {
        &self.buckets
    }

    /// Returns the number of out-of-range values.
    pub fn out_of_range(&self) -> usize {
        self.out_of_range
    }

    /// Returns the total count of all values.
    pub fn total_count(&self) -> usize {
        self.buckets.iter().map(|b| b.count).sum()
    }

    /// Clears all counts.
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.count = 0;
        }
        self.out_of_range = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram() {
        let mut hist = Histogram::new(0.0, 10.0, 5);
        hist.add(2.5);
        hist.add(5.0);
        hist.add(7.5);
        hist.add(15.0); // out of range

        assert_eq!(hist.total_count(), 3);
        assert_eq!(hist.out_of_range(), 1);
    }

    #[test]
    fn test_histogram_clear() {
        let mut hist = Histogram::new(0.0, 10.0, 5);
        hist.add(5.0);
        hist.clear();
        assert_eq!(hist.total_count(), 0);
    }

    // Tests for HIGH security fix - division by zero prevention

    #[test]
    fn test_try_new_valid_inputs() {
        assert!(Histogram::try_new(0.0, 10.0, 5).is_ok());
        assert!(Histogram::try_new(-10.0, 10.0, 1).is_ok());
        assert!(Histogram::try_new(0.0, f64::MAX, 100).is_ok());
    }

    #[test]
    fn test_try_new_zero_num_buckets() {
        let result = Histogram::try_new(0.0, 10.0, 0);
        assert!(result.is_err());
        matches!(result, Err(HistogramError::InvalidNumBuckets));
    }

    #[test]
    fn test_try_new_min_equals_max() {
        let result = Histogram::try_new(10.0, 10.0, 5);
        assert!(result.is_err());
        matches!(result, Err(HistogramError::InvalidRange));
    }

    #[test]
    fn test_try_new_min_greater_than_max() {
        let result = Histogram::try_new(10.0, 0.0, 5);
        assert!(result.is_err());
        matches!(result, Err(HistogramError::InvalidRange));
    }

    #[test]
    fn test_histogram_error_display() {
        assert_eq!(
            format!("{}", HistogramError::InvalidNumBuckets),
            "num_buckets must be at least 1"
        );
        assert_eq!(
            format!("{}", HistogramError::InvalidRange),
            "min must be less than max"
        );
    }

    #[test]
    fn test_histogram_single_bucket() {
        let mut hist = Histogram::new(0.0, 10.0, 1);
        hist.add(5.0);
        hist.add(10.0);
        assert_eq!(hist.total_count(), 2);
        assert_eq!(hist.out_of_range(), 0);
    }

    #[test]
    fn test_histogram_boundary_values() {
        let mut hist = Histogram::new(0.0, 100.0, 4);

        // Add values at each bucket boundary
        hist.add(0.0); // First bucket
        hist.add(25.0); // Second bucket
        hist.add(50.0); // Third bucket
        hist.add(75.0); // Fourth bucket
        hist.add(100.0); // Fourth bucket (max inclusive)

        assert_eq!(hist.total_count(), 5);
    }

    #[test]
    fn test_histogram_negative_range() {
        let mut hist = Histogram::new(-100.0, 0.0, 4);
        hist.add(-75.0);
        hist.add(-50.0);
        hist.add(-25.0);
        assert_eq!(hist.total_count(), 3);
    }
}
