//! Frame Identity Resolver - T0-1
//!
//! Per FRAME_IDENTITY_CONTRACT.md:
//! - Primary timeline index = display_idx (PTS order)
//! - decode_idx is internal only (DTS order)
//! - PTS quality detection (OK/WARN/BAD)
//!
//! Edge cases per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md:
//! - VFR, missing or duplicated PTS → fallback to display_idx with badge
//! - PTS quality badge: OK/WARN/BAD

use serde::{Deserialize, Serialize};

/// Frame index mapping between display order (PTS) and decode order (DTS)
///
/// **Primary timeline index = display_idx (PTS order)**
/// decode_idx is internal only and used for decoder reordering.
///
/// Invariants (FRAME_IDENTITY_CONTRACT.md):
/// - display_idx is stable across sessions for the same stream
/// - A frame selection always maps to exactly one display_idx
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameIndexMap {
    /// Total frame count
    pub frame_count: usize,

    /// Map: display_idx → decode_idx (INTERNAL ONLY per FRAME_IDENTITY_CONTRACT)
    display_to_decode: Vec<usize>,

    /// Map: decode_idx → display_idx (INTERNAL ONLY per FRAME_IDENTITY_CONTRACT)
    decode_to_display: Vec<usize>,

    /// Map: display_idx → PTS (if available)
    display_to_pts: Vec<Option<u64>>,

    /// Map: display_idx → DTS (if available)
    display_to_dts: Vec<Option<u64>>,

    /// PTS quality assessment
    pub pts_quality: PtsQuality,
}

impl FrameIndexMap {
    /// Create a new FrameIndexMap from frame metadata
    ///
    /// Frames are provided in decode order. This function:
    /// 1. Determines PTS quality
    /// 2. Sorts frames by PTS to establish display_idx
    /// 3. Creates bidirectional mapping between display_idx and decode_idx
    pub fn new(frames: &[FrameMetadata]) -> Self {
        let frame_count = frames.len();

        if frame_count == 0 {
            return Self::empty();
        }

        // Assess PTS quality
        let pts_quality = Self::assess_pts_quality(frames);

        // Build display order (PTS sorted)
        let mut display_order: Vec<(usize, Option<u64>)> = frames
            .iter()
            .enumerate()
            .map(|(decode_idx, frame)| (decode_idx, frame.pts))
            .collect();

        // Sort by PTS, fallback to decode order for missing/duplicate PTS
        display_order.sort_by(
            |(decode_a, pts_a), (decode_b, pts_b)| match (pts_a, pts_b) {
                (Some(a), Some(b)) => a.cmp(b).then_with(|| decode_a.cmp(decode_b)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => decode_a.cmp(decode_b),
            },
        );

        // Build mappings
        let mut display_to_decode = Vec::with_capacity(frame_count);
        let mut decode_to_display = vec![0; frame_count];
        let mut display_to_pts = Vec::with_capacity(frame_count);
        let mut display_to_dts = Vec::with_capacity(frame_count);

        for (display_idx, (decode_idx, _)) in display_order.iter().enumerate() {
            display_to_decode.push(*decode_idx);
            decode_to_display[*decode_idx] = display_idx;
            display_to_pts.push(frames[*decode_idx].pts);
            display_to_dts.push(frames[*decode_idx].dts);
        }

        Self {
            frame_count,
            display_to_decode,
            decode_to_display,
            display_to_pts,
            display_to_dts,
            pts_quality,
        }
    }

    /// Create an empty FrameIndexMap
    fn empty() -> Self {
        Self {
            frame_count: 0,
            display_to_decode: Vec::new(),
            decode_to_display: Vec::new(),
            display_to_pts: Vec::new(),
            display_to_dts: Vec::new(),
            pts_quality: PtsQuality::Ok,
        }
    }

    /// Assess PTS quality from frame metadata
    ///
    /// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
    /// - OK: All frames have valid, monotonic PTS
    /// - WARN: Some missing PTS, but majority present
    /// - BAD: Major issues (duplicates, VFR, >50% missing)
    fn assess_pts_quality(frames: &[FrameMetadata]) -> PtsQuality {
        if frames.is_empty() {
            return PtsQuality::Ok;
        }

        let total = frames.len();
        let missing_count = frames.iter().filter(|f| f.pts.is_none()).count();

        // Check if >50% missing
        if missing_count > total / 2 {
            return PtsQuality::Bad;
        }

        // Check for duplicates and collect PTS values in sorted order
        let mut pts_values: Vec<u64> = frames.iter().filter_map(|f| f.pts).collect();
        pts_values.sort_unstable();

        // Check for duplicate PTS values
        let has_duplicates = pts_values.windows(2).any(|w| w[0] == w[1]);

        if has_duplicates {
            return PtsQuality::Bad;
        }

        // Note: We do NOT check for monotonicity in decode order.
        // PTS values are allowed to be out of order in decode sequence (e.g., B-frames).
        // The sorting step above establishes display order.
        // Duplicates are the only PTS ordering issue we flag as BAD.

        // Check for VFR (variable frame rate) by analyzing PTS deltas
        // Use sorted PTS values to compute deltas in display order
        if pts_values.len() >= 3 {
            let mut deltas: Vec<u64> = Vec::new();

            for window in pts_values.windows(2) {
                deltas.push(window[1] - window[0]);
            }

            if !deltas.is_empty() {
                let mean_delta = deltas.iter().sum::<u64>() as f64 / deltas.len() as f64;
                let variance = deltas
                    .iter()
                    .map(|&d| {
                        let diff = d as f64 - mean_delta;
                        diff * diff
                    })
                    .sum::<f64>()
                    / deltas.len() as f64;
                let std_dev = variance.sqrt();

                // If coefficient of variation > 0.3, consider VFR
                if mean_delta > 0.0 && (std_dev / mean_delta) > 0.3 {
                    return PtsQuality::Warn;
                }
            }
        }

        // If some missing but <50%, WARN
        if missing_count > 0 {
            return PtsQuality::Warn;
        }

        PtsQuality::Ok
    }

    /// Convert display_idx to decode_idx
    #[inline]
    pub fn display_to_decode_idx(&self, display_idx: usize) -> Option<usize> {
        self.display_to_decode.get(display_idx).copied()
    }

    /// Convert decode_idx to display_idx
    ///
    /// **WARNING**: This function is for TESTING ONLY.
    /// Per FRAME_IDENTITY_CONTRACT: decode_idx is internal-only.
    /// Production code must use display_idx as primary index.
    #[cfg(test)]
    #[inline]
    pub fn decode_to_display_idx(&self, decode_idx: usize) -> Option<usize> {
        self.decode_to_display.get(decode_idx).copied()
    }

    /// Get PTS for display_idx
    #[inline]
    pub fn get_pts(&self, display_idx: usize) -> Option<u64> {
        self.display_to_pts.get(display_idx).and_then(|&pts| pts)
    }

    /// Get DTS for display_idx
    #[inline]
    pub fn get_dts(&self, display_idx: usize) -> Option<u64> {
        self.display_to_dts.get(display_idx).and_then(|&dts| dts)
    }

    /// Get frame count
    #[inline]
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Get PTS quality
    #[inline]
    pub fn pts_quality(&self) -> PtsQuality {
        self.pts_quality
    }

    /// Check if reordering is present (display order != decode order)
    pub fn has_reordering(&self) -> bool {
        self.display_to_decode
            .iter()
            .enumerate()
            .any(|(display_idx, &decode_idx)| display_idx != decode_idx)
    }
}

/// Frame metadata for building FrameIndexMap
///
/// Minimal metadata required to establish display/decode ordering.
#[derive(Debug, Clone, Copy)]
pub struct FrameMetadata {
    /// Presentation timestamp (if available)
    pub pts: Option<u64>,
    /// Decode timestamp (if available)
    pub dts: Option<u64>,
}

/// PTS quality assessment
///
/// Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md §A:
/// - OK: All frames have valid, monotonic PTS
/// - WARN: Some missing PTS or VFR detected, but usable
/// - BAD: Major issues (duplicates, >50% missing, non-monotonic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtsQuality {
    /// All PTS values present and monotonic
    Ok,
    /// Some issues detected (missing PTS, VFR), but usable
    Warn,
    /// Major issues (duplicates, >50% missing, non-monotonic)
    Bad,
}

