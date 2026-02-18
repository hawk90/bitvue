#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for worker module

use bitvue_core::worker::{AsyncJobManager, Job};
use bitvue_core::StreamId;

#[test]
fn test_async_job_manager_creation() {
    let manager = AsyncJobManager::new();
    assert_eq!(manager.current_request_id(StreamId::A), 0);
    assert_eq!(manager.current_request_id(StreamId::B), 0);
    assert_eq!(manager.in_flight_count(StreamId::A), 0);
    assert_eq!(manager.in_flight_count(StreamId::B), 0);
}

#[test]
fn test_increment_request_id() {
    let manager = AsyncJobManager::new();

    // Initial state
    assert_eq!(manager.current_request_id(StreamId::A), 0);

    // Increment StreamA
    let id1 = manager.increment_request_id(StreamId::A);
    assert_eq!(id1, 1);
    assert_eq!(manager.current_request_id(StreamId::A), 1);

    // StreamB should be independent
    assert_eq!(manager.current_request_id(StreamId::B), 0);

    // Increment again
    let id2 = manager.increment_request_id(StreamId::A);
    assert_eq!(id2, 2);
    assert_eq!(manager.current_request_id(StreamId::A), 2);
}

#[test]
fn test_latest_wins_queue() {
    // Per ASYNC_PIPELINE_BACKPRESSURE.md:
    // "Latest-wins: only most recent request kept"
    let manager = AsyncJobManager::new();
    let request_id = manager.current_request_id(StreamId::A);

    // Submit job1
    let job1 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id,
    };
    manager.submit(job1.clone());

    // Submit job2 (should start immediately, in-flight count = 1)
    let job2 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 1,
        request_id,
    };
    manager.submit(job2.clone());

    assert_eq!(manager.in_flight_count(StreamId::A), 2);

    // Submit job3 (should queue, replacing any previous queued job)
    let job3 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 2,
        request_id,
    };
    manager.submit(job3.clone());

    // Submit job4 (should replace job3 in queue - latest-wins)
    let job4 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 3,
        request_id,
    };
    manager.submit(job4.clone());

    // Still 2 in-flight (job1, job2)
    assert_eq!(manager.in_flight_count(StreamId::A), 2);

    // Complete job1 -> job4 should start (job3 was replaced)
    assert!(manager.complete_job(&job1));
    assert_eq!(manager.in_flight_count(StreamId::A), 2); // job2, job4
}

#[test]
fn test_max_in_flight_limit() {
    // Per ASYNC_PIPELINE_BACKPRESSURE.md:
    // "Max in-flight tasks: 2 per stream"
    let manager = AsyncJobManager::new();
    let request_id = manager.current_request_id(StreamId::A);

    // Submit 3 jobs
    for i in 0..3 {
        let job = Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: i,
            request_id,
        };
        manager.submit(job);
    }

    // Only 2 should be in-flight
    assert_eq!(manager.in_flight_count(StreamId::A), 2);
    assert!(manager.has_pending_work(StreamId::A));
}

#[test]
fn test_per_stream_independence() {
    let manager = AsyncJobManager::new();
    let request_id_a = manager.current_request_id(StreamId::A);
    let request_id_b = manager.current_request_id(StreamId::B);

    // Submit jobs to both streams
    for i in 0..3 {
        manager.submit(Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: i,
            request_id: request_id_a,
        });
        manager.submit(Job::DecodeFrame {
            stream_id: StreamId::B,
            frame_index: i,
            request_id: request_id_b,
        });
    }

    // Each stream should have 2 in-flight
    assert_eq!(manager.in_flight_count(StreamId::A), 2);
    assert_eq!(manager.in_flight_count(StreamId::B), 2);
}

#[test]
fn test_late_result_discarded() {
    // Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §E:
    // "Late results discarded if request_id mismatches."
    let manager = AsyncJobManager::new();

    let job = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id: 0, // Current request_id
    };
    manager.submit(job.clone());

    // Increment request_id (simulates file reload)
    manager.increment_request_id(StreamId::A);
    // Now current_request_id is 1, but job has request_id 0

    // Try to complete old job -> should be discarded
    let accepted = manager.complete_job(&job);
    assert!(!accepted); // Late result rejected
}

