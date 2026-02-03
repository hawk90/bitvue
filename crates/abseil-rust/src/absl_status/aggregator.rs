//! Status aggregation for combining multiple errors into one.

use alloc::vec::Vec;

use super::status::Status;
use super::StatusCode;

/// Aggregates multiple statuses into a single status.
#[derive(Clone, Debug)]
pub struct StatusAggregator {
    statuses: Vec<Status>,
    strategy: AggregationStrategy,
}

/// Strategy for aggregating multiple statuses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AggregationStrategy {
    /// Return the first error (ignore others).
    FirstError,
    /// Return the last error.
    LastError,
    /// Return the most severe error.
    MostSevere,
    /// Aggregate all messages into one.
    AllMessages,
}

impl Default for StatusAggregator {
    fn default() -> Self {
        Self {
            statuses: Vec::new(),
            strategy: AggregationStrategy::FirstError,
        }
    }
}

impl StatusAggregator {
    /// Creates a new aggregator with the given strategy.
    pub fn new(strategy: AggregationStrategy) -> Self {
        Self {
            statuses: Vec::new(),
            strategy,
        }
    }

    /// Adds a status to the aggregation.
    pub fn add(&mut self, status: Status) {
        if !status.is_ok() {
            self.statuses.push(status);
        }
    }

    /// Adds multiple statuses.
    pub fn add_all(&mut self, statuses: impl IntoIterator<Item = Status>) {
        for status in statuses {
            self.add(status);
        }
    }

    /// Returns the aggregated status.
    pub fn aggregate(&self) -> Status {
        if self.statuses.is_empty() {
            return Status::ok();
        }

        match self.strategy {
            AggregationStrategy::FirstError => self.statuses.first().cloned().unwrap_or_else(Status::ok),
            AggregationStrategy::LastError => self.statuses.last().cloned().unwrap_or_else(Status::ok),
            AggregationStrategy::MostSevere => {
                // Define severity order (lower = more severe)
                let severity_order = |code: StatusCode| -> u8 {
                    match code {
                        StatusCode::Internal => 0,
                        StatusCode::DataLoss => 1,
                        StatusCode::Unknown => 2,
                        StatusCode::Unauthenticated => 3,
                        StatusCode::PermissionDenied => 4,
                        StatusCode::ResourceExhausted => 5,
                        StatusCode::FailedPrecondition => 6,
                        StatusCode::OutOfRange => 7,
                        StatusCode::InvalidArgument => 8,
                        StatusCode::NotFound => 9,
                        StatusCode::AlreadyExists => 10,
                        StatusCode::Unavailable => 11,
                        StatusCode::DeadlineExceeded => 12,
                        StatusCode::Cancelled => 13,
                        StatusCode::Unimplemented => 14,
                        StatusCode::Aborted => 15,
                        StatusCode::Ok => 16,
                    }
                };

                self.statuses
                    .iter()
                    .min_by_key(|s| severity_order(s.code()))
                    .cloned()
                    .unwrap_or_else(Status::ok)
            }
            AggregationStrategy::AllMessages => {
                let code = self.statuses[0].code();
                let messages = self.statuses
                    .iter()
                    .map(|s| s.message())
                    .collect::<Vec<_>>()
                    .join("; ");
                Status::new(code, format!("Multiple errors: {}", messages))
            }
        }
    }

    /// Returns the number of errors in the aggregation.
    pub fn error_count(&self) -> usize {
        self.statuses.len()
    }

    /// Returns true if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.statuses.is_empty()
    }

    /// Clears all accumulated statuses.
    pub fn clear(&mut self) {
        self.statuses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // StatusAggregator tests
    #[test]
    fn test_status_aggregator_first_error() {
        let mut aggregator = StatusAggregator::new(AggregationStrategy::FirstError);
        aggregator.add(Status::new(StatusCode::Internal, "Error 1"));
        aggregator.add(Status::new(StatusCode::NotFound, "Error 2"));

        let result = aggregator.aggregate();
        assert_eq!(result.code(), StatusCode::Internal);
    }

    #[test]
    fn test_status_aggregator_last_error() {
        let mut aggregator = StatusAggregator::new(AggregationStrategy::LastError);
        aggregator.add(Status::new(StatusCode::Internal, "Error 1"));
        aggregator.add(Status::new(StatusCode::NotFound, "Error 2"));

        let result = aggregator.aggregate();
        assert_eq!(result.code(), StatusCode::NotFound);
    }

    #[test]
    fn test_status_aggregator_all_messages() {
        let mut aggregator = StatusAggregator::new(AggregationStrategy::AllMessages);
        aggregator.add(Status::new(StatusCode::Internal, "Error 1"));
        aggregator.add(Status::new(StatusCode::Internal, "Error 2"));

        let result = aggregator.aggregate();
        assert!(result.message().contains("Error 1"));
        assert!(result.message().contains("Error 2"));
    }

    #[test]
    fn test_status_aggregator_empty() {
        let aggregator = StatusAggregator::new(AggregationStrategy::FirstError);
        let result = aggregator.aggregate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_aggregator_error_count() {
        let mut aggregator = StatusAggregator::new(AggregationStrategy::FirstError);
        assert_eq!(aggregator.error_count(), 0);
        assert!(!aggregator.has_errors());

        aggregator.add(Status::new(StatusCode::Internal, "Error"));
        assert_eq!(aggregator.error_count(), 1);
        assert!(aggregator.has_errors());
    }

    #[test]
    fn test_status_aggregator_clear() {
        let mut aggregator = StatusAggregator::new(AggregationStrategy::FirstError);
        aggregator.add(Status::new(StatusCode::Internal, "Error"));
        aggregator.clear();
        assert_eq!(aggregator.error_count(), 0);
    }
}