impl PtsQuality {
    /// Get display text for the badge
    pub fn badge_text(&self) -> &'static str {
        match self {
            PtsQuality::Ok => "PTS: OK",
            PtsQuality::Warn => "PTS: WARN",
            PtsQuality::Bad => "PTS: BAD",
        }
    }

    /// Get color hint for UI display
    pub fn color_hint(&self) -> PtsQualityColor {
        match self {
            PtsQuality::Ok => PtsQualityColor::Green,
            PtsQuality::Warn => PtsQualityColor::Yellow,
            PtsQuality::Bad => PtsQualityColor::Red,
        }
    }

    /// Get tooltip explanation
    pub fn tooltip(&self) -> &'static str {
        match self {
            PtsQuality::Ok => "All frames have valid, monotonic PTS values",
            PtsQuality::Warn => {
                "Some PTS values missing or variable frame rate detected. Timeline uses frame index fallback."
            }
            PtsQuality::Bad => {
                "Major PTS issues detected (duplicates, non-monotonic, or >50% missing). Timeline uses frame index."
            }
        }
    }
}

/// Color hint for PTS quality badge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtsQualityColor {
    Green,
    Yellow,
    Red,
}

// ============================================================================
// T0-1: Frame Identity Extractor API (viz_core.002)
// ============================================================================

/// Frame identity extractor trait for codec-specific frame metadata extraction
///
/// Deliverable: extract_api:FrameIdentity:Core:AV1:viz_core
///
/// Each codec implements this trait to extract PTS/DTS from its bitstream.
/// The extractor provides codec-specific logic while maintaining the
/// FRAME_IDENTITY_CONTRACT (display_idx vs decode_idx).
pub trait FrameIdentityExtractor {
    /// Extract frame metadata from raw bitstream data
    ///
    /// Returns frame metadata in decode order.
    /// The FrameIndexMap will sort these into display order.
    fn extract_frames(&self, data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError>;

    /// Get codec name for this extractor
    fn codec_name(&self) -> &'static str;

    /// Check if this codec supports PTS/DTS extraction
    fn supports_timestamps(&self) -> bool {
        true
    }
}

/// Error type for frame identity extraction
#[derive(Debug, Clone)]
pub enum ExtractionError {
    /// Invalid bitstream format
    InvalidFormat(String),
    /// Codec parsing failed
    ParseError(String),
    /// Unsupported codec feature
    Unsupported(String),
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ExtractionError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ExtractionError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            ExtractionError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ExtractionError {}

// ============================================================================
// AV1 Frame Identity Extractor
// ============================================================================

/// AV1 frame identity extractor
///
/// Extracts frame metadata (PTS/DTS) from AV1 bitstream.
/// Supports:
/// - IVF container (with PTS)
/// - Raw OBU stream (no container timestamps)
/// - ISOBMFF/Matroska (future)
pub struct Av1FrameIdentityExtractor {
    /// Whether to expect container timestamps
    pub has_container: bool,
}

impl Av1FrameIdentityExtractor {
    /// Create new AV1 extractor
    pub fn new() -> Self {
        Self {
            has_container: false,
        }
    }

    /// Create AV1 extractor expecting container timestamps
    pub fn with_container() -> Self {
        Self {
            has_container: true,
        }
    }
}

impl Default for Av1FrameIdentityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameIdentityExtractor for Av1FrameIdentityExtractor {
    fn extract_frames(&self, _data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        // TODO: Implement actual AV1 frame extraction
        // For now, return stub implementation
        Ok(vec![])
    }

    fn codec_name(&self) -> &'static str {
        "AV1"
    }
}

// ============================================================================
// S.T0-1.AV1.FrameIdentity.Core.impl.quirks_AV1.001
// AV1 tile/sb mapping quirks for frame identity
// ============================================================================

/// AV1-specific quirks for frame identity
///
/// AV1 has several codec-specific features that affect frame identity:
/// 1. Tile-to-superblock mapping: Tiles may affect frame reordering visualization
/// 2. Film grain synthesis: Film grain flags can create "virtual" frames
/// 3. show_existing_frame: References previously decoded frames without new data
///
/// These quirks are documented here but implementation is deferred to AV1 parser integration.
#[derive(Debug, Clone)]
pub struct Av1FrameIdentityQuirks {
    /// Whether this frame uses show_existing_frame (references existing frame)
    pub is_show_existing: bool,

    /// Frame index referenced by show_existing_frame (if applicable)
    pub show_existing_frame_idx: Option<usize>,

    /// Whether film grain synthesis is enabled
    pub has_film_grain: bool,

    /// Number of tiles in this frame (affects spatial parsing)
    pub tile_count: Option<usize>,
}

impl Av1FrameIdentityQuirks {
    /// Create default quirks (no special handling)
    pub fn default_quirks() -> Self {
        Self {
            is_show_existing: false,
            show_existing_frame_idx: None,
            has_film_grain: false,
            tile_count: None,
        }
    }