#[test]
fn test_is_result_current() {
    let manager = AsyncJobManager::new();

    let job_v1 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id: 0,
    };

    // Current initially
    assert!(manager.is_result_current(&job_v1));

    // Increment request_id
    manager.increment_request_id(StreamId::A);

    // Now stale
    assert!(!manager.is_result_current(&job_v1));

    // New job is current
    let job_v2 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id: 1,
    };
    assert!(manager.is_result_current(&job_v2));
}

#[test]
fn test_scrub_cancels_decode_convert() {
    // Per ASYNC_PIPELINE_BACKPRESSURE.md:
    // "Cancel all non-current decode/convert jobs"
    let manager = AsyncJobManager::new();
    let request_id = manager.current_request_id(StreamId::A);

    // Submit decode and non-decode jobs
    manager.submit(Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id,
    });
    manager.submit(Job::BuildTimelineIndex {
        stream_id: StreamId::A,
        request_id,
    });

    assert_eq!(manager.in_flight_count(StreamId::A), 2);

    // Scrub should cancel decode/convert but keep others
    manager.scrub(StreamId::A);

    // BuildTimelineIndex should remain, DecodeFrame should be cancelled
    assert_eq!(manager.in_flight_count(StreamId::A), 1);
}

#[test]
fn test_cancel_all() {
    let manager = AsyncJobManager::new();
    let request_id = manager.current_request_id(StreamId::A);

    // Submit jobs
    for i in 0..3 {
        manager.submit(Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: i,
            request_id,
        });
    }

    assert!(manager.has_pending_work(StreamId::A));

    // Cancel all
    manager.cancel_all(StreamId::A);

    assert_eq!(manager.in_flight_count(StreamId::A), 0);
    assert!(!manager.has_pending_work(StreamId::A));
}

#[test]
fn test_job_completion_starts_queued() {
    let manager = AsyncJobManager::new();
    let request_id = manager.current_request_id(StreamId::A);

    let job1 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id,
    };
    let job2 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 1,
        request_id,
    };
    let job3 = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 2,
        request_id,
    };

    manager.submit(job1.clone());
    manager.submit(job2.clone());
    manager.submit(job3.clone());

    // job1, job2 in-flight; job3 queued
    assert_eq!(manager.in_flight_count(StreamId::A), 2);

    // Complete job1 -> job3 should start
    manager.complete_job(&job1);

    // job2, job3 now in-flight
    assert_eq!(manager.in_flight_count(StreamId::A), 2);
}

#[test]
fn test_job_is_decode_convert() {
    let decode = Job::DecodeFrame {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id: 0,
    };
    assert!(decode.is_decode_convert());

    let overlay = Job::RenderOverlay {
        stream_id: StreamId::A,
        frame_index: 0,
        request_id: 0,
    };
    assert!(overlay.is_decode_convert());

    let parse = Job::ParseUnits {
        stream_id: StreamId::A,
        request_id: 0,
    };
    assert!(!parse.is_decode_convert());
}

#[test]
fn test_increment_request_id_cancels_all() {
    // Per ASYNC_PIPELINE_BACKPRESSURE.md:
    // "Increment request_id cancels all pending jobs"
    let manager = AsyncJobManager::new();
    let request_id = manager.current_request_id(StreamId::A);

    // Submit jobs
    for i in 0..3 {
        manager.submit(Job::DecodeFrame {
            stream_id: StreamId::A,
            frame_index: i,
            request_id,
        });
    }

    assert!(manager.has_pending_work(StreamId::A));

    // Increment request_id (file reload)
    manager.increment_request_id(StreamId::A);

    // All jobs should be cancelled
    assert_eq!(manager.in_flight_count(StreamId::A), 0);
    assert!(!manager.has_pending_work(StreamId::A));
}
