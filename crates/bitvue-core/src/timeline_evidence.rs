//! Timeline Evidence Integration - T4-1 Evidence Chain
//!
//! Deliverable: evidence_chain_01_bit_offset:Foundations:Timeline:AV1:evidence_chain
//!
//! Integrates the evidence chain system with timeline elements to enable:
//! - Tracing timeline frames back to bit offsets in the bitstream
//! - Linking timeline markers to syntax elements
//! - Bidirectional navigation: timeline ↔ bit offset

use crate::timeline::TimelineFrame;
use crate::{
    BitOffsetEvidence, BitRange, DecodeArtifactType, DecodeEvidence, EvidenceChain, EvidenceId,
    FrameMarker, SyntaxEvidence, SyntaxNodeType, VizElementType, VizEvidence,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Timeline evidence manager
///
/// Links timeline visualization elements to their source bit offsets through
/// the evidence chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvidenceManager {
    /// Evidence chain for bidirectional linking
    evidence_chain: EvidenceChain,

    /// Mapping from display_idx to evidence IDs
    frame_evidence_map: HashMap<usize, TimelineFrameEvidence>,

    /// Next evidence ID counter
    next_evidence_id: u64,
}

/// Evidence bundle for a timeline frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineFrameEvidence {
    /// Display index
    pub display_idx: usize,

    /// Bit offset evidence ID (where frame data starts)
    pub bit_offset_id: EvidenceId,

    /// Syntax evidence ID (frame header OBU)
    pub syntax_id: EvidenceId,

    /// Decode evidence ID (decoded frame)
    pub decode_id: EvidenceId,

    /// Timeline viz evidence ID
    pub timeline_viz_id: EvidenceId,
}

impl TimelineEvidenceManager {
    /// Create a new timeline evidence manager
    pub fn new(evidence_chain: EvidenceChain) -> Self {
        Self {
            evidence_chain,
            frame_evidence_map: HashMap::new(),
            next_evidence_id: 0,
        }
    }

    /// Generate a new unique evidence ID
    fn next_id(&mut self) -> EvidenceId {
        let id = format!("timeline_ev_{}", self.next_evidence_id);
        self.next_evidence_id += 1;
        id
    }

    /// Create evidence chain for a timeline frame
    ///
    /// Creates the full evidence chain:
    /// bit_offset → syntax → decode → viz
    pub fn create_frame_evidence(
        &mut self,
        timeline_frame: &TimelineFrame,
        bit_range: BitRange,
        frame_size_bytes: usize,
    ) -> TimelineFrameEvidence {
        let display_idx = timeline_frame.display_idx;

        // Stage 01: Bit offset evidence
        let bit_offset_id = self.next_id();
        let mut bit_offset_ev = BitOffsetEvidence::new(
            bit_offset_id.clone(),
            bit_range,
            format!("timeline_frame_{}", display_idx),
        );

        // Stage 02: Syntax evidence (frame header)
        let syntax_id = self.next_id();
        let mut syntax_ev = SyntaxEvidence::new(
            syntax_id.clone(),
            SyntaxNodeType::FrameHeader,
            format!("frame_{}_header", display_idx),
            bit_range,
            bit_offset_id.clone(),
        );
        syntax_ev.add_metadata("frame_type".to_string(), timeline_frame.frame_type.clone());
        syntax_ev.add_metadata("size_bytes".to_string(), frame_size_bytes.to_string());
        syntax_ev.add_metadata(
            "is_keyframe".to_string(),
            (timeline_frame.marker == FrameMarker::Key).to_string(),
        );
        syntax_ev.add_metadata("display_idx".to_string(), display_idx.to_string());
        if let Some(pts) = timeline_frame.pts {
            syntax_ev.add_metadata("pts".to_string(), pts.to_string());
        }
        if let Some(dts) = timeline_frame.dts {
            syntax_ev.add_metadata("dts".to_string(), dts.to_string());
        }

        // Stage 03: Decode evidence (decoded frame)
        let decode_id = self.next_id();
        let mut decode_ev = DecodeEvidence::new(
            decode_id.clone(),
            DecodeArtifactType::YuvFrame,
            format!("frame_{}_yuv", display_idx),
            syntax_id.clone(),
        );
        decode_ev.set_frame_indices(display_idx, display_idx);
        decode_ev.add_metadata("size_bytes".to_string(), frame_size_bytes.to_string());

        // Stage 04: Viz evidence (timeline track)
        let timeline_viz_id = self.next_id();
        let mut viz_ev = VizEvidence::new(
            timeline_viz_id.clone(),
            VizElementType::TimelineLane,
            format!("timeline_frame_{}", display_idx),
            decode_id.clone(),
        );
        viz_ev.set_frame_indices(display_idx, display_idx);

        // Set temporal position (normalize to 0..1 range, will be set properly by caller)
        viz_ev.set_temporal_pos(display_idx as f64);

        // Add visual properties
        viz_ev.add_visual_property("frame_type".to_string(), timeline_frame.frame_type.clone());
        viz_ev.add_visual_property("marker".to_string(), format!("{:?}", timeline_frame.marker));
        viz_ev.add_visual_property(
            "is_keyframe".to_string(),
            (timeline_frame.marker == FrameMarker::Key).to_string(),
        );
        viz_ev.add_visual_property(
            "size_bytes".to_string(),
            timeline_frame.size_bytes.to_string(),
        );
        viz_ev.add_visual_property("display_idx".to_string(), display_idx.to_string());

        // Add metadata
        viz_ev.add_metadata("display_idx".to_string(), display_idx.to_string());

        // UX Core: Establish bidirectional links for evidence chain navigation
        bit_offset_ev.link_syntax(syntax_id.clone());
        syntax_ev.link_decode(decode_id.clone());
        decode_ev.link_viz(timeline_viz_id.clone());

        // Add all evidence to the chain
        self.evidence_chain.add_bit_offset(bit_offset_ev);
        self.evidence_chain.add_syntax(syntax_ev);
        self.evidence_chain.add_decode(decode_ev);
        self.evidence_chain.add_viz(viz_ev);

        // Create evidence bundle
        let frame_evidence = TimelineFrameEvidence {
            display_idx,
            bit_offset_id,
            syntax_id,
            decode_id,
            timeline_viz_id,
        };

        // Store in map
        self.frame_evidence_map
            .insert(display_idx, frame_evidence.clone());

        frame_evidence
    }