    /// Create quirks for show_existing_frame
    pub fn show_existing(frame_idx: usize) -> Self {
        Self {
            is_show_existing: true,
            show_existing_frame_idx: Some(frame_idx),
            has_film_grain: false,
            tile_count: None,
        }
    }

    /// Check if this frame needs special identity handling
    pub fn needs_special_handling(&self) -> bool {
        self.is_show_existing || self.has_film_grain
    }

    /// Create quirks for frame with film grain
    pub fn with_film_grain() -> Self {
        Self {
            is_show_existing: false,
            show_existing_frame_idx: None,
            has_film_grain: true,
            tile_count: None,
        }
    }

    /// Create quirks for frame with tile count
    pub fn with_tiles(count: usize) -> Self {
        Self {
            is_show_existing: false,
            show_existing_frame_idx: None,
            has_film_grain: false,
            tile_count: Some(count),
        }
    }

    /// Set film grain flag
    pub fn set_film_grain(&mut self, enabled: bool) {
        self.has_film_grain = enabled;
    }

    /// Set tile count
    pub fn set_tile_count(&mut self, count: usize) {
        self.tile_count = Some(count);
    }
}

// ============================================================================
// Timeline-specific AV1 Quirks Extensions (quirks_AV1.001)
// ============================================================================

/// Timeline metadata with AV1 quirks
///
/// Deliverable: av1_tiles:FrameIdentity:Timeline:AV1:quirks_AV1
///
/// Extends timeline frame metadata with AV1-specific information
/// that affects visualization but not frame identity.
#[derive(Debug, Clone)]
pub struct TimelineFrameWithQuirks {
    /// Display index (primary identity)
    pub display_idx: usize,
    /// Frame size in bytes
    pub size_bytes: u64,
    /// Frame type string
    pub frame_type: String,
    /// AV1-specific quirks
    pub quirks: Av1FrameIdentityQuirks,
}

impl TimelineFrameWithQuirks {
    /// Create new timeline frame with quirks
    pub fn new(
        display_idx: usize,
        size_bytes: u64,
        frame_type: String,
        quirks: Av1FrameIdentityQuirks,
    ) -> Self {
        Self {
            display_idx,
            size_bytes,
            frame_type,
            quirks,
        }
    }

    /// Check if this frame is a "virtual" frame (show_existing_frame)
    ///
    /// show_existing_frame creates a display frame without new coded data.
    /// Per FRAME_IDENTITY_CONTRACT: Still gets unique display_idx.
    pub fn is_virtual_frame(&self) -> bool {
        self.quirks.is_show_existing
    }

    /// Get referenced frame index for virtual frames
    pub fn referenced_frame(&self) -> Option<usize> {
        self.quirks.show_existing_frame_idx
    }

    /// Check if frame has film grain synthesis
    ///
    /// Film grain is applied during display, not decode.
    /// Does not affect frame identity or timeline position.
    pub fn has_film_grain(&self) -> bool {
        self.quirks.has_film_grain
    }

    /// Get tile count (for spatial parsing hints)
    pub fn tile_count(&self) -> Option<usize> {
        self.quirks.tile_count
    }

    /// Get visualization hint for this frame
    ///
    /// Suggests how timeline should display this frame:
    /// - Virtual frames: use lighter/dashed rendering
    /// - Film grain frames: add grain indicator
    /// - Multi-tile frames: add tile count badge
    pub fn viz_hint(&self) -> FrameVizHint {
        if self.is_virtual_frame() {
            FrameVizHint::Virtual
        } else if self.has_film_grain() {
            FrameVizHint::FilmGrain
        } else if let Some(count) = self.tile_count() {
            if count > 1 {
                FrameVizHint::MultiTile(count)
            } else {
                FrameVizHint::Normal
            }
        } else {
            FrameVizHint::Normal
        }
    }
}

/// Visualization hint for timeline rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameVizHint {
    /// Normal frame
    Normal,
    /// Virtual frame (show_existing_frame)
    Virtual,
    /// Frame with film grain synthesis
    FilmGrain,
    /// Frame with multiple tiles
    MultiTile(usize),
}

/// AV1 quirks timeline helper
///
/// Manages AV1 quirks for timeline display without affecting frame identity.
#[derive(Debug, Clone)]
pub struct Av1QuirksTimeline {
    /// Frame metadata with quirks (indexed by display_idx)
    frames: Vec<TimelineFrameWithQuirks>,
    /// Frame index map (identity layer)
    index_map: FrameIndexMap,
}

impl Av1QuirksTimeline {
    /// Create new AV1 quirks timeline
    pub fn new(
        frames_meta: Vec<FrameMetadata>,
        frames_quirks: Vec<(u64, String, Av1FrameIdentityQuirks)>,
    ) -> Self {
        let index_map = FrameIndexMap::new(&frames_meta);

        // Build timeline frames with quirks in display order
        let mut frames = Vec::new();
        for display_idx in 0..index_map.frame_count() {
            if let Some(decode_idx) = index_map.display_to_decode_idx(display_idx) {
                let (size, frame_type, quirks) =
                    frames_quirks.get(decode_idx).cloned().unwrap_or_else(|| {
                        (
                            0,
                            "UNKNOWN".to_string(),
                            Av1FrameIdentityQuirks::default_quirks(),
                        )
                    });

                frames.push(TimelineFrameWithQuirks::new(
                    display_idx,
                    size,
                    frame_type,
                    quirks,
                ));
            }
        }

        Self { frames, index_map }
    }

    /// Get frame with quirks by display_idx
    pub fn get_frame(&self, display_idx: usize) -> Option<&TimelineFrameWithQuirks> {
        self.frames.get(display_idx)
    }

