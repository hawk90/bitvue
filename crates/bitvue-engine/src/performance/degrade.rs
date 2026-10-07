//! Adaptive degradation policy: `DegradeLevel` and the `PerfBudget` checker that steps it
//! up/down from observed frame/overlay/tooltip times.

use super::budget;
use serde::{Deserialize, Serialize};

/// Degradation level for adaptive LOD
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum DegradeLevel {
    /// Full quality - all features enabled
    #[default]
    Full = 0,
    /// Medium quality - reduce overlay detail
    Medium = 1,
    /// Low quality - disable expensive overlays
    Low = 2,
    /// Minimal - only essential rendering
    Minimal = 3,
}

impl DegradeLevel {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::Minimal => "Minimal",
        }
    }

    /// Degrade one level (if possible)
    pub fn degrade(&self) -> Self {
        match self {
            Self::Full => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low => Self::Minimal,
            Self::Minimal => Self::Minimal,
        }
    }

    /// Upgrade one level (if possible)
    pub fn upgrade(&self) -> Self {
        match self {
            Self::Full => Self::Full,
            Self::Medium => Self::Full,
            Self::Low => Self::Medium,
            Self::Minimal => Self::Low,
        }
    }
}

/// Performance budget checker with adaptive degradation
#[derive(Debug, Clone, Default)]
pub struct PerfBudget {
    /// Current degradation level
    pub degrade_level: DegradeLevel,

    /// Last frame time (ms)
    pub last_frame_ms: f64,

    /// Last hit test time (ms)
    pub last_hit_test_ms: f64,

    /// Last overlay render time (ms)
    pub last_overlay_ms: f64,

    /// Last tooltip build time (ms)
    pub last_tooltip_ms: f64,

    /// Last selection propagation time (ms)
    pub last_selection_ms: f64,

    /// Consecutive frames over budget
    pub(super) over_budget_count: u32,

    /// Consecutive frames under budget
    pub(super) under_budget_count: u32,
}

impl PerfBudget {
    /// Create new performance budget tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record frame timing and check if degradation needed
    pub fn record_frame(&mut self, frame_ms: f64) -> bool {
        self.last_frame_ms = frame_ms;

        if frame_ms > budget::UI_FRAME_TARGET_MS {
            self.over_budget_count += 1;
            self.under_budget_count = 0;

            // Degrade after 3 consecutive over-budget frames
            if self.over_budget_count >= 3 && self.degrade_level != DegradeLevel::Minimal {
                self.degrade_level = self.degrade_level.degrade();
                self.over_budget_count = 0;
                return true; // Degraded
            }
        } else {
            self.under_budget_count += 1;
            self.over_budget_count = 0;

            // Upgrade after 10 consecutive under-budget frames
            if self.under_budget_count >= 10 && self.degrade_level != DegradeLevel::Full {
                self.degrade_level = self.degrade_level.upgrade();
                self.under_budget_count = 0;
                return true; // Upgraded
            }
        }

        false
    }

    /// Record hit test timing
    pub fn record_hit_test(&mut self, ms: f64) -> bool {
        self.last_hit_test_ms = ms;
        ms > budget::HIT_TEST_MAX_MS
    }

    /// Record overlay render timing
    pub fn record_overlay(&mut self, ms: f64) -> bool {
        self.last_overlay_ms = ms;
        ms > budget::OVERLAY_RENDER_MAX_MS
    }

    /// Record tooltip build timing
    pub fn record_tooltip(&mut self, ms: f64) -> bool {
        self.last_tooltip_ms = ms;
        ms > budget::TOOLTIP_BUILD_MAX_MS
    }

    /// Record selection propagation timing
    pub fn record_selection(&mut self, ms: f64) -> bool {
        self.last_selection_ms = ms;
        ms > budget::SELECTION_PROPAGATION_MAX_MS
    }

    /// Check if overlays should be simplified based on current degradation
    pub fn should_simplify_overlays(&self) -> bool {
        self.degrade_level >= DegradeLevel::Medium
    }

    /// Check if expensive overlays should be disabled
    pub fn should_disable_expensive_overlays(&self) -> bool {
        self.degrade_level >= DegradeLevel::Low
    }

    /// Check if tooltips should be deferred
    pub fn should_defer_tooltips(&self) -> bool {
        self.degrade_level >= DegradeLevel::Low
            || self.last_tooltip_ms > budget::TOOLTIP_BUILD_MAX_MS
    }

    /// Get recommended overlay LOD (0.0 = minimal, 1.0 = full)
    pub fn overlay_lod(&self) -> f32 {
        match self.degrade_level {
            DegradeLevel::Full => 1.0,
            DegradeLevel::Medium => 0.75,
            DegradeLevel::Low => 0.5,
            DegradeLevel::Minimal => 0.25,
        }
    }

