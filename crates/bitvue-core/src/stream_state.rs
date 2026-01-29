//! StreamState - Single source of truth for a loaded stream
//!
//! Monster Pack v3: ARCHITECTURE.md §2.1

use crate::types::SyntaxModel;
use crate::{ByteCache, StreamId, UnitKey};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// StreamState holds all parsed data for a single stream
///
/// This is the core model that UI panels read from.
/// All mutations happen via Command → Core → Event flow.
#[derive(Clone)]
pub struct StreamState {
    /// Stream identifier (A or B for dual view)
    pub stream_id: StreamId,

    /// File path
    pub file_path: Option<PathBuf>,

    /// ByteCache for file access
    pub byte_cache: Option<Arc<ByteCache>>,

    /// Container-level metadata
    pub container: Option<ContainerModel>,

    /// Parsed units (OBUs/NALs)
    pub units: Option<UnitModel>,

    /// Syntax tree for selected unit
    pub syntax: Option<SyntaxModel>,

    /// Timeline index (Phase 1)
    pub timeline: Option<TimelineModel>,

    /// Decoded frames (Phase 2)
    pub frames: Option<FrameModel>,

    /// Quality metrics (Phase 3)
    pub metrics: Option<MetricsModel>,

    /// Diagnostics
    pub diagnostics: Vec<crate::event::Diagnostic>,

    /// File invalidation flag
    pub file_invalidated: bool,
}

impl StreamState {
    /// Create a new empty stream state
    pub fn new(stream_id: StreamId) -> Self {
        Self {
            stream_id,
            file_path: None,
            byte_cache: None,
            container: None,
            units: None,
            syntax: None,
            timeline: None,
            frames: None,
            metrics: None,
            diagnostics: Vec::new(),
            file_invalidated: false,
        }
    }

    /// Check if a file is loaded
    pub fn is_loaded(&self) -> bool {
        self.file_path.is_some() && self.byte_cache.is_some()
    }

    /// Get file size in bytes
    pub fn file_size(&self) -> Option<u64> {
        self.byte_cache.as_ref().map(|cache| cache.len())
    }

    /// Add a diagnostic
    pub fn add_diagnostic(&mut self, diagnostic: crate::event::Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Clear all diagnostics
    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    /// Get diagnostics by severity
    pub fn diagnostics_by_severity(
        &self,
        severity: crate::event::Severity,
    ) -> Vec<&crate::event::Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .collect()
    }
}

/// Container-level metadata
#[derive(Debug, Clone)]
pub struct ContainerModel {
    /// Container format (IVF, MP4, MKV, TS, RAW)
    pub format: ContainerFormat,

    /// Codec type (AV1, H264, HEVC)
    pub codec: String,

    /// Track count (for MP4/MKV)
    pub track_count: usize,

    /// Duration in milliseconds (if available)
    pub duration_ms: Option<u64>,

    /// Total bitrate in bps (if available)
    pub bitrate_bps: Option<u64>,

    /// Frame width (from sequence header)
    pub width: Option<u32>,

    /// Frame height (from sequence header)
    pub height: Option<u32>,

    /// Bit depth (8, 10, 12) (from sequence header)
    pub bit_depth: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerFormat {
    Raw,
    Ivf,
    Mp4,
    Mkv,
    Ts,
}

/// Unit model - tree of parsed bitstream units
#[derive(Debug, Clone)]
pub struct UnitModel {
    /// All units in order
    pub units: Vec<UnitNode>,

    /// Total unit count
    pub unit_count: usize,

    /// Frame count (for display)
    pub frame_count: usize,
}

/// A single unit node in the tree
///
/// Refactored from god object to use composition.
/// Public fields maintained for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitNode {
    /// Unique key for this unit
    pub key: UnitKey,

    /// Unit type (e.g., "SEQUENCE_HEADER", "FRAME", etc.)
    pub unit_type: String,