    /// Get all virtual frames (show_existing_frame)
    pub fn virtual_frames(&self) -> Vec<usize> {
        self.frames
            .iter()
            .filter_map(|f| {
                if f.is_virtual_frame() {
                    Some(f.display_idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all frames with film grain
    pub fn film_grain_frames(&self) -> Vec<usize> {
        self.frames
            .iter()
            .filter_map(|f| {
                if f.has_film_grain() {
                    Some(f.display_idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all multi-tile frames
    pub fn multi_tile_frames(&self) -> Vec<(usize, usize)> {
        self.frames
            .iter()
            .filter_map(|f| {
                if let Some(count) = f.tile_count() {
                    if count > 1 {
                        return Some((f.display_idx, count));
                    }
                }
                None
            })
            .collect()
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get index map
    pub fn index_map(&self) -> &FrameIndexMap {
        &self.index_map
    }
}

// ============================================================================
// Codec Extractor Stubs (unsupported codecs)
// ============================================================================

/// H.264 frame identity extractor (stub)
pub struct H264FrameIdentityExtractor;

impl FrameIdentityExtractor for H264FrameIdentityExtractor {
    fn extract_frames(&self, _data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        Err(ExtractionError::Unsupported(
            "H.264 frame extraction not yet implemented".to_string(),
        ))
    }

    fn codec_name(&self) -> &'static str {
        "H.264"
    }

    fn supports_timestamps(&self) -> bool {
        false
    }
}

/// HEVC frame identity extractor (stub)
pub struct HevcFrameIdentityExtractor;

impl FrameIdentityExtractor for HevcFrameIdentityExtractor {
    fn extract_frames(&self, _data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        Err(ExtractionError::Unsupported(
            "HEVC frame extraction not yet implemented".to_string(),
        ))
    }

    fn codec_name(&self) -> &'static str {
        "HEVC"
    }

    fn supports_timestamps(&self) -> bool {
        false
    }
}

/// VP9 frame identity extractor (stub)
pub struct Vp9FrameIdentityExtractor;

impl FrameIdentityExtractor for Vp9FrameIdentityExtractor {
    fn extract_frames(&self, _data: &[u8]) -> Result<Vec<FrameMetadata>, ExtractionError> {
        Err(ExtractionError::Unsupported(
            "VP9 frame extraction not yet implemented".to_string(),
        ))
    }

    fn codec_name(&self) -> &'static str {
        "VP9"
    }

    fn supports_timestamps(&self) -> bool {
        false
    }
}

// ============================================================================
// T0-1: Frame Mapping Join (viz_core.003)
// ============================================================================

/// Build FrameIndexMap from extractor and raw data
///
/// Deliverable: frame_map:FrameIdentity:Core:AV1:viz_core
///
/// This function binds extracted units to display_idx/decode_idx and PTS/DTS mapping.
/// Per FRAME_IDENTITY_CONTRACT:
/// - Primary timeline index = display_idx (PTS order)
/// - decode_idx is internal only
pub fn build_frame_map_from_extractor(
    extractor: &dyn FrameIdentityExtractor,
    data: &[u8],
) -> Result<FrameIndexMap, ExtractionError> {
    // Extract frame metadata in decode order
    let frames = extractor.extract_frames(data)?;

    // Build FrameIndexMap (sorts into display order)
    Ok(FrameIndexMap::new(&frames))
}

/// Frame mapper - combines extractor and index map
///
/// High-level API for frame identity resolution.
/// Handles codec detection, extraction, and mapping in one step.
pub struct FrameMapper {
    extractor: Box<dyn FrameIdentityExtractor>,
    index_map: Option<FrameIndexMap>,
}

impl FrameMapper {
    /// Create new mapper with specific extractor
    pub fn new(extractor: Box<dyn FrameIdentityExtractor>) -> Self {
        Self {
            extractor,
            index_map: None,
        }
    }

    /// Create AV1 mapper
    pub fn for_av1() -> Self {
        Self::new(Box::new(Av1FrameIdentityExtractor::new()))
    }

    /// Create H.264 mapper
    pub fn for_h264() -> Self {
        Self::new(Box::new(H264FrameIdentityExtractor))
    }

    /// Create HEVC mapper
    pub fn for_hevc() -> Self {
        Self::new(Box::new(HevcFrameIdentityExtractor))
    }

    /// Create VP9 mapper
    pub fn for_vp9() -> Self {
        Self::new(Box::new(Vp9FrameIdentityExtractor))
    }

    /// Build frame index map from data
    pub fn build(&mut self, data: &[u8]) -> Result<&FrameIndexMap, ExtractionError> {
        let map = build_frame_map_from_extractor(self.extractor.as_ref(), data)?;
        self.index_map = Some(map);
        Ok(self.index_map.as_ref().unwrap())
    }

    /// Get the built index map
    pub fn index_map(&self) -> Option<&FrameIndexMap> {
        self.index_map.as_ref()
    }

    /// Get codec name
    pub fn codec_name(&self) -> &'static str {
        self.extractor.codec_name()
    }
}

// ============================================================================
// Timeline Extractor API (viz_core.002)
// ============================================================================

use crate::timeline::{FrameMarker, TimelineBase, TimelineFrame};

/// Timeline extractor trait for converting FrameIndexMap to Timeline visualization
///
/// Deliverable: extract_api:FrameIdentity:Timeline:AV1:viz_core
///
/// This trait bridges frame identity (FrameIndexMap) with timeline visualization (TimelineBase).
/// Each codec can customize timeline presentation (e.g., frame type strings, markers).
///
/// Per FRAME_IDENTITY_CONTRACT:
/// - Timeline uses display_idx as the canonical horizontal axis
/// - decode_idx is internal only and must not be exposed
pub trait TimelineExtractor {
    /// Extract timeline from frame index map
    ///
    /// Converts FrameIndexMap (identity layer) to TimelineBase (viz layer).
    /// Per FRAME_IDENTITY_CONTRACT: Timeline uses display_idx as primary index.
    ///
    /// # Arguments
    ///
    /// * `stream_id` - Unique stream identifier
    /// * `index_map` - Frame identity map (display/decode ordering)
    /// * `frame_sizes` - Frame sizes in bytes (indexed by display_idx)
    /// * `frame_types` - Frame type strings (indexed by display_idx)
    ///
    /// # Returns
    ///
    /// TimelineBase with frames in display order
    fn extract_timeline(
        &self,
        stream_id: String,
        index_map: &FrameIndexMap,
        frame_sizes: &[u64],
        frame_types: &[String],
    ) -> TimelineBase;

    /// Get codec name
    fn codec_name(&self) -> &'static str;

    /// Determine frame marker from frame type string
    ///
    /// Codec-specific logic to mark keyframes, errors, etc.
    fn determine_marker(&self, frame_type: &str) -> FrameMarker {
        // Default: mark keyframes
        if frame_type.contains("KEY") || frame_type == "I" {
            FrameMarker::Key
        } else {
            FrameMarker::None
        }
    }
}

// ============================================================================
// AV1 Timeline Extractor
// ============================================================================

/// AV1 timeline extractor
///
/// Converts AV1 frame identity data to timeline visualization.
/// Supports AV1-specific frame types (KEY_FRAME, INTER_FRAME, INTRA_ONLY_FRAME, SWITCH_FRAME).
pub struct Av1TimelineExtractor;

impl TimelineExtractor for Av1TimelineExtractor {
    fn extract_timeline(
        &self,
        stream_id: String,
        index_map: &FrameIndexMap,
        frame_sizes: &[u64],
        frame_types: &[String],
    ) -> TimelineBase {
        let mut timeline = TimelineBase::new(stream_id);

        for display_idx in 0..index_map.frame_count() {
            let size = frame_sizes.get(display_idx).copied().unwrap_or(0);
            let frame_type = frame_types
                .get(display_idx)
                .cloned()
                .unwrap_or_else(|| "UNKNOWN".to_string());

            let marker = self.determine_marker(&frame_type);

            let mut frame = TimelineFrame::new(display_idx, size, frame_type).with_marker(marker);

            // Add PTS/DTS if available
            if let Some(pts) = index_map.get_pts(display_idx) {
                frame = frame.with_pts(pts);
            }
            if let Some(dts) = index_map.get_dts(display_idx) {
                frame = frame.with_dts(dts);
            }

            timeline.add_frame(frame);
        }

        timeline
    }

    fn codec_name(&self) -> &'static str {
        "AV1"
    }

    fn determine_marker(&self, frame_type: &str) -> FrameMarker {
        // AV1-specific frame type detection
        match frame_type {
            "KEY_FRAME" => FrameMarker::Key,
            "INTRA_ONLY_FRAME" => FrameMarker::Key,
            _ => FrameMarker::None,
        }
    }
}

// ============================================================================
// Codec Timeline Extractor Stubs
// ============================================================================

/// H.264 timeline extractor (stub)
pub struct H264TimelineExtractor;

impl TimelineExtractor for H264TimelineExtractor {
    fn extract_timeline(
        &self,
        stream_id: String,
        index_map: &FrameIndexMap,
        frame_sizes: &[u64],
        frame_types: &[String],
    ) -> TimelineBase {
        // Stub: Use default extraction logic with PTS/DTS support
        let mut timeline = TimelineBase::new(stream_id);
        for display_idx in 0..index_map.frame_count() {
            let size = frame_sizes.get(display_idx).copied().unwrap_or(0);
            let frame_type = frame_types
                .get(display_idx)
                .cloned()
                .unwrap_or_else(|| "P".to_string());
            let marker = self.determine_marker(&frame_type);

            let mut frame = TimelineFrame::new(display_idx, size, frame_type).with_marker(marker);

            // Add PTS/DTS if available
            if let Some(pts) = index_map.get_pts(display_idx) {
                frame = frame.with_pts(pts);
            }
            if let Some(dts) = index_map.get_dts(display_idx) {
                frame = frame.with_dts(dts);
            }

            timeline.add_frame(frame);
        }
        timeline
    }

    fn codec_name(&self) -> &'static str {
        "H.264"
    }

    fn determine_marker(&self, frame_type: &str) -> FrameMarker {
        // H.264-specific frame type detection
        match frame_type {
            "IDR" => FrameMarker::Key,
            "I" => FrameMarker::Key,
            _ => FrameMarker::None,
        }
    }
}

/// HEVC timeline extractor (stub)
pub struct HevcTimelineExtractor;

impl TimelineExtractor for HevcTimelineExtractor {
    fn extract_timeline(
        &self,
        stream_id: String,
        index_map: &FrameIndexMap,
        frame_sizes: &[u64],
        frame_types: &[String],
    ) -> TimelineBase {
        // Stub: Use default extraction logic
        let mut timeline = TimelineBase::new(stream_id);
        for display_idx in 0..index_map.frame_count() {
            let size = frame_sizes.get(display_idx).copied().unwrap_or(0);
            let frame_type = frame_types
                .get(display_idx)
                .cloned()
                .unwrap_or_else(|| "P".to_string());
            let marker = self.determine_marker(&frame_type);
            let frame = TimelineFrame::new(display_idx, size, frame_type).with_marker(marker);
            timeline.add_frame(frame);
        }
        timeline
    }

    fn codec_name(&self) -> &'static str {
        "HEVC"
    }
}

/// VP9 timeline extractor (stub)
pub struct Vp9TimelineExtractor;

impl TimelineExtractor for Vp9TimelineExtractor {
    fn extract_timeline(
        &self,
        stream_id: String,
        index_map: &FrameIndexMap,
        frame_sizes: &[u64],
        frame_types: &[String],
    ) -> TimelineBase {
        // Stub: Use default extraction logic
        let mut timeline = TimelineBase::new(stream_id);
        for display_idx in 0..index_map.frame_count() {
            let size = frame_sizes.get(display_idx).copied().unwrap_or(0);
            let frame_type = frame_types
                .get(display_idx)
                .cloned()
                .unwrap_or_else(|| "P".to_string());
            let marker = self.determine_marker(&frame_type);
            let frame = TimelineFrame::new(display_idx, size, frame_type).with_marker(marker);
            timeline.add_frame(frame);
        }
        timeline
    }

    fn codec_name(&self) -> &'static str {
        "VP9"
    }
}

/// VVC timeline extractor (stub)
pub struct VvcTimelineExtractor;

impl TimelineExtractor for VvcTimelineExtractor {
    fn extract_timeline(
        &self,
        stream_id: String,
        index_map: &FrameIndexMap,
        frame_sizes: &[u64],
        frame_types: &[String],
    ) -> TimelineBase {
        // Stub: Use default extraction logic with PTS/DTS support
        let mut timeline = TimelineBase::new(stream_id);
        for display_idx in 0..index_map.frame_count() {
            let size = frame_sizes.get(display_idx).copied().unwrap_or(0);
            let frame_type = frame_types
                .get(display_idx)
                .cloned()
                .unwrap_or_else(|| "P".to_string());
            let marker = self.determine_marker(&frame_type);

            let mut frame = TimelineFrame::new(display_idx, size, frame_type).with_marker(marker);

            // Add PTS/DTS if available
            if let Some(pts) = index_map.get_pts(display_idx) {
                frame = frame.with_pts(pts);
            }
            if let Some(dts) = index_map.get_dts(display_idx) {
                frame = frame.with_dts(dts);
            }

            timeline.add_frame(frame);
        }
        timeline
    }

    fn codec_name(&self) -> &'static str {
        "VVC"
    }
}

/// AVS3 timeline extractor (stub)
pub struct Avs3TimelineExtractor;

impl TimelineExtractor for Avs3TimelineExtractor {
    fn extract_timeline(
        &self,
        stream_id: String,
        index_map: &FrameIndexMap,
        frame_sizes: &[u64],
        frame_types: &[String],
    ) -> TimelineBase {
        // Stub: Use default extraction logic
        let mut timeline = TimelineBase::new(stream_id);
        for display_idx in 0..index_map.frame_count() {
            let size = frame_sizes.get(display_idx).copied().unwrap_or(0);
            let frame_type = frame_types
                .get(display_idx)
                .cloned()
                .unwrap_or_else(|| "P".to_string());
            let marker = self.determine_marker(&frame_type);
            let frame = TimelineFrame::new(display_idx, size, frame_type).with_marker(marker);
            timeline.add_frame(frame);
        }
        timeline
    }

    fn codec_name(&self) -> &'static str {
        "AVS3"
    }
}

// ============================================================================
// Timeline Mapper - High-level Pipeline API (viz_core.003)
// ============================================================================

/// Timeline mapper - complete pipeline from frame data to timeline visualization
///
/// Deliverable: frame_map:FrameIdentity:Timeline:AV1:viz_core
///
/// This type combines:
/// 1. Frame identity mapping (FrameIndexMap)
/// 2. Timeline extraction (TimelineExtractor trait)
/// 3. Complete pipeline from raw data to Timeline viz
///
/// Per FRAME_IDENTITY_CONTRACT:
/// - All operations use display_idx as primary index
/// - decode_idx is internal only
/// - Timeline uses display_idx as horizontal axis
pub struct TimelineMapper {
    /// Stream identifier
    stream_id: String,
    /// Frame index map (identity layer)
    index_map: FrameIndexMap,
    /// Frame sizes in display order
    frame_sizes: Vec<u64>,
    /// Frame types in display order
    frame_types: Vec<String>,
}

impl TimelineMapper {
    /// Create a new timeline mapper from frame metadata
    ///
    /// # Arguments
    ///
    /// * `stream_id` - Unique stream identifier
    /// * `frames` - Frame metadata in decode order (PTS/DTS)
    /// * `frame_sizes` - Frame sizes in decode order
    /// * `frame_types` - Frame type strings in decode order
    ///
    /// # Returns
    ///
    /// TimelineMapper with frames sorted into display order
    ///
    /// # Example
    ///
    /// ```
    /// use bitvue_core::frame_identity::{FrameMetadata, TimelineMapper};
    ///
    /// let frames = vec![
    ///     FrameMetadata { pts: Some(0), dts: Some(0) },
    ///     FrameMetadata { pts: Some(1000), dts: Some(1000) },
    /// ];
    /// let sizes = vec![10000, 3000];
    /// let types = vec!["KEY_FRAME".to_string(), "INTER_FRAME".to_string()];
    ///
    /// let mapper = TimelineMapper::new("stream_A".to_string(), frames, sizes, types);
    /// let timeline = mapper.build_timeline_av1();
    /// ```
    pub fn new(
        stream_id: String,
        frames: Vec<FrameMetadata>,
        frame_sizes: Vec<u64>,
        frame_types: Vec<String>,
    ) -> Self {
        // Build frame index map (sorts into display order)
        let index_map = FrameIndexMap::new(&frames);

        // Reorder frame_sizes and frame_types into display order
        let mut display_sizes = Vec::with_capacity(index_map.frame_count());
        let mut display_types = Vec::with_capacity(index_map.frame_count());

        for display_idx in 0..index_map.frame_count() {
            if let Some(decode_idx) = index_map.display_to_decode_idx(display_idx) {
                let size = frame_sizes.get(decode_idx).copied().unwrap_or(0);
                let frame_type = frame_types
                    .get(decode_idx)
                    .cloned()
                    .unwrap_or_else(|| "UNKNOWN".to_string());

                display_sizes.push(size);
                display_types.push(frame_type);
            } else {
                // Should never happen if FrameIndexMap is valid
                display_sizes.push(0);
                display_types.push("UNKNOWN".to_string());
            }
        }

        Self {
            stream_id,
            index_map,
            frame_sizes: display_sizes,
            frame_types: display_types,
        }
    }

