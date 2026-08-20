//! Timeline extractor API: bridges frame identity ([`super::FrameIndexMap`]) with timeline
//! visualization ([`crate::timeline::TimelineBase`]) (viz_core.002/viz_core.003).

use super::FrameIndexMap;
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
    ///
    /// Default implementation provides common extraction logic for all codecs.
    /// Codecs can override if they need custom behavior.
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
                .unwrap_or_else(|| self.default_frame_type().to_string());

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

    /// Get codec name
    fn codec_name(&self) -> &'static str;

    /// Get default frame type string for unknown frames
    ///
    /// Different codecs use different conventions:
    /// - AV1: "UNKNOWN"
    /// - H.264, HEVC, VP9, VVC: "P" (Predicted)
    ///
    /// Codecs can override this method to use their convention.
    fn default_frame_type(&self) -> &'static str {
        "P"
    }

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
    fn codec_name(&self) -> &'static str {
        "AV1"
    }

    fn default_frame_type(&self) -> &'static str {
        // AV1 uses "UNKNOWN" as the default frame type
        "UNKNOWN"
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
// Codec Timeline Extractor Implementations
// ============================================================================

/// H.264 timeline extractor
pub struct H264TimelineExtractor;

impl TimelineExtractor for H264TimelineExtractor {
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

/// HEVC timeline extractor
pub struct HevcTimelineExtractor;

impl TimelineExtractor for HevcTimelineExtractor {
    fn codec_name(&self) -> &'static str {
        "HEVC"
    }
}

/// VP9 timeline extractor
pub struct Vp9TimelineExtractor;

impl TimelineExtractor for Vp9TimelineExtractor {
    fn codec_name(&self) -> &'static str {
        "VP9"
    }
}

/// VVC timeline extractor
pub struct VvcTimelineExtractor;

impl TimelineExtractor for VvcTimelineExtractor {
    fn codec_name(&self) -> &'static str {
        "VVC"
    }
}

/// AVS3 timeline extractor
pub struct Avs3TimelineExtractor;

impl TimelineExtractor for Avs3TimelineExtractor {
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
    /// use bitvue_engine::frame_identity::{FrameMetadata, TimelineMapper};
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
        frames: Vec<super::FrameMetadata>,
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
