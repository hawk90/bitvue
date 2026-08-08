//! Player HUD - T2-2
//!
//! Displays frame metadata, quality badges, and overlay state

use super::{PipelineState, ResolutionTier};
use crate::types::FrameType;
use serde::{Deserialize, Serialize};

/// PTS quality badge
///
/// Per T0-1 FRAME_IDENTITY_CONTRACT:
/// Display PTS quality and warnings in player HUD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtsQualityBadge {
    /// PTS quality is OK
    Ok,
    /// Warning (VFR or < 50% missing)
    Warn,
    /// Bad (> 50% missing or duplicates)
    Bad,
}

impl PtsQualityBadge {
    /// Get badge text
    pub fn text(&self) -> &'static str {
        match self {
            PtsQualityBadge::Ok => "PTS: OK",
            PtsQualityBadge::Warn => "PTS: WARN",
            PtsQualityBadge::Bad => "PTS: BAD",
        }
    }

    /// Get badge color hint (for UI rendering)
    pub fn color_hint(&self) -> &'static str {
        match self {
            PtsQualityBadge::Ok => "green",
            PtsQualityBadge::Warn => "yellow",
            PtsQualityBadge::Bad => "red",
        }
    }

    /// Get tooltip text
    pub fn tooltip(&self) -> &'static str {
        match self {
            PtsQualityBadge::Ok => "Presentation timestamps are valid",
            PtsQualityBadge::Warn => "Variable frame rate or some PTS missing",
            PtsQualityBadge::Bad => "Many PTS missing or duplicates detected",
        }
    }
}

/// Alignment confidence badge
///
/// Per T2-2 deliverable: alignment/confidence badges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignmentBadge {
    /// Perfect alignment
    Perfect,
    /// Good alignment (within tolerance)
    Good,
    /// Poor alignment
    Poor,
}

impl AlignmentBadge {
    pub fn text(&self) -> &'static str {
        match self {
            AlignmentBadge::Perfect => "Align: Perfect",
            AlignmentBadge::Good => "Align: Good",
            AlignmentBadge::Poor => "Align: Poor",
        }
    }

    pub fn color_hint(&self) -> &'static str {
        match self {
            AlignmentBadge::Perfect => "green",
            AlignmentBadge::Good => "yellow",
            AlignmentBadge::Poor => "red",
        }
    }
}

/// Key toggle state
///
/// Per T2-2: Display key toggles state in HUD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeyToggles {
    /// Grid overlay enabled
    pub grid: bool,
    /// QP heatmap enabled
    pub qp_heatmap: bool,
    /// MV overlay enabled
    pub mv_overlay: bool,
    /// Partition overlay enabled
    pub partition: bool,
    /// Diff overlay enabled
    pub diff: bool,
}

impl KeyToggles {
    /// Get active overlay names
    pub fn active_overlays(&self) -> Vec<&'static str> {
        let mut active = Vec::new();
        if self.grid {
            active.push("Grid");
        }
        if self.qp_heatmap {
            active.push("QP");
        }
        if self.mv_overlay {
            active.push("MV");
        }
        if self.partition {
            active.push("Part");
        }
        if self.diff {
            active.push("Diff");
        }
        active
    }

    /// Check if any overlay is active
    pub fn has_active(&self) -> bool {
        self.grid || self.qp_heatmap || self.mv_overlay || self.partition || self.diff
    }
}

/// Frame type display extension for HUD
///
/// Provides display-friendly formatting for FrameType from types::FrameType.
pub trait FrameTypeDisplay {
    /// Get short name (single character for display)
    fn short_name(&self) -> &'static str;

    /// Get full name
    fn full_name(&self) -> &'static str;
}

impl FrameTypeDisplay for FrameType {
    fn short_name(&self) -> &'static str {
        match self {
            FrameType::Key => "I",
            FrameType::Inter => "P",
            FrameType::BFrame => "B",
            FrameType::IntraOnly => "I",
            FrameType::Switch => "S",
            FrameType::SI => "SI",
            FrameType::SP => "SP",
            FrameType::Unknown => "?",
        }
    }

    fn full_name(&self) -> &'static str {
        match self {
            FrameType::Key => "Key",
            FrameType::Inter => "Inter",
            FrameType::BFrame => "B",
            FrameType::IntraOnly => "Intra",
            FrameType::Switch => "Switch",
            FrameType::SI => "SI",
            FrameType::SP => "SP",
            FrameType::Unknown => "Unknown",
        }
    }
}