    /// Get the frame index map
    pub fn index_map(&self) -> &FrameIndexMap {
        &self.index_map
    }

    /// Get frame sizes in display order
    pub fn frame_sizes(&self) -> &[u64] {
        &self.frame_sizes
    }

    /// Get frame types in display order
    pub fn frame_types(&self) -> &[String] {
        &self.frame_types
    }

    /// Build timeline using AV1 extractor
    ///
    /// This is the "join" operation that binds:
    /// - Frame identity (display_idx/decode_idx mapping)
    /// - Frame metadata (sizes, types)
    /// - Timeline visualization (TimelineBase)
    pub fn build_timeline_av1(&self) -> TimelineBase {
        let extractor = Av1TimelineExtractor;
        extractor.extract_timeline(
            self.stream_id.clone(),
            &self.index_map,
            &self.frame_sizes,
            &self.frame_types,
        )
    }

    /// Build timeline using H.264 extractor
    pub fn build_timeline_h264(&self) -> TimelineBase {
        let extractor = H264TimelineExtractor;
        extractor.extract_timeline(
            self.stream_id.clone(),
            &self.index_map,
            &self.frame_sizes,
            &self.frame_types,
        )
    }

    /// Build timeline using HEVC extractor
    pub fn build_timeline_hevc(&self) -> TimelineBase {
        let extractor = HevcTimelineExtractor;
        extractor.extract_timeline(
            self.stream_id.clone(),
            &self.index_map,
            &self.frame_sizes,
            &self.frame_types,
        )
    }

