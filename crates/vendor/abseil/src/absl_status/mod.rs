//! Status utilities for error handling.
//!
//! This module provides status utilities similar to Abseil's `absl/status` directory.
//!
//! # Overview
//!
//! Status utilities provide a structured way to handle errors and return values
//! that may fail. This is similar to Go's error handling or C++'s StatusOr.
//!
//! # Components
//!
//! - [`Status`] - Status type for error codes and messages
//! - [`StatusCode`] - Enumeration of standard error codes
//! - [`StatusOr<T>`] - Type that contains either a Status (error) or a value T
//! - [`StatusBuilder`] - Builder for constructing Status with additional context
//! - [`ErrorChain`] - Chain of errors with context
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_status::{Status, StatusCode, StatusOr};
//!
//! fn read_config() -> StatusOr<String> {
//!     // Try to read config...
//!     Err(Status::new(StatusCode::NotFound, "Config file not found"))
//! }
//!
//! fn process_config() -> Status {
//!     match read_config() {
//!         Ok(config) => {
//!             // Process config...
//!             Status::ok()
//!         }
//!         Err(status) => status,
//!     }
//! }
//! ```


extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// Submodules
pub mod aggregator;
pub mod annotation;
pub mod builder;
pub mod categories;
pub mod context;
pub mod error_chain;
pub mod fallback;
pub mod helpers;
pub mod metrics;
pub mod retry;
pub mod status;
pub statusor;
pub mod transform;

// Core re-exports
pub use status::{Status, StatusCode};
pub use statusor::StatusOr;

// Builder re-exports
pub use builder::StatusBuilder;

// Error chain re-exports
pub use error_chain::{ErrorChain, ErrorChainIter, IsError, ToStatus};

// Context re-exports
pub use context::WithContext;

// Helpers re-exports
pub use helpers::StatusHelpers;

// Categories re-exports
pub use categories::ErrorCategory;

// Retry re-exports
pub use retry::{BackoffStrategy, RetryPolicy, RetryResult, retry_sync};

// Annotation re-exports
pub use annotation::{AnnotatedStatus, StatusAnnotation};

// Aggregator re-exports
pub use aggregator::{AggregationStrategy, StatusAggregator};

// Transform re-exports
pub use transform::StatusTransformer;

// Fallback re-exports
pub use fallback::{CachedFallback, fallback, try_fallbacks};

// Metrics re-exports
pub use metrics::StatusMetrics;

// Macros
pub use context::{status_builder, status_err, statusor_err};