/// Decode status for HUD display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecodeStatus {
    /// Not yet decoded
    Pending,
    /// Currently decoding
    Decoding,
    /// Successfully decoded
    Complete,
    /// Decode failed
    Failed,
}

impl DecodeStatus {
    pub fn text(&self) -> &'static str {
        match self {
            DecodeStatus::Pending => "Pending",
            DecodeStatus::Decoding => "Decoding...",
            DecodeStatus::Complete => "OK",
            DecodeStatus::Failed => "FAILED",
        }
    }

    pub fn color_hint(&self) -> &'static str {
        match self {
            DecodeStatus::Pending => "gray",
            DecodeStatus::Decoding => "blue",
            DecodeStatus::Complete => "green",
            DecodeStatus::Failed => "red",
        }
    }
}

/// Player HUD data
///
/// Per T2-2 deliverable: PlayerHUD
/// Per WS_PLAYER_SPATIAL: frame idx, pts/dts, type, size, qp_avg, bpp, decode status
///
/// Displays frame metadata, quality badges, and overlay state
#[derive(Debug, Clone)]
pub struct PlayerHud {
    /// Current frame display index
    pub display_idx: usize,

    /// Presentation timestamp (if available)
    pub pts: Option<u64>,

    /// Decode timestamp (if available)
    pub dts: Option<u64>,

    /// Frame type (I/P/B)
    pub frame_type: FrameType,

    /// Frame size in bytes
    pub frame_size: Option<usize>,

    /// Average QP for frame
    pub qp_avg: Option<f32>,

    /// Bits per pixel
    pub bpp: Option<f32>,

    /// Decode status
    pub decode_status: DecodeStatus,

    /// Frame width
    pub width: u32,

    /// Frame height
    pub height: u32,

    /// Bit depth (8, 10, 12)
    pub bit_depth: u8,

    /// Resolution tier (actual decoded resolution)
    pub res_tier: ResolutionTier,

    /// PTS quality badge
    pub pts_quality: PtsQualityBadge,

    /// Alignment badge (for compare mode)
    pub alignment: Option<AlignmentBadge>,

    /// Key toggles state
    pub toggles: KeyToggles,

    /// Pipeline state (fast/quality)
    pub pipeline_state: PipelineState,
}

impl PlayerHud {
    /// Create a new HUD with default values
    pub fn new(display_idx: usize, width: u32, height: u32) -> Self {
        Self {
            display_idx,
            pts: None,
            dts: None,
            frame_type: FrameType::Unknown,
            frame_size: None,
            qp_avg: None,
            bpp: None,
            decode_status: DecodeStatus::Pending,
            width,
            height,
            bit_depth: 8,
            res_tier: ResolutionTier::Full,
            pts_quality: PtsQualityBadge::Ok,
            alignment: None,
            toggles: KeyToggles::default(),
            pipeline_state: PipelineState::FastPath,
        }
    }

    /// Format frame info line
    ///
    /// Example: `"Frame 42 [I] | 1920x1080 | 10-bit"`
    pub fn frame_info_line(&self) -> String {
        format!(
            "Frame {} [{}] | {}x{} | {}-bit",
            self.display_idx,
            self.frame_type.short_name(),
            self.width,
            self.height,
            self.bit_depth
        )
    }

    /// Format frame stats line (size, QP, BPP)
    ///
    /// Example: "Size: 45KB | QP: 28.5 | BPP: 0.25"
    pub fn frame_stats_line(&self) -> String {
        let size_str = self
            .frame_size
            .map(|s| {
                if s >= 1024 * 1024 {
                    format!("{:.2}MB", s as f64 / (1024.0 * 1024.0))
                } else if s >= 1024 {
                    format!("{:.1}KB", s as f64 / 1024.0)
                } else {
                    format!("{}B", s)
                }
            })
            .unwrap_or_else(|| "N/A".to_string());

        let qp_str = self
            .qp_avg
            .map(|qp| format!("{:.1}", qp))
            .unwrap_or_else(|| "N/A".to_string());

        let bpp_str = self
            .bpp
            .map(|bpp| format!("{:.3}", bpp))
            .unwrap_or_else(|| "N/A".to_string());

        format!("Size: {} | QP: {} | BPP: {}", size_str, qp_str, bpp_str)
    }