    /// Build timeline using VP9 extractor
    pub fn build_timeline_vp9(&self) -> TimelineBase {
        let extractor = Vp9TimelineExtractor;
        extractor.extract_timeline(
            self.stream_id.clone(),
            &self.index_map,
            &self.frame_sizes,
            &self.frame_types,
        )
    }

    /// Build timeline using VVC extractor
    pub fn build_timeline_vvc(&self) -> TimelineBase {
        let extractor = VvcTimelineExtractor;
        extractor.extract_timeline(
            self.stream_id.clone(),
            &self.index_map,
            &self.frame_sizes,
            &self.frame_types,
        )
    }

    /// Build timeline using AVS3 extractor
    pub fn build_timeline_avs3(&self) -> TimelineBase {
        let extractor = Avs3TimelineExtractor;
        extractor.extract_timeline(
            self.stream_id.clone(),
            &self.index_map,
            &self.frame_sizes,
            &self.frame_types,
        )
    }
}

// ============================================================================
// Timeline Axis API (viz_core.007)
// ============================================================================

/// Timeline axis scale mode
///
/// Controls how the timeline axis scales to fit viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisScaleMode {
    /// Auto-scale to fit all frames in viewport
    Auto,
    /// Fixed scale (pixels per frame)
    Fixed(u32),
    /// Fit exactly N frames in viewport
    FitFrames(usize),
}

