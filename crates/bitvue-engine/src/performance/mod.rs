//! Performance Instrumentation - T9-1
//!
//! Per PERF_PROFILING_INSTRUMENTATION.md:
//! - Required timers: open_file, io_read, mmap, parse, index, decode, convert, overlay, upload, paint
//! - Cache hit rate tracking
//! - Developer HUD toggle
//! - Exportable performance reports
//! - JSON logging format per event
//!
//! Per perf_budget_and_instrumentation.json :
//! - Performance budgets with automatic degradation
//! - LOD virtualization when over budget

#[cfg(test)]
use std::time::Duration;

mod degrade;
mod metrics;

pub use degrade::*;
pub use metrics::*;

/// Performance budget constants (ms) -
pub mod budget {
    /// Target frame time for 60fps (16.6ms)
    pub const UI_FRAME_TARGET_MS: f64 = 16.6;

    /// Maximum time for hit testing (1.5ms)
    pub const HIT_TEST_MAX_MS: f64 = 1.5;

    /// Maximum time for overlay rendering (6.0ms)
    pub const OVERLAY_RENDER_MAX_MS: f64 = 6.0;

    /// Maximum time for tooltip building (0.8ms)
    pub const TOOLTIP_BUILD_MAX_MS: f64 = 0.8;

    /// Maximum time for selection propagation (2.0ms)
    pub const SELECTION_PROPAGATION_MAX_MS: f64 = 2.0;
}

#[allow(
    unused_imports,
    unused_variables,
    unused_mut,
    dead_code,
    unused_comparisons,
    unused_must_use,
    hidden_glob_reexports,
    unreachable_code,
    non_camel_case_types,
    unused_parens,
    unused_assignments
)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_constants() {
        // Verify budget constants match perf_budget_and_instrumentation.json
        assert!((budget::UI_FRAME_TARGET_MS - 16.6).abs() < f64::EPSILON);
        assert!((budget::HIT_TEST_MAX_MS - 1.5).abs() < f64::EPSILON);
        assert!((budget::OVERLAY_RENDER_MAX_MS - 6.0).abs() < f64::EPSILON);
        assert!((budget::TOOLTIP_BUILD_MAX_MS - 0.8).abs() < f64::EPSILON);
        assert!((budget::SELECTION_PROPAGATION_MAX_MS - 2.0).abs() < f64::EPSILON);
    }
}

// Additional comprehensive tests
include!("../performance_test.rs");