    /// Format decode status line
    ///
    /// Example: "Decode: OK"
    pub fn decode_status_line(&self) -> String {
        format!("Decode: {}", self.decode_status.text())
    }

    /// Format PTS/DTS line
    ///
    /// Example: "PTS: 1234 | DTS: 1234"
    pub fn pts_dts_line(&self) -> String {
        let pts_str = self
            .pts
            .map(|p| format!("{}", p))
            .unwrap_or_else(|| "N/A".to_string());
        let dts_str = self
            .dts
            .map(|d| format!("{}", d))
            .unwrap_or_else(|| "N/A".to_string());
        format!("PTS: {} | DTS: {}", pts_str, dts_str)
    }

    /// Format resolution tier line
    ///
    /// Example: "Preview: Half (960x540)"
    pub fn resolution_line(&self) -> String {
        let (scaled_w, scaled_h) = self.res_tier.scale_dims(self.width, self.height);
        let tier_name = match self.res_tier {
            ResolutionTier::Quarter => "Quarter",
            ResolutionTier::Half => "Half",
            ResolutionTier::Full => "Full",
        };
        format!("Preview: {} ({}x{})", tier_name, scaled_w, scaled_h)
    }

    /// Format pipeline state line
    ///
    /// Example: "State: Fast Path" or "State: Quality"
    pub fn pipeline_state_line(&self) -> String {
        let state_str = match self.pipeline_state {
            PipelineState::FastPath => "Fast Path",
            PipelineState::IdleWaiting { .. } => "Idle...",
            PipelineState::QualityUpgrade => "Upgrading...",
            PipelineState::QualityComplete => "Quality",
        };
        format!("State: {}", state_str)
    }

    /// Format active overlays line
    ///
    /// Example: "Overlays: Grid, QP, MV"
    pub fn overlays_line(&self) -> Option<String> {
        if self.toggles.has_active() {
            let active = self.toggles.active_overlays().join(", ");
            Some(format!("Overlays: {}", active))
        } else {
            None
        }
    }