/// Timeline axis bounds
///
/// Defines the visible range of the timeline axis.
/// Per FRAME_IDENTITY_CONTRACT: bounds are in display_idx coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisBounds {
    /// First visible frame (display_idx)
    pub start: usize,
    /// Last visible frame (display_idx, inclusive)
    pub end: usize,
}

impl AxisBounds {
    /// Create new axis bounds
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get visible frame count
    pub fn frame_count(&self) -> usize {
        if self.end >= self.start {
            self.end - self.start + 1
        } else {
            0
        }
    }

    /// Check if display_idx is within bounds
    pub fn contains(&self, display_idx: usize) -> bool {
        display_idx >= self.start && display_idx <= self.end
    }

    /// Clamp display_idx to bounds
    pub fn clamp(&self, display_idx: usize) -> usize {
        display_idx.clamp(self.start, self.end)
    }
}

/// Timeline axis
///
/// Deliverable: axis:FrameIdentity:Timeline:AV1:viz_core
///
/// Manages timeline horizontal axis with multi-scale support.
/// Per FRAME_IDENTITY_CONTRACT:
/// - Axis uses display_idx as primary coordinate
/// - All queries/operations use display_idx
#[derive(Debug, Clone)]
pub struct TimelineAxis {
    /// Total frame count (from FrameIndexMap)
    total_frames: usize,
    /// Viewport width in pixels
    viewport_width_px: f32,
    /// Scale mode
    scale_mode: AxisScaleMode,
    /// Current visible bounds (display_idx range)
    bounds: AxisBounds,
    /// Minimum pixels per frame (for readability)
    min_pixels_per_frame: f32,
}

impl TimelineAxis {
    /// Create new timeline axis
    ///
    /// # Arguments
    ///
    /// * `total_frames` - Total frame count from FrameIndexMap
    /// * `viewport_width_px` - Viewport width in pixels
    /// * `scale_mode` - Initial scale mode
    pub fn new(total_frames: usize, viewport_width_px: f32, scale_mode: AxisScaleMode) -> Self {
        let bounds = AxisBounds::new(0, total_frames.saturating_sub(1));

        let mut axis = Self {
            total_frames,
            viewport_width_px,
            scale_mode,
            bounds,
            min_pixels_per_frame: 2.0, // Minimum 2px per frame
        };

        axis.update_bounds_from_scale();
        axis
    }

    /// Get current scale mode
    pub fn scale_mode(&self) -> AxisScaleMode {
        self.scale_mode
    }

    /// Set scale mode
    pub fn set_scale_mode(&mut self, mode: AxisScaleMode) {
        self.scale_mode = mode;
        self.update_bounds_from_scale();
    }

    /// Get current bounds
    pub fn bounds(&self) -> AxisBounds {
        self.bounds
    }

    /// Set bounds (manual pan/zoom)
    ///
    /// Clamps to valid range [0, total_frames - 1]
    pub fn set_bounds(&mut self, start: usize, end: usize) {
        let clamped_start = start.min(self.total_frames.saturating_sub(1));
        let clamped_end = end.min(self.total_frames.saturating_sub(1));

        self.bounds = AxisBounds::new(clamped_start, clamped_end);
        self.scale_mode = AxisScaleMode::Fixed(self.pixels_per_frame() as u32);
    }

    /// Update bounds based on current scale mode
    fn update_bounds_from_scale(&mut self) {
        if self.total_frames == 0 {
            // Empty stream: set end < start to get frame_count() = 0
            self.bounds = AxisBounds::new(0, 0);
            self.bounds.end = 0;
            self.bounds.start = 1; // end < start → frame_count = 0
            return;
        }

        match self.scale_mode {
            AxisScaleMode::Auto => {
                // Show all frames
                self.bounds = AxisBounds::new(0, self.total_frames - 1);
            }
            AxisScaleMode::Fixed(pixels_per_frame) => {
                // Keep current bounds, adjust scale
                let visible_frames =
                    (self.viewport_width_px / pixels_per_frame as f32).max(1.0) as usize;
                let end = (self.bounds.start + visible_frames - 1).min(self.total_frames - 1);
                self.bounds.end = end;
            }
            AxisScaleMode::FitFrames(n) => {
                // Show exactly N frames starting from current position
                let n = n.max(1).min(self.total_frames);
                let end = (self.bounds.start + n - 1).min(self.total_frames - 1);
                self.bounds.end = end;
            }
        }
    }

    /// Get pixels per frame for current scale
    pub fn pixels_per_frame(&self) -> f32 {
        let visible_frames = self.bounds.frame_count();
        if visible_frames == 0 {
            return self.min_pixels_per_frame;
        }

        let ppf = self.viewport_width_px / visible_frames as f32;
        ppf.max(self.min_pixels_per_frame)
    }

    /// Convert display_idx to pixel position
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Uses display_idx as input.
    pub fn display_idx_to_pixel(&self, display_idx: usize) -> f32 {
        if display_idx < self.bounds.start {
            return 0.0;
        }

        let offset = (display_idx - self.bounds.start) as f32;
        offset * self.pixels_per_frame()
    }

    /// Convert pixel position to display_idx
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Returns display_idx.
    pub fn pixel_to_display_idx(&self, pixel_x: f32) -> usize {
        let offset = (pixel_x / self.pixels_per_frame()).floor() as usize;
        let display_idx = self.bounds.start + offset;

        // Clamp to valid range
        display_idx.min(self.total_frames.saturating_sub(1))
    }

    /// Pan axis by frame count (positive = right, negative = left)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Operates on display_idx.
    pub fn pan(&mut self, delta_frames: isize) {
        if self.total_frames == 0 {
            return;
        }

        let visible_frames = self.bounds.frame_count();
        let max_start = self.total_frames.saturating_sub(visible_frames);

        let current_start = self.bounds.start as isize;
        let new_start = (current_start + delta_frames)
            .max(0)
            .min(max_start as isize) as usize;

        let new_end = (new_start + visible_frames - 1).min(self.total_frames - 1);

        self.bounds = AxisBounds::new(new_start, new_end);
    }

    /// Zoom in (show fewer frames with more detail)
    pub fn zoom_in(&mut self) {
        let current_visible = self.bounds.frame_count();
        let new_visible = (current_visible / 2).max(1);

        self.scale_mode = AxisScaleMode::FitFrames(new_visible);
        self.update_bounds_from_scale();
    }

    /// Zoom out (show more frames with less detail)
    pub fn zoom_out(&mut self) {
        let current_visible = self.bounds.frame_count();
        let new_visible = (current_visible * 2).min(self.total_frames);

        self.scale_mode = AxisScaleMode::FitFrames(new_visible);
        self.update_bounds_from_scale();
    }

