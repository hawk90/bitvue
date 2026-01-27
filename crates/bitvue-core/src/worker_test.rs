// Worker module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;
use std::path::PathBuf;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test path
fn create_test_path() -> PathBuf {
    PathBuf::from("/tmp/test.ivf")
}

/// Create a test ParseContainer job
fn create_test_parse_job(stream_id: StreamId, request_id: u64) -> Job {
    Job::ParseContainer {
        stream_id,
        path: create_test_path(),
        request_id,
    }
}

/// Create a test DecodeFrame job
fn create_test_decode_job(stream_id: StreamId, frame_index: usize, request_id: u64) -> Job {
    Job::DecodeFrame {
        stream_id,
        frame_index,
        request_id,
    }
}

// ============================================================================
// Job Enum Tests
// ============================================================================

#[cfg(test)]
mod job_enum_tests {
    use super::*;

    #[test]
    fn test_job_parse_container() {
        // Arrange & Act
        let job = Job::ParseContainer {
            stream_id: StreamId::A,
            path: create_test_path(),
            request_id: 1,
        };

        // Assert
        assert!(matches!(job, Job::ParseContainer { .. }));
        assert_eq!(job.stream_id(), StreamId::A);
        assert_eq!(job.request_id(), 1);
    }

    #[test]
    fn test_job_parse_units() {
        // Arrange & Act
        let job = Job::ParseUnits {
            stream_id: StreamId::B,
            request_id: 2,
        };

        // Assert
        assert_eq!(job.stream_id(), StreamId::B);
        assert_eq!(job.request_id(), 2);
    }

    #[test]
    fn test_job_build_syntax_tree() {
        // Arrange & Act
        let job = Job::BuildSyntaxTree {
            stream_id: StreamId::A,
            unit_offset: 1000,
            request_id: 3,
        };

        // Assert
        assert_eq!(job.stream_id(), StreamId::A);
        assert_eq!(job.request_id(), 3);
    }

    #[test]
    fn test_job_decode_frame() {
        // Arrange & Act
        let job = Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: 10,
            request_id: 4,
        };

        // Assert
        assert!(matches!(job, Job::DecodeFrame { .. }));
        assert_eq!(job.stream_id(), StreamId::A);
        assert_eq!(job.request_id(), 4);
    }

    #[test]
    fn test_job_render_overlay() {
        // Arrange & Act
        let job = Job::RenderOverlay {
            stream_id: StreamId::B,
            frame_index: 20,
            request_id: 5,
        };

        // Assert
        assert!(matches!(job, Job::RenderOverlay { .. }));
    }

    #[test]
    fn test_job_is_decode_convert_decode() {
        // Arrange
        let job = Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: 10,
            request_id: 1,
        };

        // Act & Assert
        assert!(job.is_decode_convert());
    }

    #[test]
    fn test_job_is_decode_convert_overlay() {
        // Arrange
        let job = Job::RenderOverlay {
            stream_id: StreamId::A,
            frame_index: 10,
            request_id: 1,
        };

        // Act & Assert
        assert!(job.is_decode_convert());
    }

    #[test]
    fn test_job_is_decode_convert_parse() {
        // Arrange
        let job = Job::ParseContainer {
            stream_id: StreamId::A,
            path: create_test_path(),
            request_id: 1,
        };

        // Act & Assert
        assert!(!job.is_decode_convert());
    }
}

// ============================================================================
// JobPriority Tests
// ============================================================================

#[cfg(test)]
mod job_priority_tests {
    use super::*;

    #[test]
    fn test_job_priority_values() {
        // Arrange & Act
        let low = JobPriority::Low;
        let normal = JobPriority::Normal;
        let high = JobPriority::High;

        // Assert
        assert_eq!(low as u8, 1);
        assert_eq!(normal as u8, 2);
        assert_eq!(high as u8, 3);
    }

    #[test]
    fn test_job_priority_ordering() {
        // Arrange
        let low = JobPriority::Low;
        let normal = JobPriority::Normal;
        let high = JobPriority::High;

        // Assert
        assert!(low < normal);
        assert!(normal < high);
        assert!(low < high);
    }

    #[test]
    fn test_job_priority_equality() {
        // Arrange
        let normal1 = JobPriority::Normal;
        let normal2 = JobPriority::Normal;
        let high = JobPriority::High;

        // Assert
        assert_eq!(normal1, normal2);
        assert_ne!(normal1, high);
    }
}

// ============================================================================
// JobState Tests
// ============================================================================

#[cfg(test)]
mod job_state_tests {
    use super::*;

    #[test]
    fn test_job_state_all_values() {
        // Arrange & Act
        let states = [
            JobState::Queued,
            JobState::InFlight,
            JobState::Completed,
            JobState::Cancelled,
            JobState::Discarded,
        ];

        // Assert - All states exist
        assert_eq!(states.len(), 5);
    }