    /// Byte offset in file
    pub offset: u64,

    /// Size in bytes
    pub size: usize,

    /// Frame index (if this unit is a frame)
    pub frame_index: Option<usize>,

    /// Frame type (KEY, INTER, INTRA_ONLY, SWITCH for AV1; I, P, B for H.264/HEVC)
    pub frame_type: Option<String>,

    /// Presentation timestamp (if available)
    pub pts: Option<u64>,

    /// Decode timestamp (if available, VQAnalyzer parity)
    pub dts: Option<u64>,

    /// Display string for tree view
    pub display_name: String,

    /// Child units (for hierarchical containers like MP4)
    pub children: Vec<UnitNode>,

    /// Average QP for this frame (if available)
    /// AV1: base_q_idx (0-255), H.264/HEVC: avg QP (0-51)
    pub qp_avg: Option<u8>,

    /// Motion vector grid for this frame (if available)
    /// Contains motion vectors extracted from INTER blocks
    pub mv_grid: Option<crate::MVGrid>,

    /// Temporal layer ID (for scalable coding, AV1 temporal_id)
    pub temporal_id: Option<u8>,

    /// Referenced frame indices (for P/B frames)
    /// e.g., [0, 3] means this frame references frames 0 and 3
    pub ref_frames: Option<Vec<usize>>,

    /// Reference slot indices (raw slot numbers from bitstream)
    /// e.g., [2, 4, 7] for AV1 means slots LAST(2), GOLDEN(4), ALTREF(7)
    pub ref_slots: Option<Vec<u8>>,
}

// ============================================================================
// Focused value objects for UnitNode (refactoring from god object)
// ============================================================================

/// Unit header - basic unit metadata
///
/// Contains the essential identification and location information for a unit.
/// Separated from analysis data for clearer separation of concerns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitHeader {
    /// Unique key for this unit
    pub key: UnitKey,

    /// Unit type (e.g., "SEQUENCE_HEADER", "FRAME", etc.)
    pub unit_type: String,

    /// Byte offset in file
    pub offset: u64,

    /// Size in bytes
    pub size: usize,

    /// Display string for tree view
    pub display_name: String,
}

impl UnitHeader {
    /// Create a new unit header
    pub fn new(stream: StreamId, unit_type: String, offset: u64, size: usize) -> Self {
        let key = UnitKey {
            stream,
            unit_type: unit_type.clone(),
            offset,
            size,
        };
        let display_name = format!("{} @ 0x{:08X}", unit_type, offset);

        Self {
            key,
            unit_type,
            offset,
            size,
            display_name,
        }
    }
}

/// Frame information - temporal and frame-specific data
///
/// Contains information specific to frame units (PTS/DTS, frame type, etc.)
/// Separated from analysis data to avoid mixing concerns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameInfo {
    /// Frame index (if this unit is a frame)
    pub frame_index: Option<usize>,

    /// Frame type (KEY, INTER, INTRA_ONLY, SWITCH for AV1; I, P, B for H.264/HEVC)
    pub frame_type: Option<String>,

    /// Presentation timestamp (if available)
    pub pts: Option<u64>,

    /// Decode timestamp (if available, VQAnalyzer parity)
    pub dts: Option<u64>,

    /// Temporal layer ID (for scalable coding, AV1 temporal_id)
    pub temporal_id: Option<u8>,
}

/// Frame analysis - QP, motion vectors, reference data
///
/// Contains analysis data extracted from the frame.
/// Separated from basic frame info for clearer separation of concerns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameAnalysis {
    /// Average QP for this frame (if available)
    /// AV1: base_q_idx (0-255), H.264/HEVC: avg QP (0-51)
    pub qp_avg: Option<u8>,

    /// Motion vector grid for this frame (if available)
    /// Contains motion vectors extracted from INTER blocks
    pub mv_grid: Option<crate::MVGrid>,

    /// Referenced frame indices (for P/B frames)
    /// e.g., [0, 3] means this frame references frames 0 and 3
    pub ref_frames: Option<Vec<usize>>,

    /// Reference slot indices (raw slot numbers from bitstream)
    /// e.g., [2, 4, 7] for AV1 means slots LAST(2), GOLDEN(4), ALTREF(7)
    pub ref_slots: Option<Vec<u8>>,
}

