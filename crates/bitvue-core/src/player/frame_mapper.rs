//! Frame Mapping Join - viz_core.003
//!
//! Binds extracted units to display_idx/decode_idx and pts/dts mapping.
//!
//! FRAME_IDENTITY_CONTRACT:
//! - display_idx is PRIMARY (PTS-sorted display order)
//! - decode_idx is INTERNAL ONLY (DTS-sorted decoder order)
//! - All public APIs use display_idx
//! - decode_idx never exposed to UI

use crate::player::extractor::ExtractedFrame;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Frame mapping entry
///
/// FRAME_IDENTITY_CONTRACT: display_idx is PRIMARY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMapEntry {
    /// Display index (PTS-sorted order) - PRIMARY identifier
    pub display_idx: usize,

    /// Decode index (DTS-sorted order) - INTERNAL ONLY
    pub(crate) decode_idx: usize,

    /// Presentation timestamp
    pub pts: u64,

    /// Decode timestamp (internal only)
    pub(crate) dts: u64,

    /// Frame type
    pub frame_type: String,

    /// Bit offset in bitstream
    pub bit_offset: u64,

    /// Frame size in bytes
    pub size_bytes: usize,
}

impl FrameMapEntry {
    /// Get display index (PUBLIC API)
    pub fn display_idx(&self) -> usize {
        self.display_idx
    }

    /// Get PTS (PUBLIC API)
    pub fn pts(&self) -> u64 {
        self.pts
    }

    /// Get frame type (PUBLIC API)
    pub fn frame_type(&self) -> &str {
        &self.frame_type
    }

    /// Get bit offset (PUBLIC API)
    pub fn bit_offset(&self) -> u64 {
        self.bit_offset
    }
}

/// Frame mapper - joins display and decode order
///
/// Per FRAME_IDENTITY_CONTRACT:
/// - Primary index: display_idx (PTS order)
/// - Internal only: decode_idx (DTS order)
/// - Never expose decode_idx in public API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMapper {
    /// Frame entries indexed by display_idx (PTS order)
    frames: Vec<FrameMapEntry>,

    /// Quick lookup: display_idx -> decode_idx (internal)
    display_to_decode: HashMap<usize, usize>,

    /// Quick lookup: decode_idx -> display_idx
    decode_to_display: HashMap<usize, usize>,

    /// Quick lookup: PTS -> display_idx
    pts_to_display: HashMap<u64, usize>,

    /// Total frame count
    count: usize,
}

impl FrameMapper {
    /// Create new frame mapper from extracted frames
    ///
    /// IMPORTANT: This sorts frames by PTS to establish display order
    pub fn new(mut extracted_frames: Vec<ExtractedFrame>) -> Self {
        // Sort by PTS to get display order
        extracted_frames.sort_by_key(|f| f.pts);

        let count = extracted_frames.len();
        let mut frames = Vec::with_capacity(count);
        let mut display_to_decode = HashMap::new();
        let mut decode_to_display = HashMap::new();
        let mut pts_to_display = HashMap::new();

        // Assign display_idx based on PTS-sorted order
        for (display_idx, extracted) in extracted_frames.iter().enumerate() {
            let decode_idx = extracted.display_idx; // Original index is decode order

            let entry = FrameMapEntry {
                display_idx,
                decode_idx,
                pts: extracted.pts,
                dts: extracted.dts,
                frame_type: extracted.frame_type.clone(),
                bit_offset: extracted.bit_offset,
                size_bytes: extracted.size_bytes,
            };

            frames.push(entry);
            display_to_decode.insert(display_idx, decode_idx);
            decode_to_display.insert(decode_idx, display_idx);
            pts_to_display.insert(extracted.pts, display_idx);
        }

        Self {
            frames,
            display_to_decode,
            decode_to_display,
            pts_to_display,
            count,
        }
    }

    /// Get frame count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get frame by display_idx (PUBLIC API)
    pub fn get(&self, display_idx: usize) -> Option<&FrameMapEntry> {
        self.frames.get(display_idx)
    }

    /// Get frame by PTS (PUBLIC API)
    pub fn get_by_pts(&self, pts: u64) -> Option<&FrameMapEntry> {
        self.pts_to_display
            .get(&pts)
            .and_then(|idx| self.frames.get(*idx))
    }