    /// Center axis on specific display_idx
    pub fn center_on(&mut self, display_idx: usize) {
        if self.total_frames == 0 {
            return;
        }

        let visible_frames = self.bounds.frame_count();
        let half = visible_frames / 2;

        // Calculate ideal start (centering on display_idx)
        let ideal_start = display_idx.saturating_sub(half);

        // Clamp to ensure we always show visible_frames frames
        let max_start = self.total_frames.saturating_sub(visible_frames);
        let new_start = ideal_start.min(max_start);

        let new_end = (new_start + visible_frames - 1).min(self.total_frames - 1);

        self.bounds = AxisBounds::new(new_start, new_end);
    }

    /// Get viewport width
    pub fn viewport_width(&self) -> f32 {
        self.viewport_width_px
    }

    /// Set viewport width (triggers rescale)
    pub fn set_viewport_width(&mut self, width_px: f32) {
        self.viewport_width_px = width_px;
        self.update_bounds_from_scale();
    }

    /// Get total frame count
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}

// ============================================================================
// Timeline Cursor API (viz_core.009)
// ============================================================================

/// Cursor/crosshair visibility state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorVisibility {
    /// Cursor is visible
    Visible,
    /// Cursor is hidden
    Hidden,
}

/// Timeline cursor/crosshair
///
/// Deliverable: cursor:FrameIdentity:Timeline:AV1:viz_core
///
/// Global cursor that syncs across timeline, player, and overlays.
/// Per FRAME_IDENTITY_CONTRACT:
/// - Cursor position is in display_idx coordinates
/// - All operations use display_idx
#[derive(Debug, Clone)]
pub struct TimelineCursor {
    /// Current cursor position (display_idx)
    position: Option<usize>,
    /// Visibility state
    visibility: CursorVisibility,
    /// Total frame count (for bounds checking)
    total_frames: usize,
}

impl TimelineCursor {
    /// Create new timeline cursor
    ///
    /// # Arguments
    ///
    /// * `total_frames` - Total frame count from FrameIndexMap
    pub fn new(total_frames: usize) -> Self {
        Self {
            position: None,
            visibility: CursorVisibility::Hidden,
            total_frames,
        }
    }

    /// Get current cursor position (display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Returns display_idx.
    pub fn position(&self) -> Option<usize> {
        self.position
    }

    /// Set cursor position (display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Takes display_idx as input.
    /// Automatically clamps to valid range and shows cursor.
    pub fn set_position(&mut self, display_idx: usize) {
        if self.total_frames == 0 {
            self.position = None;
            self.visibility = CursorVisibility::Hidden;
            return;
        }

        // Clamp to valid range
        let clamped = display_idx.min(self.total_frames - 1);
        self.position = Some(clamped);
        self.visibility = CursorVisibility::Visible;
    }

    /// Clear cursor position (hide cursor)
    pub fn clear(&mut self) {
        self.position = None;
        self.visibility = CursorVisibility::Hidden;
    }

    /// Get visibility state
    pub fn visibility(&self) -> CursorVisibility {
        self.visibility
    }

    /// Show cursor at current position
    pub fn show(&mut self) {
        if self.position.is_some() {
            self.visibility = CursorVisibility::Visible;
        }
    }

    /// Hide cursor (keeps position)
    pub fn hide(&mut self) {
        self.visibility = CursorVisibility::Hidden;
    }

    /// Check if cursor is visible
    pub fn is_visible(&self) -> bool {
        self.visibility == CursorVisibility::Visible && self.position.is_some()
    }

    /// Move cursor by delta frames (positive = right, negative = left)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Operates on display_idx.
    pub fn move_by(&mut self, delta: isize) {
        if let Some(current) = self.position {
            let new_pos = (current as isize + delta).max(0) as usize;
            self.set_position(new_pos);
        }
    }

    /// Move cursor to next frame
    pub fn next_frame(&mut self) {
        self.move_by(1);
    }

    /// Move cursor to previous frame
    pub fn prev_frame(&mut self) {
        self.move_by(-1);
    }

    /// Get total frame count
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}

// ============================================================================
// Cursor Sync - Timeline ↔ Player ↔ Overlays
// ============================================================================

/// Cursor sync coordinator
///
/// Synchronizes cursor position across:
/// - Timeline view (horizontal axis)
/// - Player view (current frame display)
/// - Overlay views (QP heatmap, MV, etc.)
///
/// Per FRAME_IDENTITY_CONTRACT:
/// - All sync operations use display_idx
/// - decode_idx is never exposed
#[derive(Debug, Clone)]
pub struct CursorSync {
    /// Global cursor state
    cursor: TimelineCursor,
    /// Frame index map (for PTS queries)
    index_map: FrameIndexMap,
}

impl CursorSync {
    /// Create new cursor sync coordinator
    pub fn new(index_map: FrameIndexMap) -> Self {
        let total_frames = index_map.frame_count();
        Self {
            cursor: TimelineCursor::new(total_frames),
            index_map,
        }
    }

    /// Get cursor
    pub fn cursor(&self) -> &TimelineCursor {
        &self.cursor
    }

    /// Get mutable cursor
    pub fn cursor_mut(&mut self) -> &mut TimelineCursor {
        &mut self.cursor
    }

    /// Sync cursor from timeline click (pixel → display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Converts pixel to display_idx.
    pub fn sync_from_timeline_click(&mut self, pixel_x: f32, axis: &TimelineAxis) {
        let display_idx = axis.pixel_to_display_idx(pixel_x);
        self.cursor.set_position(display_idx);
    }

    /// Sync cursor from player seek (display_idx)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Takes display_idx as input.
    pub fn sync_from_player_seek(&mut self, display_idx: usize) {
        self.cursor.set_position(display_idx);
    }

    /// Sync cursor from PTS (for external control)
    ///
    /// Converts PTS → display_idx and syncs cursor.
    pub fn sync_from_pts(&mut self, pts: u64) -> bool {
        // Find display_idx for this PTS
        for display_idx in 0..self.index_map.frame_count() {
            if self.index_map.get_pts(display_idx) == Some(pts) {
                self.cursor.set_position(display_idx);
                return true;
            }
        }
        false // PTS not found
    }

    /// Get cursor PTS (for external queries)
    ///
    /// Per FRAME_IDENTITY_CONTRACT: Uses display_idx internally.
    pub fn cursor_pts(&self) -> Option<u64> {
        let display_idx = self.cursor.position()?;
        self.index_map.get_pts(display_idx)
    }

    /// Get frame index map
    pub fn index_map(&self) -> &FrameIndexMap {
        &self.index_map
    }
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("frame_identity_test.rs");
}
