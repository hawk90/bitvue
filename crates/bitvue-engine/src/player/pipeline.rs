//! Player pipeline - T2-1
//!
//! Per FAST_PATH_QUALITY_PATH_POLICY.md:
//! - Fast path: Quarter/Half resolution, luma-only preview, <= 60ms target
//! - Quality path: 200ms idle → upgrade (high-quality decode, RGBA, overlays)

use super::{ColorSpace, DecodeParams, ResolutionTier};
use std::time::{Duration, Instant};

/// Player pipeline state
///
/// Per FAST_PATH_QUALITY_PATH_POLICY.md:
/// "Trigger: no user input for 200ms"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    /// Fast path active (preview mode)
    FastPath,

    /// Idle, waiting to upgrade
    IdleWaiting { elapsed_ms: u64 },

    /// Quality upgrade in progress
    QualityUpgrade,

    /// Quality path complete
    QualityComplete,
}

/// Player frame pipeline
///
/// Per T2-1 deliverable: PlayerPipeline
///
/// Implements decode→convert→texture pipeline with:
/// - Fast-path preview (Quarter/Half res, YUV)
/// - Idle quality upgrade (Full res, RGBA)
/// - Texture reuse
pub struct PlayerPipeline {
    /// Current frame index
    current_frame: Option<usize>,

    /// Pipeline state
    state: PipelineState,

    /// Last user input timestamp
    last_input_time: Instant,

    /// Idle threshold (default: 200ms)
    idle_threshold: Duration,

    /// Fast-path resolution tier (default: Half)
    fast_path_tier: ResolutionTier,

    /// Enable quality upgrade (can be disabled during scrub)
    quality_upgrade_enabled: bool,
}

impl PlayerPipeline {
    /// Create a new player pipeline
    pub fn new() -> Self {
        Self {
            current_frame: None,
            state: PipelineState::FastPath,
            last_input_time: Instant::now(),
            idle_threshold: Duration::from_millis(200),
            fast_path_tier: ResolutionTier::Half,
            quality_upgrade_enabled: true,
        }
    }

    /// Set fast-path resolution tier
    pub fn set_fast_path_tier(&mut self, tier: ResolutionTier) {
        self.fast_path_tier = tier;
    }

    /// Set idle threshold
    pub fn set_idle_threshold(&mut self, duration: Duration) {
        self.idle_threshold = duration;
    }

    /// Enable or disable quality upgrade
    ///
    /// Per PERFORMANCE_DEGRADATION_RULES.md:
    /// "During scrub: Quality path disabled"
    pub fn set_quality_upgrade_enabled(&mut self, enabled: bool) {
        self.quality_upgrade_enabled = enabled;
        if !enabled
            && matches!(
                self.state,
                PipelineState::QualityUpgrade | PipelineState::IdleWaiting { .. }
            )
        {
            self.state = PipelineState::FastPath;
        }
    }

    /// Notify user input (resets idle timer)
    ///
    /// Per FAST_PATH_QUALITY_PATH_POLICY.md:
    /// "Abort upgrade immediately on user input."
    pub fn on_user_input(&mut self) {
        self.last_input_time = Instant::now();

        // Abort quality upgrade if in progress
        if matches!(
            self.state,
            PipelineState::QualityUpgrade | PipelineState::IdleWaiting { .. }
        ) {
            self.state = PipelineState::FastPath;
        }
    }

    /// Update pipeline state (call each frame)
    ///
    /// Returns true if quality upgrade should be triggered.
    pub fn update(&mut self) -> bool {
        if !self.quality_upgrade_enabled {
            self.state = PipelineState::FastPath;
            return false;
        }

        let elapsed = self.last_input_time.elapsed();

        match self.state {
            PipelineState::FastPath => {
                if elapsed >= self.idle_threshold {
                    // Transition directly to QualityUpgrade
                    self.state = PipelineState::QualityUpgrade;
                    true
                } else {
                    false
                }
            }
            PipelineState::IdleWaiting { .. } => {
                // This state is transient, immediately upgrade
                self.state = PipelineState::QualityUpgrade;
                true
            }
            PipelineState::QualityUpgrade => {
                // Upgrade in progress, wait for completion
                false
            }
            PipelineState::QualityComplete => {
                // Already at quality
                false
            }
        }
    }

    /// Mark quality upgrade as complete
    pub fn mark_quality_complete(&mut self) {
        if matches!(self.state, PipelineState::QualityUpgrade) {
            self.state = PipelineState::QualityComplete;
        }
    }

    /// Set current frame (triggers fast-path decode)
    pub fn set_current_frame(&mut self, frame_idx: usize) {
        if self.current_frame != Some(frame_idx) {
            self.current_frame = Some(frame_idx);
            self.state = PipelineState::FastPath;
        }
    }

    /// Get current frame
    pub fn current_frame(&self) -> Option<usize> {
        self.current_frame
    }

    /// Get pipeline state
    pub fn state(&self) -> PipelineState {
        self.state
    }