impl UnitNode {
    /// Create a new unit node
    pub fn new(stream: StreamId, unit_type: String, offset: u64, size: usize) -> Self {
        let key = UnitKey {
            stream,
            unit_type: unit_type.clone(),
            offset,
            size,
        };

        let display_name = format!("{} @ 0x{:08X}", unit_type, offset);

        Self {
            key,
            unit_type,
            offset,
            size,
            frame_index: None,
            frame_type: None,
            pts: None,
            dts: None,
            display_name,
            children: Vec::new(),
            qp_avg: None,
            mv_grid: None,
            temporal_id: None,
            ref_frames: None,
            ref_slots: None,
        }
    }

    /// Set QP average for this unit
    pub fn with_qp(mut self, qp: u8) -> Self {
        self.qp_avg = Some(qp);
        self
    }

    /// Set frame type for this unit
    pub fn with_frame_type(mut self, frame_type: impl Into<String>) -> Self {
        self.frame_type = Some(frame_type.into());
        self
    }

    // ============================================================================
    // New methods using focused value types (god object refactoring)
    // ============================================================================

    /// Create UnitNode from focused components
    ///
    /// This is the preferred way to construct UnitNode after refactoring,
    /// as it clearly separates concerns.
    pub fn from_components(
        header: UnitHeader,
        frame_info: FrameInfo,
        analysis: FrameAnalysis,
        children: Vec<UnitNode>,
    ) -> Self {
        Self {
            key: header.key,
            unit_type: header.unit_type,
            offset: header.offset,
            size: header.size,
            display_name: header.display_name,
            frame_index: frame_info.frame_index,
            frame_type: frame_info.frame_type,
            pts: frame_info.pts,
            dts: frame_info.dts,
            temporal_id: frame_info.temporal_id,
            qp_avg: analysis.qp_avg,
            mv_grid: analysis.mv_grid,
            ref_frames: analysis.ref_frames,
            ref_slots: analysis.ref_slots,
            children,
        }
    }

    /// Extract unit header from this node
    pub fn header(&self) -> UnitHeader {
        UnitHeader {
            key: self.key.clone(),
            unit_type: self.unit_type.clone(),
            offset: self.offset,
            size: self.size,
            display_name: self.display_name.clone(),
        }
    }

    /// Extract frame info from this node
    pub fn frame_info(&self) -> FrameInfo {
        FrameInfo {
            frame_index: self.frame_index,
            frame_type: self.frame_type.clone(),
            pts: self.pts,
            dts: self.dts,
            temporal_id: self.temporal_id,
        }
    }

    /// Extract frame analysis from this node
    pub fn analysis(&self) -> FrameAnalysis {
        FrameAnalysis {
            qp_avg: self.qp_avg,
            mv_grid: self.mv_grid.clone(),
            ref_frames: self.ref_frames.clone(),
            ref_slots: self.ref_slots.clone(),
        }
    }

    /// Set frame info using the focused type
    pub fn with_frame_info(mut self, info: FrameInfo) -> Self {
        self.frame_index = info.frame_index;
        self.frame_type = info.frame_type;
        self.pts = info.pts;
        self.dts = info.dts;
        self.temporal_id = info.temporal_id;
        self
    }

    /// Set frame analysis using the focused type
    pub fn with_analysis(mut self, analysis: FrameAnalysis) -> Self {
        self.qp_avg = analysis.qp_avg;
        self.mv_grid = analysis.mv_grid;
        self.ref_frames = analysis.ref_frames;
        self.ref_slots = analysis.ref_slots;
        self
    }