    #[test]
    fn test_job_state_equality() {
        // Arrange
        let queued1 = JobState::Queued;
        let queued2 = JobState::Queued;
        let in_flight = JobState::InFlight;

        // Assert
        assert_eq!(queued1, queued2);
        assert_ne!(queued1, in_flight);
    }
}

// ============================================================================
// StreamQueue Tests
// ============================================================================

#[cfg(test)]
mod stream_queue_tests {
    

    // Note: StreamQueue is private to worker module
    // We test it indirectly through AsyncJobManager
}

// ============================================================================
// AsyncJobManager Tests
// ============================================================================

#[cfg(test)]
mod async_job_manager_tests {
    use super::*;

    #[test]
    fn test_async_job_manager_new() {
        // Arrange & Act
        let manager = AsyncJobManager::new();

        // Assert
        assert_eq!(manager.current_request_id(StreamId::A), 0);
        assert_eq!(manager.current_request_id(StreamId::B), 0);
        assert_eq!(manager.in_flight_count(StreamId::A), 0);
        assert_eq!(manager.in_flight_count(StreamId::B), 0);
    }

    #[test]
    fn test_async_job_manager_default() {
        // Arrange & Act
        let manager = AsyncJobManager::default();

        // Assert
        assert_eq!(manager.current_request_id(StreamId::A), 0);
        assert_eq!(manager.current_request_id(StreamId::B), 0);
    }

    #[test]
    fn test_increment_request_id_stream_a() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act
        let id1 = manager.increment_request_id(StreamId::A);
        let id2 = manager.increment_request_id(StreamId::A);

        // Assert
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(manager.current_request_id(StreamId::A), 2);
    }

    #[test]
    fn test_increment_request_id_stream_b() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act
        let id1 = manager.increment_request_id(StreamId::B);
        let id2 = manager.increment_request_id(StreamId::B);

        // Assert
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(manager.current_request_id(StreamId::B), 2);
    }

    #[test]
    fn test_increment_request_id_independent_streams() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act
        let id_a = manager.increment_request_id(StreamId::A);
        let id_b = manager.increment_request_id(StreamId::B);
        let id_a2 = manager.increment_request_id(StreamId::A);

        // Assert - Streams have independent request IDs
        assert_eq!(id_a, 1);
        assert_eq!(id_b, 1);
        assert_eq!(id_a2, 2);
        assert_eq!(manager.current_request_id(StreamId::A), 2);
        assert_eq!(manager.current_request_id(StreamId::B), 1);
    }

    #[test]
    fn test_submit_job_starts_immediately() {
        // Arrange
        let manager = AsyncJobManager::new();
        let job = create_test_parse_job(StreamId::A, 1);

        // Act
        manager.submit(job);

        // Assert - Job should start immediately (in-flight < 2)
        assert_eq!(manager.in_flight_count(StreamId::A), 1);
    }

    #[test]
    fn test_submit_multiple_jobs() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act - Submit 2 jobs
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Assert - Both should be in-flight (max is 2)
        assert_eq!(manager.in_flight_count(StreamId::A), 2);
    }

    #[test]
    fn test_submit_third_job_queues() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act - Submit 3 jobs
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Assert - Third job should queue (max in-flight is 2)
        assert_eq!(manager.in_flight_count(StreamId::A), 2);
        assert!(manager.has_pending_work(StreamId::A));
    }

    #[test]
    fn test_complete_job_reduces_in_flight() {
        // Arrange
        let manager = AsyncJobManager::new();
        let job = create_test_parse_job(StreamId::A, 0); // request_id must match current (0)
        manager.submit(job.clone());

        // Act
        let result = manager.complete_job(&job);

        // Assert
        assert!(result); // Result was current
        assert_eq!(manager.in_flight_count(StreamId::A), 0);
    }

    #[test]
    fn test_complete_job_starts_queued() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Fill in-flight
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Queue third job
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Complete first job
        let job1 = create_test_parse_job(StreamId::A, 1);
        manager.complete_job(&job1);

        // Assert - Queued job should start
        assert_eq!(manager.in_flight_count(StreamId::A), 2);
    }

    #[test]
    fn test_complete_stale_job_discarded() {
        // Arrange
        let manager = AsyncJobManager::new();
        let job = create_test_parse_job(StreamId::A, 0); // request_id 0 (matches current)
        manager.submit(job.clone());

        // Increment request ID (makes job stale)
        manager.increment_request_id(StreamId::A); // Now request_id is 1

        // Act
        let result = manager.complete_job(&job);

        // Assert - Stale result should be discarded
        assert!(!result);
        assert_eq!(manager.in_flight_count(StreamId::A), 0); // Job removed as stale
    }

    #[test]
    fn test_scrub_cancels_decode_convert() {
        // Arrange
        let manager = AsyncJobManager::new();
        let decode_job = create_test_decode_job(StreamId::A, 10, 1);
        let overlay_job = Job::RenderOverlay {
            stream_id: StreamId::A,
            frame_index: 10,
            request_id: 1,
        };

        manager.submit(decode_job.clone());
        manager.submit(overlay_job.clone());

        // Act
        manager.scrub(StreamId::A);

        // Assert - In-flight count should be 0 after scrub
        // Note: The actual implementation cancels in-flight decode/convert jobs
        assert_eq!(manager.in_flight_count(StreamId::A), 0);
    }

    #[test]
    fn test_cancel_all_clears_queue() {
        // Arrange
        let manager = AsyncJobManager::new();
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Act
        manager.cancel_all(StreamId::A);

        // Assert
        assert_eq!(manager.in_flight_count(StreamId::A), 0);
        assert!(!manager.has_pending_work(StreamId::A));
    }

    #[test]
    fn test_increment_request_id_cancels_pending() {
        // Arrange
        let manager = AsyncJobManager::new();
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Act
        manager.increment_request_id(StreamId::A);

        // Assert - All jobs should be cancelled
        assert_eq!(manager.in_flight_count(StreamId::A), 0);
    }

    #[test]
    fn test_has_pending_work_true() {
        // Arrange
        let manager = AsyncJobManager::new();
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Act & Assert
        assert!(manager.has_pending_work(StreamId::A));
    }

    #[test]
    fn test_has_pending_work_false() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act & Assert
        assert!(!manager.has_pending_work(StreamId::A));
    }

    #[test]
    fn test_is_result_current_true() {
        // Arrange
        let manager = AsyncJobManager::new();
        let job = create_test_parse_job(StreamId::A, 0); // request_id matches initial

        // Act & Assert
        assert!(manager.is_result_current(&job));
    }

    #[test]
    fn test_is_result_current_false() {
        // Arrange
        let manager = AsyncJobManager::new();
        let job = create_test_parse_job(StreamId::A, 5); // request_id doesn't match

        // Act & Assert
        assert!(!manager.is_result_current(&job));
    }

    #[test]
    fn test_latest_wins_queue_behavior() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Fill in-flight
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Queue multiple jobs (latest wins)
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));

        // Assert - Only 2 in-flight, 1 queued (latest)
        assert_eq!(manager.in_flight_count(StreamId::A), 2);
        assert!(manager.has_pending_work(StreamId::A));
    }

    #[test]
    fn test_separate_stream_queues() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act - Submit jobs to both streams
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::B, 1));
        manager.submit(create_test_parse_job(StreamId::A, 1));
        manager.submit(create_test_parse_job(StreamId::B, 1));

        // Assert - Each stream has independent queue
        assert_eq!(manager.in_flight_count(StreamId::A), 2);
        assert_eq!(manager.in_flight_count(StreamId::B), 2);
    }
}