    /// Check if quality upgrade is due
    pub fn should_upgrade(&self) -> bool {
        self.quality_upgrade_enabled
            && !matches!(
                self.state,
                PipelineState::QualityUpgrade | PipelineState::QualityComplete
            )
            && self.last_input_time.elapsed() >= self.idle_threshold
    }

    /// Get decode params for current state
    pub fn current_decode_params(&self) -> Option<DecodeParams> {
        self.current_frame.map(|frame_idx| match self.state {
            PipelineState::FastPath | PipelineState::IdleWaiting { .. } => {
                DecodeParams::new(frame_idx, self.fast_path_tier, ColorSpace::Yuv)
            }
            PipelineState::QualityUpgrade | PipelineState::QualityComplete => {
                DecodeParams::quality_path(frame_idx)
            }
        })
    }
}

impl Default for PlayerPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_player_pipeline_creation() {
        let pipeline = PlayerPipeline::new();
        assert!(pipeline.current_frame.is_none());
        assert_eq!(pipeline.state, PipelineState::FastPath);
        assert!(pipeline.quality_upgrade_enabled);
    }

    #[test]
    fn test_player_pipeline_set_frame() {
        let mut pipeline = PlayerPipeline::new();
        pipeline.set_current_frame(5);

        assert_eq!(pipeline.current_frame(), Some(5));
        assert_eq!(pipeline.state(), PipelineState::FastPath);
    }

    #[test]
    fn test_player_pipeline_idle_upgrade() {
        let mut pipeline = PlayerPipeline::new();
        pipeline.set_idle_threshold(Duration::from_millis(50));
        pipeline.set_current_frame(0);

        // Initially fast path
        assert_eq!(pipeline.state(), PipelineState::FastPath);

        // Wait for idle threshold
        thread::sleep(Duration::from_millis(60));

        // Update should trigger upgrade
        let should_upgrade = pipeline.update();
        assert!(should_upgrade);
        assert_eq!(pipeline.state(), PipelineState::QualityUpgrade);

        // Mark complete
        pipeline.mark_quality_complete();
        assert_eq!(pipeline.state(), PipelineState::QualityComplete);
    }

    #[test]
    fn test_player_pipeline_user_input_aborts_upgrade() {
        let mut pipeline = PlayerPipeline::new();
        pipeline.set_idle_threshold(Duration::from_millis(50));
        pipeline.set_current_frame(0);

        // Wait for idle
        thread::sleep(Duration::from_millis(60));
        pipeline.update();
        assert_eq!(pipeline.state(), PipelineState::QualityUpgrade);

        // User input aborts upgrade
        pipeline.on_user_input();
        assert_eq!(pipeline.state(), PipelineState::FastPath);
    }

    #[test]
    fn test_player_pipeline_quality_disabled_during_scrub() {
        let mut pipeline = PlayerPipeline::new();
        pipeline.set_idle_threshold(Duration::from_millis(50));
        pipeline.set_current_frame(0);

        // Wait for idle
        thread::sleep(Duration::from_millis(60));
        pipeline.update();

        // Disable quality upgrade (scrub mode)
        pipeline.set_quality_upgrade_enabled(false);
        assert_eq!(pipeline.state(), PipelineState::FastPath);

        // Update should not trigger upgrade
        let should_upgrade = pipeline.update();
        assert!(!should_upgrade);
    }

    #[test]
    fn test_player_pipeline_current_decode_params() {
        let mut pipeline = PlayerPipeline::new();
        pipeline.set_current_frame(10);

        // Fast path params
        let params = pipeline.current_decode_params().unwrap();
        assert_eq!(params.frame_idx, 10);
        assert_eq!(params.res_tier, ResolutionTier::Half);
        assert_eq!(params.color_space, ColorSpace::Yuv);

        // Trigger quality upgrade
        pipeline.state = PipelineState::QualityUpgrade;
        let params = pipeline.current_decode_params().unwrap();
        assert_eq!(params.frame_idx, 10);
        assert_eq!(params.res_tier, ResolutionTier::Full);
        assert_eq!(params.color_space, ColorSpace::Rgba);
    }

    #[test]
    fn test_player_pipeline_should_upgrade() {
        let mut pipeline = PlayerPipeline::new();
        pipeline.set_idle_threshold(Duration::from_millis(50));
        pipeline.set_current_frame(0);

        // Initially should not upgrade
        assert!(!pipeline.should_upgrade());

        // Wait for idle
        thread::sleep(Duration::from_millis(60));

        // Now should upgrade (check before update())
        assert!(pipeline.should_upgrade());

        // Update triggers upgrade
        let should_upgrade = pipeline.update();
        assert!(should_upgrade);
        assert_eq!(pipeline.state(), PipelineState::QualityUpgrade);
    }

    #[test]
    fn test_player_pipeline_fast_path_tier_customization() {
        let mut pipeline = PlayerPipeline::new();
        pipeline.set_fast_path_tier(ResolutionTier::Quarter);
        pipeline.set_current_frame(0);

        let params = pipeline.current_decode_params().unwrap();
        assert_eq!(params.res_tier, ResolutionTier::Quarter);
    }
}