    /// Get all badges (for rendering)
    pub fn badges(&self) -> Vec<(&'static str, &'static str)> {
        let mut badges = Vec::new();

        // PTS quality badge
        badges.push((self.pts_quality.text(), self.pts_quality.color_hint()));

        // Alignment badge (if in compare mode)
        if let Some(alignment) = &self.alignment {
            badges.push((alignment.text(), alignment.color_hint()));
        }

        badges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pts_quality_badge() {
        let ok = PtsQualityBadge::Ok;
        assert_eq!(ok.text(), "PTS: OK");
        assert_eq!(ok.color_hint(), "green");
        assert!(ok.tooltip().contains("valid"));

        let warn = PtsQualityBadge::Warn;
        assert_eq!(warn.text(), "PTS: WARN");
        assert_eq!(warn.color_hint(), "yellow");

        let bad = PtsQualityBadge::Bad;
        assert_eq!(bad.text(), "PTS: BAD");
        assert_eq!(bad.color_hint(), "red");
    }

    #[test]
    fn test_alignment_badge() {
        let perfect = AlignmentBadge::Perfect;
        assert_eq!(perfect.text(), "Align: Perfect");
        assert_eq!(perfect.color_hint(), "green");

        let good = AlignmentBadge::Good;
        assert_eq!(good.text(), "Align: Good");
        assert_eq!(good.color_hint(), "yellow");

        let poor = AlignmentBadge::Poor;
        assert_eq!(poor.text(), "Align: Poor");
        assert_eq!(poor.color_hint(), "red");
    }

    #[test]
    fn test_key_toggles() {
        let mut toggles = KeyToggles::default();
        assert!(!toggles.has_active());
        assert_eq!(toggles.active_overlays().len(), 0);

        toggles.grid = true;
        toggles.qp_heatmap = true;
        assert!(toggles.has_active());

        let active = toggles.active_overlays();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"Grid"));
        assert!(active.contains(&"QP"));
    }

    #[test]
    fn test_key_toggles_all_active() {
        let mut toggles = KeyToggles::default();
        toggles.grid = true;
        toggles.qp_heatmap = true;
        toggles.mv_overlay = true;
        toggles.partition = true;
        toggles.diff = true;

        let active = toggles.active_overlays();
        assert_eq!(active.len(), 5);
        assert!(active.contains(&"Grid"));
        assert!(active.contains(&"QP"));
        assert!(active.contains(&"MV"));
        assert!(active.contains(&"Part"));
        assert!(active.contains(&"Diff"));
    }

    #[test]
    fn test_player_hud_creation() {
        let hud = PlayerHud::new(42, 1920, 1080);
        assert_eq!(hud.display_idx, 42);
        assert_eq!(hud.width, 1920);
        assert_eq!(hud.height, 1080);
        assert_eq!(hud.bit_depth, 8);
        assert_eq!(hud.res_tier, ResolutionTier::Full);
        assert_eq!(hud.pts_quality, PtsQualityBadge::Ok);
        assert!(hud.alignment.is_none());
    }

    #[test]
    fn test_player_hud_frame_info_line() {
        let mut hud = PlayerHud::new(42, 1920, 1080);
        let line = hud.frame_info_line();
        assert_eq!(line, "Frame 42 [?] | 1920x1080 | 8-bit");

        hud.bit_depth = 10;
        hud.frame_type = FrameType::Key;
        let line = hud.frame_info_line();
        assert_eq!(line, "Frame 42 [I] | 1920x1080 | 10-bit");

        hud.frame_type = FrameType::Inter;
        let line = hud.frame_info_line();
        assert_eq!(line, "Frame 42 [P] | 1920x1080 | 10-bit");
    }

    #[test]
    fn test_player_hud_pts_dts_line() {
        let mut hud = PlayerHud::new(0, 1920, 1080);

        // No PTS/DTS
        let line = hud.pts_dts_line();
        assert_eq!(line, "PTS: N/A | DTS: N/A");

        // With PTS/DTS
        hud.pts = Some(1000);
        hud.dts = Some(1000);
        let line = hud.pts_dts_line();
        assert_eq!(line, "PTS: 1000 | DTS: 1000");
    }

    #[test]
    fn test_player_hud_resolution_line() {
        let mut hud = PlayerHud::new(0, 1920, 1080);

        hud.res_tier = ResolutionTier::Full;
        assert_eq!(hud.resolution_line(), "Preview: Full (1920x1080)");

        hud.res_tier = ResolutionTier::Half;
        assert_eq!(hud.resolution_line(), "Preview: Half (960x540)");

        hud.res_tier = ResolutionTier::Quarter;
        assert_eq!(hud.resolution_line(), "Preview: Quarter (480x270)");
    }

    #[test]
    fn test_player_hud_pipeline_state_line() {
        let mut hud = PlayerHud::new(0, 1920, 1080);

        hud.pipeline_state = PipelineState::FastPath;
        assert_eq!(hud.pipeline_state_line(), "State: Fast Path");

        hud.pipeline_state = PipelineState::QualityUpgrade;
        assert_eq!(hud.pipeline_state_line(), "State: Upgrading...");

        hud.pipeline_state = PipelineState::QualityComplete;
        assert_eq!(hud.pipeline_state_line(), "State: Quality");
    }

    #[test]
    fn test_player_hud_overlays_line() {
        let mut hud = PlayerHud::new(0, 1920, 1080);

        // No overlays
        assert!(hud.overlays_line().is_none());

        // With overlays
        hud.toggles.grid = true;
        hud.toggles.qp_heatmap = true;
        let line = hud.overlays_line().unwrap();
        assert!(line.contains("Grid"));
        assert!(line.contains("QP"));
    }

    #[test]
    fn test_player_hud_badges() {
        let mut hud = PlayerHud::new(0, 1920, 1080);

        // Only PTS badge
        let badges = hud.badges();
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].0, "PTS: OK");
        assert_eq!(badges[0].1, "green");

        // With alignment badge
        hud.alignment = Some(AlignmentBadge::Perfect);
        let badges = hud.badges();
        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].0, "PTS: OK");
        assert_eq!(badges[1].0, "Align: Perfect");

        // Warning badges
        hud.pts_quality = PtsQualityBadge::Warn;
        hud.alignment = Some(AlignmentBadge::Poor);
        let badges = hud.badges();
        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].0, "PTS: WARN");
        assert_eq!(badges[0].1, "yellow");
        assert_eq!(badges[1].0, "Align: Poor");
        assert_eq!(badges[1].1, "red");
    }

    #[test]
    fn test_frame_type() {
        let key = FrameType::Key;
        assert_eq!(key.short_name(), "I");
        assert_eq!(key.full_name(), "Key");

        let inter = FrameType::Inter;
        assert_eq!(inter.short_name(), "P");
        assert_eq!(inter.full_name(), "Inter");

        let bframe = FrameType::BFrame;
        assert_eq!(bframe.short_name(), "B");
        assert_eq!(bframe.full_name(), "B");

        let unknown = FrameType::Unknown;
        assert_eq!(unknown.short_name(), "?");
        assert_eq!(unknown.full_name(), "Unknown");
    }

    #[test]
    fn test_decode_status() {
        let pending = DecodeStatus::Pending;
        assert_eq!(pending.text(), "Pending");
        assert_eq!(pending.color_hint(), "gray");

        let decoding = DecodeStatus::Decoding;
        assert_eq!(decoding.text(), "Decoding...");
        assert_eq!(decoding.color_hint(), "blue");

        let complete = DecodeStatus::Complete;
        assert_eq!(complete.text(), "OK");
        assert_eq!(complete.color_hint(), "green");

        let failed = DecodeStatus::Failed;
        assert_eq!(failed.text(), "FAILED");
        assert_eq!(failed.color_hint(), "red");
    }

    #[test]
    fn test_player_hud_frame_stats_line() {
        let mut hud = PlayerHud::new(0, 1920, 1080);

        // All N/A
        let line = hud.frame_stats_line();
        assert_eq!(line, "Size: N/A | QP: N/A | BPP: N/A");

        // With frame size in bytes
        hud.frame_size = Some(512);
        let line = hud.frame_stats_line();
        assert_eq!(line, "Size: 512B | QP: N/A | BPP: N/A");

        // With frame size in KB
        hud.frame_size = Some(45 * 1024);
        hud.qp_avg = Some(28.5);
        let line = hud.frame_stats_line();
        assert_eq!(line, "Size: 45.0KB | QP: 28.5 | BPP: N/A");

        // With all fields
        hud.frame_size = Some(2 * 1024 * 1024 + 512 * 1024); // 2.5MB
        hud.qp_avg = Some(28.5);
        hud.bpp = Some(0.254);
        let line = hud.frame_stats_line();
        assert_eq!(line, "Size: 2.50MB | QP: 28.5 | BPP: 0.254");
    }

    #[test]
    fn test_player_hud_decode_status_line() {
        let mut hud = PlayerHud::new(0, 1920, 1080);

        hud.decode_status = DecodeStatus::Pending;
        assert_eq!(hud.decode_status_line(), "Decode: Pending");

        hud.decode_status = DecodeStatus::Decoding;
        assert_eq!(hud.decode_status_line(), "Decode: Decoding...");

        hud.decode_status = DecodeStatus::Complete;
        assert_eq!(hud.decode_status_line(), "Decode: OK");

        hud.decode_status = DecodeStatus::Failed;
        assert_eq!(hud.decode_status_line(), "Decode: FAILED");
    }

    #[test]
    fn test_player_hud_comprehensive() {
        let mut hud = PlayerHud::new(42, 1920, 1080);

        // Set all fields
        hud.pts = Some(1000);
        hud.dts = Some(1000);
        hud.frame_type = FrameType::Key;
        hud.frame_size = Some(100 * 1024); // 100KB
        hud.qp_avg = Some(25.0);
        hud.bpp = Some(0.4);
        hud.decode_status = DecodeStatus::Complete;
        hud.bit_depth = 10;
        hud.res_tier = ResolutionTier::Full;
        hud.pts_quality = PtsQualityBadge::Ok;

        // Test all lines
        assert_eq!(hud.frame_info_line(), "Frame 42 [I] | 1920x1080 | 10-bit");
        assert_eq!(hud.pts_dts_line(), "PTS: 1000 | DTS: 1000");
        assert_eq!(
            hud.frame_stats_line(),
            "Size: 100.0KB | QP: 25.0 | BPP: 0.400"
        );
        assert_eq!(hud.decode_status_line(), "Decode: OK");
        assert_eq!(hud.resolution_line(), "Preview: Full (1920x1080)");
        assert_eq!(hud.pipeline_state_line(), "State: Fast Path");

        let badges = hud.badges();
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].0, "PTS: OK");
    }
}