    /// Find frame by PTS (nearest match)
    pub fn find_by_pts(&self, pts: u64) -> Option<&FrameMapEntry> {
        // First try exact match
        if let Some(frame) = self.get_by_pts(pts) {
            return Some(frame);
        }

        // Find nearest frame with PTS <= target
        self.frames
            .iter()
            .rev()
            .find(|f| f.pts <= pts)
            .or_else(|| self.frames.first())
    }

    /// Get all frames in display order (PUBLIC API)
    pub fn frames(&self) -> &[FrameMapEntry] {
        &self.frames
    }

    /// Get frame range in display order
    pub fn range(&self, start: usize, end: usize) -> &[FrameMapEntry] {
        let end = end.min(self.count);
        &self.frames[start..end]
    }

    /// Check if display_idx is valid
    pub fn is_valid_display_idx(&self, display_idx: usize) -> bool {
        display_idx < self.count
    }

    /// Check if PTS exists
    pub fn has_pts(&self, pts: u64) -> bool {
        self.pts_to_display.contains_key(&pts)
    }

    /// Get PTS range (min, max)
    pub fn pts_range(&self) -> Option<(u64, u64)> {
        if self.frames.is_empty() {
            None
        } else {
            Some((
                self.frames.first().unwrap().pts,
                self.frames.last().unwrap().pts,
            ))
        }
    }

    /// Internal: Get decode_idx for display_idx
    ///
    /// IMPORTANT: This is internal only, never expose to public API
    #[allow(dead_code)]
    pub(crate) fn display_to_decode_idx(&self, display_idx: usize) -> Option<usize> {
        self.display_to_decode.get(&display_idx).copied()
    }

    /// Internal: Get display_idx for decode_idx
    ///
    /// Used internally for decoder feedback
    #[allow(dead_code)]
    pub(crate) fn decode_to_display_idx(&self, decode_idx: usize) -> Option<usize> {
        self.decode_to_display.get(&decode_idx).copied()
    }

    /// Check if stream has reordered frames (B-frames)
    ///
    /// Returns true if any frame has display_idx != decode_idx
    pub fn has_reordering(&self) -> bool {
        self.frames.iter().any(|f| f.display_idx != f.decode_idx)
    }

    /// Get reordering statistics
    pub fn reordering_stats(&self) -> ReorderingStats {
        let mut max_reorder_distance = 0;
        let mut reordered_count = 0;

        for frame in &self.frames {
            if frame.display_idx != frame.decode_idx {
                reordered_count += 1;
                let distance = (frame.display_idx as i64 - frame.decode_idx as i64).abs();
                max_reorder_distance = max_reorder_distance.max(distance as usize);
            }
        }

        ReorderingStats {
            total_frames: self.count,
            reordered_frames: reordered_count,
            max_reorder_distance,
            has_reordering: reordered_count > 0,
        }
    }
}

