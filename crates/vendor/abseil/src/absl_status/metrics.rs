//! Metrics for tracking status occurrences.

use alloc::collections::BTreeMap;

use super::status::Status;
use super::status::StatusCode;

/// Simple metrics for status occurrences.
#[derive(Clone, Debug, Default)]
pub struct StatusMetrics {
    /// Count of each status code.
    code_counts: BTreeMap<StatusCode, usize>,
    /// Total status count.
    total_count: usize,
}

impl StatusMetrics {
    /// Creates a new metrics tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a status.
    pub fn record(&mut self, status: &Status) {
        *self.code_counts.entry(status.code()).or_insert(0) += 1;
        self.total_count += 1;
    }

    /// Returns the count for a specific status code.
    pub fn count(&self, code: StatusCode) -> usize {
        self.code_counts.get(&code).copied().unwrap_or(0)
    }

    /// Returns the total number of statuses recorded.
    pub fn total(&self) -> usize {
        self.total_count
    }

    /// Returns the number of errors (non-OK statuses).
    pub fn error_count(&self) -> usize {
        self.total_count - self.count(StatusCode::Ok)
    }

    /// Calculates the error rate (0.0 to 1.0).
    pub fn error_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.error_count() as f64 / self.total_count as f64
        }
    }

    /// Returns the most common status code.
    pub fn most_common(&self) -> Option<StatusCode> {
        self.code_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&code, _)| code)
    }

    /// Resets all metrics.
    pub fn reset(&mut self) {
        self.code_counts.clear();
        self.total_count = 0;
    }

    /// Returns an iterator over all recorded status codes and their counts.
    pub fn iter(&self) -> impl Iterator<Item = (StatusCode, usize)> + '_ {
        self.code_counts.iter().map(|(&code, &count)| (code, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_metrics_new() {
        let metrics = StatusMetrics::new();
        assert_eq!(metrics.total(), 0);
        assert_eq!(metrics.error_count(), 0);
    }

    #[test]
    fn test_status_metrics_record() {
        let mut metrics = StatusMetrics::new();
        metrics.record(&Status::new(StatusCode::Internal, "Error"));
        assert_eq!(metrics.total(), 1);
        assert_eq!(metrics.count(StatusCode::Internal), 1);
    }

    #[test]
    fn test_status_metrics_error_count() {
        let mut metrics = StatusMetrics::new();
        metrics.record(&Status::ok());
        metrics.record(&Status::new(StatusCode::Internal, "Error"));
        metrics.record(&Status::new(StatusCode::NotFound, "Not found"));

        assert_eq!(metrics.total(), 3);
        assert_eq!(metrics.error_count(), 2);
    }

    #[test]
    fn test_status_metrics_error_rate() {
        let mut metrics = StatusMetrics::new();
        metrics.record(&Status::ok());
        metrics.record(&Status::ok());
        metrics.record(&Status::new(StatusCode::Internal, "Error"));

        assert!((metrics.error_rate() - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_status_metrics_most_common() {
        let mut metrics = StatusMetrics::new();
        metrics.record(&Status::new(StatusCode::Internal, "Error"));
        metrics.record(&Status::new(StatusCode::Internal, "Error"));
        metrics.record(&Status::new(StatusCode::NotFound, "Not found"));

        assert_eq!(metrics.most_common(), Some(StatusCode::Internal));
    }

    #[test]
    fn test_status_metrics_reset() {
        let mut metrics = StatusMetrics::new();
        metrics.record(&Status::new(StatusCode::Internal, "Error"));
        metrics.reset();
        assert_eq!(metrics.total(), 0);
    }

    #[test]
    fn test_status_metrics_iter() {
        let mut metrics = StatusMetrics::new();
        metrics.record(&Status::new(StatusCode::Internal, "Error"));
        metrics.record(&Status::new(StatusCode::NotFound, "Not found"));

        let codes: Vec<_> = metrics.iter().map(|(code, _)| code).collect();
        assert_eq!(codes.len(), 2);
        assert!(codes.contains(&StatusCode::Internal));
        assert!(codes.contains(&StatusCode::NotFound));
    }
}