    /// Check if this unit has frame information
    pub fn has_frame_info(&self) -> bool {
        self.frame_index.is_some() || self.frame_type.is_some()
    }

    /// Check if this unit has analysis data
    pub fn has_analysis(&self) -> bool {
        self.qp_avg.is_some()
            || self.mv_grid.is_some()
            || self.ref_frames.is_some()
            || self.ref_slots.is_some()
    }
}

// SyntaxModel and SyntaxNode are now defined in types.rs
// Import them from crate::types via the parent module

/// Timeline model (Phase 1)
#[derive(Debug, Clone)]
pub struct TimelineModel {
    /// Frame identity resolver (T0-1: FRAME_IDENTITY_CONTRACT)
    /// Primary timeline index = display_idx (PTS order)
    pub frame_index_map: crate::frame_identity::FrameIndexMap,

    /// Frame entries for timeline
    pub frames: Vec<TimelineFrame>,

    /// LOD levels for zoom (Level 0 = full detail)
    pub lod_levels: Vec<TimelineLod>,
}

#[derive(Debug, Clone)]
pub struct TimelineFrame {
    /// Frame index
    pub index: usize,

    /// Frame type (I, P, B for AV1: Key, Inter, IntraOnly, Switch)
    pub frame_type: String,

    /// Size in bytes
    pub size_bytes: usize,

    /// Presentation timestamp
    pub pts: Option<u64>,

    /// Has errors
    pub has_error: bool,

    /// Is bookmarked
    pub is_bookmark: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineLod {
    /// LOD level (0 = highest detail)
    pub level: u32,

    /// Aggregated frame groups
    pub groups: Vec<TimelineGroup>,
}

#[derive(Debug, Clone)]
pub struct TimelineGroup {
    /// Start frame index
    pub start_frame: usize,

    /// End frame index (exclusive)
    pub end_frame: usize,

    /// Average size in bytes
    pub avg_size_bytes: usize,

    /// Has keyframe
    pub has_keyframe: bool,

    /// Has error
    pub has_error: bool,
}

/// Frame model (Phase 2) with LRU cache for seek performance
///
/// Uses a proper LRU cache with 32 frame capacity to optimize
/// back-and-forth seeking common in video analysis workflows.
#[derive(Debug)]
pub struct FrameModel {
    /// Decoded frames cache (LRU for seek optimization)
    cache: lru::LruCache<usize, CachedFrame>,
}

impl FrameModel {
    /// Create a new FrameModel with optimized cache size for seeking
    ///
    /// Default capacity of 32 frames balances memory usage with
    /// smooth back-and-forth navigation during frame analysis.
    pub fn new() -> Self {
        Self {
            cache: lru::LruCache::new(std::num::NonZeroUsize::new(32).unwrap()),
        }
    }

    /// Create with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: lru::LruCache::new(std::num::NonZeroUsize::new(capacity.max(1)).unwrap()),
        }
    }

    /// Insert a frame into the cache (LRU eviction automatic)
    pub fn insert_lru(&mut self, frame: CachedFrame) {
        let idx = frame.index;
        self.cache.put(idx, frame);
    }

    /// Get a frame from the cache (updates LRU order)
    pub fn get(&mut self, index: usize) -> Option<&CachedFrame> {
        self.cache.get(&index)
    }

    /// Peek at a frame without updating LRU order
    pub fn peek(&self, index: usize) -> Option<&CachedFrame> {
        self.cache.peek(&index)
    }

    /// Check if frame is in cache without affecting LRU order
    pub fn contains(&self, index: usize) -> bool {
        self.cache.contains(&index)
    }

    /// Get cache statistics for debugging
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.cap().get())
    }
}