/// Reordering statistics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReorderingStats {
    /// Total frame count
    pub total_frames: usize,

    /// Number of reordered frames
    pub reordered_frames: usize,

    /// Maximum reordering distance
    pub max_reorder_distance: usize,

    /// Whether stream has any reordering
    pub has_reordering: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frames() -> Vec<ExtractedFrame> {
        // Create frames with PTS != DTS (simulating B-frames)
        // Display order (PTS): 0, 1, 2, 3, 4
        // Decode order (DTS):  0, 3, 1, 2, 4

        vec![
            ExtractedFrame {
                display_idx: 0, // decode_idx
                pts: 0,
                dts: 0,
                frame_type: "I".to_string(),
                size_bytes: 1000,
                bit_offset: 0,
                qp_map: None,
                motion_vectors: None,
                partitions: None,
            },
            ExtractedFrame {
                display_idx: 1, // decode_idx
                pts: 99,        // PTS for frame 3 in display order
                dts: 33,
                frame_type: "P".to_string(),
                size_bytes: 800,
                bit_offset: 1000,
                qp_map: None,
                motion_vectors: None,
                partitions: None,
            },
            ExtractedFrame {
                display_idx: 2, // decode_idx
                pts: 33,        // PTS for frame 1 in display order
                dts: 66,
                frame_type: "B".to_string(),
                size_bytes: 500,
                bit_offset: 1800,
                qp_map: None,
                motion_vectors: None,
                partitions: None,
            },
            ExtractedFrame {
                display_idx: 3, // decode_idx
                pts: 66,        // PTS for frame 2 in display order
                dts: 99,
                frame_type: "B".to_string(),
                size_bytes: 500,
                bit_offset: 2300,
                qp_map: None,
                motion_vectors: None,
                partitions: None,
            },
            ExtractedFrame {
                display_idx: 4, // decode_idx
                pts: 132,
                dts: 132,
                frame_type: "P".to_string(),
                size_bytes: 800,
                bit_offset: 2800,
                qp_map: None,
                motion_vectors: None,
                partitions: None,
            },
        ]
    }

    #[test]
    fn test_frame_mapper_creation() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        assert_eq!(mapper.count(), 5);
    }

    #[test]
    fn test_frames_sorted_by_pts() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // Verify frames are in PTS order
        let frames = mapper.frames();
        assert_eq!(frames[0].pts, 0);
        assert_eq!(frames[1].pts, 33);
        assert_eq!(frames[2].pts, 66);
        assert_eq!(frames[3].pts, 99);
        assert_eq!(frames[4].pts, 132);
    }

    #[test]
    fn test_display_idx_is_primary() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // Frame 0 in display order (PTS=0)
        let frame_0 = mapper.get(0).unwrap();
        assert_eq!(frame_0.display_idx(), 0);
        assert_eq!(frame_0.pts(), 0);

        // Frame 1 in display order (PTS=33, originally decode_idx=2)
        let frame_1 = mapper.get(1).unwrap();
        assert_eq!(frame_1.display_idx(), 1);
        assert_eq!(frame_1.pts(), 33);
        assert_eq!(frame_1.decode_idx, 2); // Internal only

        // Frame 3 in display order (PTS=99, originally decode_idx=1)
        let frame_3 = mapper.get(3).unwrap();
        assert_eq!(frame_3.display_idx(), 3);
        assert_eq!(frame_3.pts(), 99);
        assert_eq!(frame_3.decode_idx, 1); // Internal only
    }

    #[test]
    fn test_get_by_pts() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        let frame = mapper.get_by_pts(33).unwrap();
        assert_eq!(frame.display_idx(), 1);
        assert_eq!(frame.pts(), 33);

        let frame = mapper.get_by_pts(99).unwrap();
        assert_eq!(frame.display_idx(), 3);
        assert_eq!(frame.pts(), 99);
    }

    #[test]
    fn test_find_by_pts_exact() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        let frame = mapper.find_by_pts(66).unwrap();
        assert_eq!(frame.display_idx(), 2);
        assert_eq!(frame.pts(), 66);
    }

    #[test]
    fn test_find_by_pts_nearest() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // PTS between frames - should find previous frame
        let frame = mapper.find_by_pts(50).unwrap();
        assert_eq!(frame.pts(), 33); // Nearest frame with PTS <= 50

        let frame = mapper.find_by_pts(100).unwrap();
        assert_eq!(frame.pts(), 99); // Nearest frame with PTS <= 100
    }

    #[test]
    fn test_pts_range() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        let (min_pts, max_pts) = mapper.pts_range().unwrap();
        assert_eq!(min_pts, 0);
        assert_eq!(max_pts, 132);
    }

    #[test]
    fn test_has_reordering() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        assert!(mapper.has_reordering());
    }

    #[test]
    fn test_no_reordering() {
        // Create frames with PTS == DTS (no reordering)
        let frames = vec![
            ExtractedFrame::new(0, 0, 0, "I".to_string()),
            ExtractedFrame::new(1, 33, 33, "P".to_string()),
            ExtractedFrame::new(2, 66, 66, "P".to_string()),
        ];

        let mapper = FrameMapper::new(frames);
        assert!(!mapper.has_reordering());
    }

    #[test]
    fn test_reordering_stats() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        let stats = mapper.reordering_stats();

        assert_eq!(stats.total_frames, 5);
        assert!(stats.has_reordering);
        assert!(stats.reordered_frames > 0);
        assert!(stats.max_reorder_distance > 0);
    }

    #[test]
    fn test_frame_range() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        let range = mapper.range(1, 4);
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].display_idx(), 1);
        assert_eq!(range[1].display_idx(), 2);
        assert_eq!(range[2].display_idx(), 3);
    }

    #[test]
    fn test_is_valid_display_idx() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        assert!(mapper.is_valid_display_idx(0));
        assert!(mapper.is_valid_display_idx(4));
        assert!(!mapper.is_valid_display_idx(5));
        assert!(!mapper.is_valid_display_idx(100));
    }

    #[test]
    fn test_has_pts() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        assert!(mapper.has_pts(0));
        assert!(mapper.has_pts(33));
        assert!(!mapper.has_pts(50));
        assert!(!mapper.has_pts(200));
    }

    #[test]
    fn test_frame_identity_contract_display_idx_primary() {
        // Verify FRAME_IDENTITY_CONTRACT: display_idx is PRIMARY
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // Public API uses display_idx
        for display_idx in 0..mapper.count() {
            let frame = mapper.get(display_idx).unwrap();

            // Verify display_idx is sequential
            assert_eq!(frame.display_idx(), display_idx);

            // Verify public accessors use display_idx
            assert_eq!(frame.display_idx(), display_idx);
        }
    }

    #[test]
    fn test_frame_identity_contract_decode_idx_internal() {
        // Verify FRAME_IDENTITY_CONTRACT: decode_idx is INTERNAL ONLY
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // decode_idx should not be exposed in public API
        // It's only accessible via pub(crate) methods

        // Frame with display_idx=1 has decode_idx=2
        let frame_1 = mapper.get(1).unwrap();
        assert_eq!(frame_1.display_idx(), 1);
        assert_eq!(frame_1.decode_idx, 2); // Internal field

        // Verify internal mapping works
        assert_eq!(mapper.display_to_decode_idx(1), Some(2));
        assert_eq!(mapper.decode_to_display_idx(2), Some(1));
    }

    #[test]
    fn test_reordered_frames_use_display_idx() {
        // Verify that even with B-frame reordering, display_idx is PRIMARY
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // Display order (PTS): 0(I), 33(B), 66(B), 99(P), 132(P)
        // Decode order (DTS): 0(I), 99(P), 33(B), 66(B), 132(P)

        // Frame at display_idx=1 (PTS=33, B-frame)
        let frame = mapper.get(1).unwrap();
        assert_eq!(frame.display_idx(), 1);
        assert_eq!(frame.pts(), 33);
        assert_eq!(frame.frame_type(), "B");

        // This frame was decoded 3rd (decode_idx=2)
        assert_eq!(frame.decode_idx, 2);

        // But in display order, it's 2nd (display_idx=1)
        assert_eq!(frame.display_idx(), 1);
    }

    #[test]
    fn test_empty_mapper() {
        let mapper = FrameMapper::new(Vec::new());

        assert_eq!(mapper.count(), 0);
        assert!(mapper.get(0).is_none());
        assert!(mapper.pts_range().is_none());
        assert!(!mapper.has_reordering());
    }

    #[test]
    fn test_single_frame() {
        let frames = vec![ExtractedFrame::new(0, 0, 0, "I".to_string())];
        let mapper = FrameMapper::new(frames);

        assert_eq!(mapper.count(), 1);
        let frame = mapper.get(0).unwrap();
        assert_eq!(frame.display_idx(), 0);
        assert_eq!(frame.pts(), 0);
    }

    #[test]
    fn test_display_to_decode_mapping() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // Verify internal mapping
        assert_eq!(mapper.display_to_decode_idx(0), Some(0)); // I-frame
        assert_eq!(mapper.display_to_decode_idx(1), Some(2)); // B-frame (reordered)
        assert_eq!(mapper.display_to_decode_idx(2), Some(3)); // B-frame (reordered)
        assert_eq!(mapper.display_to_decode_idx(3), Some(1)); // P-frame (reordered)
        assert_eq!(mapper.display_to_decode_idx(4), Some(4)); // P-frame

        // Invalid indices
        assert_eq!(mapper.display_to_decode_idx(5), None);
    }

    #[test]
    fn test_decode_to_display_mapping() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // Verify reverse mapping
        assert_eq!(mapper.decode_to_display_idx(0), Some(0)); // I-frame
        assert_eq!(mapper.decode_to_display_idx(1), Some(3)); // P-frame (reordered)
        assert_eq!(mapper.decode_to_display_idx(2), Some(1)); // B-frame (reordered)
        assert_eq!(mapper.decode_to_display_idx(3), Some(2)); // B-frame (reordered)
        assert_eq!(mapper.decode_to_display_idx(4), Some(4)); // P-frame

        // Invalid indices
        assert_eq!(mapper.decode_to_display_idx(5), None);
    }

    #[test]
    fn test_roundtrip_display_decode_mapping() {
        let frames = create_test_frames();
        let mapper = FrameMapper::new(frames);

        // Verify roundtrip: display -> decode -> display
        for display_idx in 0..mapper.count() {
            let decode_idx = mapper.display_to_decode_idx(display_idx).unwrap();
            let recovered = mapper.decode_to_display_idx(decode_idx).unwrap();
            assert_eq!(recovered, display_idx);
        }
    }
}