// ============================================================================
// JobManager Type Alias Tests
// ============================================================================

#[cfg(test)]
mod job_manager_alias_tests {
    use super::*;

    #[test]
    fn test_job_manager_is_async_job_manager() {
        // Arrange & Act
        let manager: JobManager = JobManager::new();

        // Assert - JobManager is just an alias
        assert_eq!(manager.current_request_id(StreamId::A), 0);
        assert_eq!(manager.current_request_id(StreamId::B), 0);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_frame_index() {
        // Arrange & Act
        let job = Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: 0,
            request_id: 1,
        };

        // Assert
        assert_eq!(job.stream_id(), StreamId::A);
        assert_eq!(job.request_id(), 1);
    }

    #[test]
    fn test_large_frame_index() {
        // Arrange & Act
        let job = Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: 999999,
            request_id: 1,
        };

        // Assert
        assert_eq!(job.stream_id(), StreamId::A);
    }

    #[test]
    fn test_large_request_id() {
        // Arrange & Act
        let job = Job::ParseContainer {
            stream_id: StreamId::A,
            path: create_test_path(),
            request_id: u64::MAX,
        };

        // Assert
        assert_eq!(job.request_id(), u64::MAX);
    }

    #[test]
    fn test_multiple_scrub_operations() {
        // Arrange
        let manager = AsyncJobManager::new();
        manager.submit(create_test_decode_job(StreamId::A, 10, 1));

        // Act - Multiple scrub operations
        manager.scrub(StreamId::A);
        manager.scrub(StreamId::A);
        manager.scrub(StreamId::A);

        // Assert - Should handle gracefully
        assert_eq!(manager.in_flight_count(StreamId::A), 0);
    }

    #[test]
    fn test_complete_job_on_empty_queue() {
        // Arrange
        let manager = AsyncJobManager::new();
        let job = create_test_parse_job(StreamId::A, 1);

        // Act - Complete job that was never submitted
        let result = manager.complete_job(&job);

        // Assert - Should return false (job not found)
        assert!(!result);
    }

    #[test]
    fn test_rapid_submit_complete_cycles() {
        // Arrange
        let manager = AsyncJobManager::new();

        // Act - Rapid submit/complete cycles
        for _i in 0..10 {
            let job = create_test_parse_job(StreamId::A, 0); // Always use request_id 0
            manager.submit(job.clone());
            manager.complete_job(&job);
        }

        // Assert - Should handle gracefully
        assert_eq!(manager.in_flight_count(StreamId::A), 0);
    }
}