    /// Reset degradation to full quality
    pub fn reset(&mut self) {
        self.degrade_level = DegradeLevel::Full;
        self.over_budget_count = 0;
        self.under_budget_count = 0;
    }
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
    fn test_degrade_level_ordering() {
        assert!(DegradeLevel::Full < DegradeLevel::Medium);
        assert!(DegradeLevel::Medium < DegradeLevel::Low);
        assert!(DegradeLevel::Low < DegradeLevel::Minimal);
    }

    #[test]
    fn test_degrade_level_degrade() {
        assert_eq!(DegradeLevel::Full.degrade(), DegradeLevel::Medium);
        assert_eq!(DegradeLevel::Medium.degrade(), DegradeLevel::Low);
        assert_eq!(DegradeLevel::Low.degrade(), DegradeLevel::Minimal);
        assert_eq!(DegradeLevel::Minimal.degrade(), DegradeLevel::Minimal);
    }

    #[test]
    fn test_degrade_level_upgrade() {
        assert_eq!(DegradeLevel::Minimal.upgrade(), DegradeLevel::Low);
        assert_eq!(DegradeLevel::Low.upgrade(), DegradeLevel::Medium);
        assert_eq!(DegradeLevel::Medium.upgrade(), DegradeLevel::Full);
        assert_eq!(DegradeLevel::Full.upgrade(), DegradeLevel::Full);
    }

    #[test]
    fn test_perf_budget_new() {
        let budget = PerfBudget::new();
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        assert!((budget.last_frame_ms - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_perf_budget_degrade_after_consecutive_over_budget() {
        let mut budget = PerfBudget::new();

        // First 2 over-budget frames shouldn't degrade
        budget.record_frame(20.0);
        assert_eq!(budget.degrade_level, DegradeLevel::Full);
        budget.record_frame(20.0);
        assert_eq!(budget.degrade_level, DegradeLevel::Full);

        // Third over-budget frame should trigger degradation
        let degraded = budget.record_frame(20.0);
        assert!(degraded);
        assert_eq!(budget.degrade_level, DegradeLevel::Medium);
    }

    #[test]
    fn test_perf_budget_upgrade_after_consecutive_under_budget() {
        let mut budget = PerfBudget::new();
        budget.degrade_level = DegradeLevel::Low;

        // Need 10 consecutive under-budget frames to upgrade
        for i in 0..9 {
            let upgraded = budget.record_frame(10.0);
            assert!(!upgraded, "Should not upgrade on frame {}", i);
        }

        // 10th under-budget frame should upgrade
        let upgraded = budget.record_frame(10.0);
        assert!(upgraded);
        assert_eq!(budget.degrade_level, DegradeLevel::Medium);
    }

    #[test]
    fn test_perf_budget_over_budget_detection() {
        let mut budget = PerfBudget::new();

        assert!(!budget.record_hit_test(1.0)); // Under budget
        assert!(budget.record_hit_test(2.0)); // Over budget

        assert!(!budget.record_overlay(5.0)); // Under budget
        assert!(budget.record_overlay(7.0)); // Over budget

        assert!(!budget.record_tooltip(0.5)); // Under budget
        assert!(budget.record_tooltip(1.0)); // Over budget

        assert!(!budget.record_selection(1.5)); // Under budget
        assert!(budget.record_selection(3.0)); // Over budget
    }

    #[test]
    fn test_perf_budget_lod_values() {
        let mut budget = PerfBudget::new();

        budget.degrade_level = DegradeLevel::Full;
        assert!((budget.overlay_lod() - 1.0).abs() < f32::EPSILON);

        budget.degrade_level = DegradeLevel::Medium;
        assert!((budget.overlay_lod() - 0.75).abs() < f32::EPSILON);

        budget.degrade_level = DegradeLevel::Low;
        assert!((budget.overlay_lod() - 0.5).abs() < f32::EPSILON);

        budget.degrade_level = DegradeLevel::Minimal;
        assert!((budget.overlay_lod() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_perf_budget_simplify_overlays() {
        let mut budget = PerfBudget::new();

        budget.degrade_level = DegradeLevel::Full;
        assert!(!budget.should_simplify_overlays());

        budget.degrade_level = DegradeLevel::Medium;
        assert!(budget.should_simplify_overlays());

        budget.degrade_level = DegradeLevel::Low;
        assert!(budget.should_simplify_overlays());
    }

    #[test]
    fn test_perf_budget_disable_expensive_overlays() {
        let mut budget = PerfBudget::new();

        budget.degrade_level = DegradeLevel::Full;
        assert!(!budget.should_disable_expensive_overlays());

        budget.degrade_level = DegradeLevel::Medium;
        assert!(!budget.should_disable_expensive_overlays());

        budget.degrade_level = DegradeLevel::Low;
        assert!(budget.should_disable_expensive_overlays());
    }

    #[test]
    fn test_perf_budget_reset() {
        let mut budget = PerfBudget::new();
        budget.degrade_level = DegradeLevel::Minimal;
        budget.record_frame(20.0);
        budget.record_frame(20.0);

        budget.reset();

        assert_eq!(budget.degrade_level, DegradeLevel::Full);
    }
}