impl Default for FrameModel {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for FrameModel {
    fn clone(&self) -> Self {
        let mut new_cache = lru::LruCache::new(self.cache.cap());
        // Copy entries preserving LRU order (oldest to newest)
        for (k, v) in self.cache.iter().rev() {
            new_cache.put(*k, v.clone());
        }
        Self { cache: new_cache }
    }
}

/// Cached decoded frame (Phase 2)
///
/// Refactored from god object to use composition.
/// Public fields maintained for backward compatibility.
///
/// Plane data is wrapped in Arc for efficient cloning and sharing
/// with DecodedFrame, avoiding expensive memory copies.
#[derive(Debug, Clone)]
pub struct CachedFrame {
    /// Frame index
    pub index: usize,

    /// RGB data (RGB8 packed: r,g,b,r,g,b,...)
    pub rgb_data: Vec<u8>,

    /// Width
    pub width: u32,

    /// Height
    pub height: u32,

    /// Decoded successfully
    pub decoded: bool,

    /// Error message (if decode failed)
    pub error: Option<String>,

    // YUV plane data (Phase 2 - for YuvViewerPanel)
    /// Y plane data (luma) - Arc-wrapped for cheap cloning from DecodedFrame
    pub y_plane: Option<std::sync::Arc<Vec<u8>>>,

    /// U plane data (chroma Cb) - may be subsampled, Arc-wrapped for cheap cloning
    pub u_plane: Option<std::sync::Arc<Vec<u8>>>,

    /// V plane data (chroma Cr) - may be subsampled, Arc-wrapped for cheap cloning
    pub v_plane: Option<std::sync::Arc<Vec<u8>>>,

    /// Chroma width (may differ from width for 4:2:0/4:2:2)
    pub chroma_width: Option<u32>,

    /// Chroma height (may differ from height for 4:2:0)
    pub chroma_height: Option<u32>,
}

impl CachedFrame {
    // ========================================================================
    // New methods using focused value types (god object refactoring)
    // ========================================================================

    /// Create CachedFrame from focused components
    ///
    /// This is the preferred way to construct CachedFrame after refactoring.
    pub fn from_components(metadata: FrameMetadata, rgb: FrameRgbData, yuv: FrameYuvData) -> Self {
        Self {
            index: metadata.index,
            width: metadata.width,
            height: metadata.height,
            decoded: metadata.decoded,
            error: metadata.error,
            rgb_data: rgb.data,
            y_plane: yuv.y_plane,
            u_plane: yuv.u_plane,
            v_plane: yuv.v_plane,
            chroma_width: yuv.chroma_width,
            chroma_height: yuv.chroma_height,
        }
    }

    /// Extract frame metadata from this cached frame
    pub fn metadata(&self) -> FrameMetadata {
        FrameMetadata {
            index: self.index,
            width: self.width,
            height: self.height,
            decoded: self.decoded,
            error: self.error.clone(),
        }
    }

    /// Extract RGB data from this cached frame
    pub fn rgb(&self) -> FrameRgbData {
        FrameRgbData {
            data: self.rgb_data.clone(),
        }
    }

    /// Extract YUV data from this cached frame
    pub fn yuv(&self) -> FrameYuvData {
        FrameYuvData {
            y_plane: self.y_plane.clone(),
            u_plane: self.u_plane.clone(),
            v_plane: self.v_plane.clone(),
            chroma_width: self.chroma_width,
            chroma_height: self.chroma_height,
        }
    }

    /// Check if frame decode was successful
    pub fn is_valid(&self) -> bool {
        self.decoded && self.error.is_none()
    }

    /// Check if frame has RGB data
    pub fn has_rgb(&self) -> bool {
        !self.rgb_data.is_empty()
    }

    /// Check if frame has YUV data
    pub fn has_yuv(&self) -> bool {
        self.y_plane.is_some() && self.u_plane.is_some() && self.v_plane.is_some()
    }

    /// Get frame size in bytes for RGB data
    pub fn rgb_size(&self) -> usize {
        self.rgb_data.len()
    }