    /// Get evidence for a timeline frame
    pub fn get_frame_evidence(&self, display_idx: usize) -> Option<&TimelineFrameEvidence> {
        self.frame_evidence_map.get(&display_idx)
    }

    /// Get bit range for a timeline frame
    ///
    /// Traverses: timeline → bit_offset
    pub fn get_frame_bit_range(&self, display_idx: usize) -> Option<BitRange> {
        let evidence = self.frame_evidence_map.get(&display_idx)?;
        let bit_offset_ev = self
            .evidence_chain
            .bit_offset_index
            .find_by_id(&evidence.bit_offset_id)?;
        Some(bit_offset_ev.bit_range)
    }

    /// Find timeline frame at temporal position
    ///
    /// Returns the closest frame to the given temporal position (0..1 normalized)
    pub fn find_frame_at_temporal_pos(
        &self,
        temporal_pos: f64,
        total_frames: usize,
    ) -> Option<usize> {
        if total_frames == 0 {
            return None;
        }

        // Convert normalized position to frame index
        let frame_idx = (temporal_pos * (total_frames - 1) as f64).round() as usize;
        Some(frame_idx.min(total_frames - 1))
    }

    /// Find all frames with a specific marker type
    pub fn find_frames_with_marker(&self, marker: FrameMarker) -> Vec<usize> {
        self.frame_evidence_map
            .iter()
            .filter_map(|(display_idx, evidence)| {
                // Check viz evidence for marker
                self.evidence_chain
                    .viz_index
                    .find_by_id(&evidence.timeline_viz_id)
                    .and_then(|viz_ev| {
                        if viz_ev
                            .visual_properties
                            .get("marker")?
                            .contains(&format!("{:?}", marker))
                        {
                            Some(*display_idx)
                        } else {
                            None
                        }
                    })
            })
            .collect()
    }

    /// Get keyframe indices
    pub fn get_keyframe_indices(&self) -> Vec<usize> {
        self.find_frames_with_marker(FrameMarker::Key)
    }

    /// Get error frame indices
    pub fn get_error_frame_indices(&self) -> Vec<usize> {
        self.find_frames_with_marker(FrameMarker::Error)
    }

    /// Get bookmark frame indices
    pub fn get_bookmark_frame_indices(&self) -> Vec<usize> {
        self.find_frames_with_marker(FrameMarker::Bookmark)
    }

    /// Get the evidence chain
    pub fn evidence_chain(&self) -> &EvidenceChain {
        &self.evidence_chain
    }

    /// Get the evidence chain (mutable)
    pub fn evidence_chain_mut(&mut self) -> &mut EvidenceChain {
        &mut self.evidence_chain
    }

    /// Trace from display_idx to bit offset (UX Core navigation)
    ///
    /// UX Core: Enables "click on timeline → jump to hex view" workflows
    pub fn trace_to_bit_offset(&self, display_idx: usize) -> Option<BitRange> {
        self.get_frame_bit_range(display_idx)
    }

    /// Trace from byte offset to display_idx (UX Core navigation)
    ///
    /// UX Core: Enables "click on hex view → jump to timeline" workflows
    pub fn trace_to_display_idx(&self, byte_offset: u64) -> Option<usize> {
        // Search all frame evidence to find which frame contains this byte offset
        for (display_idx, evidence) in &self.frame_evidence_map {
            if let Some(bit_offset_ev) = self
                .evidence_chain
                .bit_offset_index
                .find_by_id(&evidence.bit_offset_id)
            {
                let bit_range = &bit_offset_ev.bit_range;
                if byte_offset >= bit_range.byte_offset()
                    && byte_offset < bit_range.byte_offset() + (bit_range.size_bits() / 8)
                {
                    return Some(*display_idx);
                }
            }
        }
        None
    }

    /// Clear all evidence
    pub fn clear(&mut self) {
        self.frame_evidence_map.clear();
        self.next_evidence_id = 0;
    }

    /// Get total number of frames with evidence
    pub fn frame_count(&self) -> usize {
        self.frame_evidence_map.len()
    }
}

impl Default for TimelineEvidenceManager {
    fn default() -> Self {
        Self::new(EvidenceChain::new())
    }
}

#[cfg(test)]
mod tests {
    include!("timeline_evidence_test.rs");
}