    /// Get expected RGB size based on dimensions
    pub fn expected_rgb_size(&self) -> usize {
        (self.width * self.height * 3) as usize
    }

    /// Validate RGB data size matches dimensions
    pub fn is_rgb_valid(&self) -> bool {
        self.rgb_data.len() == self.expected_rgb_size()
    }
}

// ============================================================================
// Focused value objects for CachedFrame (god object refactoring)
// ============================================================================

/// Frame metadata - dimensions and status
///
/// Contains frame identification and status information,
/// separated from pixel data for clearer separation of concerns.
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    /// Frame index
    pub index: usize,

    /// Width
    pub width: u32,

    /// Height
    pub height: u32,

    /// Decoded successfully
    pub decoded: bool,

    /// Error message (if decode failed)
    pub error: Option<String>,
}

impl FrameMetadata {
    /// Create new frame metadata
    pub fn new(index: usize, width: u32, height: u32) -> Self {
        Self {
            index,
            width,
            height,
            decoded: true,
            error: None,
        }
    }

    /// Create metadata for failed decode
    pub fn failed(index: usize, width: u32, height: u32, error: String) -> Self {
        Self {
            index,
            width,
            height,
            decoded: false,
            error: Some(error),
        }
    }
}

/// RGB pixel data for a frame
///
/// Contains the RGB color data for a frame,
/// separated from metadata for flexibility.
#[derive(Debug, Clone)]
pub struct FrameRgbData {
    /// RGB data (RGB8 packed: r,g,b,r,g,b,...)
    pub data: Vec<u8>,
}

impl FrameRgbData {
    /// Create new RGB data
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Get expected size for RGB data
    pub fn expected_size(width: u32, height: u32) -> usize {
        (width * height * 3) as usize
    }

    /// Validate size matches dimensions
    pub fn is_valid_for(&self, width: u32, height: u32) -> bool {
        self.data.len() == Self::expected_size(width, height)
    }
}

/// YUV plane data for a frame
///
/// Contains YUV color data with chroma subsampling information,
/// separated from metadata and RGB data.
#[derive(Debug, Clone, Default)]
pub struct FrameYuvData {
    /// Y plane data (luma) - Arc-wrapped for cheap cloning
    pub y_plane: Option<std::sync::Arc<Vec<u8>>>,

    /// U plane data (chroma Cb) - may be subsampled, Arc-wrapped for cheap cloning
    pub u_plane: Option<std::sync::Arc<Vec<u8>>>,

    /// V plane data (chroma Cr) - may be subsampled, Arc-wrapped for cheap cloning
    pub v_plane: Option<std::sync::Arc<Vec<u8>>>,

    /// Chroma width (may differ from width for 4:2:0/4:2:2)
    pub chroma_width: Option<u32>,

    /// Chroma height (may differ from height for 4:2:0)
    pub chroma_height: Option<u32>,
}

impl FrameYuvData {
    /// Check if YUV data is present
    pub fn has_yuv(&self) -> bool {
        self.y_plane.is_some() && self.u_plane.is_some() && self.v_plane.is_some()
    }

    /// Get chroma subsampling format name
    pub fn subsampling_name(&self) -> Option<&'static str> {
        match (self.chroma_width, self.chroma_height) {
            (None, None) => None,
            (Some(_), Some(_)) => Some("4:2:0"), // Simplified, actual detection more complex
            _ => Some("unknown"),
        }
    }
}

/// Metrics model (Phase 3)
#[derive(Debug, Clone)]
pub struct MetricsModel {
    /// Per-frame PSNR values
    pub psnr: Vec<f32>,

    /// Per-frame SSIM values
    pub ssim: Vec<f32>,

    /// Per-frame VMAF values (if available)
    pub vmaf: Option<Vec<f32>>,
}

/// Per generate-tests skill: Comprehensive test suite with Arrange-Act-Assert pattern
#[cfg(test)]
mod tests {
    include!("stream_state_test.rs");
}
