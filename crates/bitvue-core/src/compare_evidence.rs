//! Compare Evidence Integration - T6-2 Evidence Chain
//!
//! Deliverable: evidence_chain_01_bit_offset:Foundations:Compare:AV1:evidence_chain
//!
//! Integrates the evidence chain system with A/B compare view to enable:
//! - Tracing compared frame pairs back to their bit offsets
//! - Linking diff heatmap pixels to both source bitstreams
//! - Bidirectional navigation: compare visualization ↔ bitstreams

use crate::{
    BitOffsetEvidence, BitRange, DecodeArtifactType, DecodeEvidence, EvidenceChain, EvidenceId,
    FramePair, SyntaxEvidence, SyntaxNodeType, VizElementType, VizEvidence,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compare evidence manager
///
/// Links compare visualization elements to their source bit offsets in both streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareEvidenceManager {
    /// Evidence chain for stream A
    evidence_chain_a: EvidenceChain,

    /// Evidence chain for stream B
    evidence_chain_b: EvidenceChain,

    /// Mapping from aligned pair to compare evidence
    pair_evidence_map: HashMap<(usize, usize), ComparePairEvidence>,

    /// Next evidence ID counter
    next_evidence_id: u64,
}

/// Evidence for a compared frame pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparePairEvidence {
    /// Frame A display index
    pub frame_a_idx: usize,

    /// Frame B display index
    pub frame_b_idx: usize,

    /// Evidence IDs for stream A
    pub stream_a: StreamEvidence,

    /// Evidence IDs for stream B
    pub stream_b: StreamEvidence,

    /// Viz evidence ID for the diff heatmap
    pub diff_viz_id: EvidenceId,
}

/// Evidence IDs for a single stream in compare
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvidence {
    /// Bit offset evidence ID
    pub bit_offset_id: EvidenceId,

    /// Syntax evidence ID
    pub syntax_id: EvidenceId,

    /// Decode evidence ID
    pub decode_id: EvidenceId,
}

impl CompareEvidenceManager {
    /// Create a new compare evidence manager
    pub fn new(evidence_chain_a: EvidenceChain, evidence_chain_b: EvidenceChain) -> Self {
        Self {
            evidence_chain_a,
            evidence_chain_b,
            pair_evidence_map: HashMap::new(),
            next_evidence_id: 0,
        }
    }

    /// Generate a new unique evidence ID
    fn next_id(&mut self) -> EvidenceId {
        let id = format!("compare_ev_{}", self.next_evidence_id);
        self.next_evidence_id += 1;
        id
    }

    /// Create evidence for a compared frame pair
    pub fn create_pair_evidence(
        &mut self,
        pair: &FramePair,
        bit_range_a: BitRange,
        bit_range_b: BitRange,
        frame_size_a: usize,
        frame_size_b: usize,
    ) -> Option<ComparePairEvidence> {
        // Skip pairs with gaps
        let frame_a = pair.stream_a_idx?;
        let frame_b = pair.stream_b_idx?;

        // Create evidence for stream A
        let bit_offset_id_a = self.next_id();
        let syntax_id_a = self.next_id();
        let decode_id_a = self.next_id();

        // Bit offset A
        self.evidence_chain_a.add_bit_offset(BitOffsetEvidence::new(
            bit_offset_id_a.clone(),
            bit_range_a,
            format!("compare_A_{}", frame_a),
        ));

        // Syntax A
        let mut syntax_ev_a = SyntaxEvidence::new(
            syntax_id_a.clone(),
            SyntaxNodeType::FrameHeader,
            format!("frame_{}_header_A", frame_a),
            bit_range_a,
            bit_offset_id_a.clone(),
        );
        syntax_ev_a.add_metadata("stream".to_string(), "A".to_string());
        syntax_ev_a.add_metadata("frame_size".to_string(), frame_size_a.to_string());
        self.evidence_chain_a.add_syntax(syntax_ev_a);

        // Decode A
        let mut decode_ev_a = DecodeEvidence::new(
            decode_id_a.clone(),
            DecodeArtifactType::YuvFrame,
            format!("frame_{}_yuv_A", frame_a),
            syntax_id_a.clone(),
        );
        decode_ev_a.set_frame_indices(frame_a, frame_a);
        decode_ev_a.add_metadata("stream".to_string(), "A".to_string());
        self.evidence_chain_a.add_decode(decode_ev_a);

        let stream_a = StreamEvidence {
            bit_offset_id: bit_offset_id_a,
            syntax_id: syntax_id_a,
            decode_id: decode_id_a,
        };

        // Create evidence for stream B
        let bit_offset_id_b = self.next_id();
        let syntax_id_b = self.next_id();
        let decode_id_b = self.next_id();

        // Bit offset B
        self.evidence_chain_b.add_bit_offset(BitOffsetEvidence::new(
            bit_offset_id_b.clone(),
            bit_range_b,
            format!("compare_B_{}", frame_b),
        ));

        // Syntax B
        let mut syntax_ev_b = SyntaxEvidence::new(
            syntax_id_b.clone(),
            SyntaxNodeType::FrameHeader,
            format!("frame_{}_header_B", frame_b),
            bit_range_b,
            bit_offset_id_b.clone(),
        );
        syntax_ev_b.add_metadata("stream".to_string(), "B".to_string());
        syntax_ev_b.add_metadata("frame_size".to_string(), frame_size_b.to_string());
        self.evidence_chain_b.add_syntax(syntax_ev_b);

        // Decode B
        let mut decode_ev_b = DecodeEvidence::new(
            decode_id_b.clone(),
            DecodeArtifactType::YuvFrame,
            format!("frame_{}_yuv_B", frame_b),
            syntax_id_b.clone(),
        );
        decode_ev_b.set_frame_indices(frame_b, frame_b);
        decode_ev_b.add_metadata("stream".to_string(), "B".to_string());
        self.evidence_chain_b.add_decode(decode_ev_b);

        let stream_b = StreamEvidence {
            bit_offset_id: bit_offset_id_b,
            syntax_id: syntax_id_b,
            decode_id: decode_id_b,
        };

        // Create diff heatmap viz evidence (links to both decode artifacts)
        let diff_viz_id = self.next_id();
        let mut diff_viz = VizEvidence::new(
            diff_viz_id.clone(),
            VizElementType::DiffHeatmap,
            format!("diff_{}_{}", frame_a, frame_b),
            stream_a.decode_id.clone(), // Primary link to stream A
        );

        diff_viz.add_visual_property("frame_a_idx".to_string(), frame_a.to_string());
        diff_viz.add_visual_property("frame_b_idx".to_string(), frame_b.to_string());
        if let Some(delta) = pair.pts_delta {
            diff_viz.add_visual_property("pts_delta".to_string(), delta.to_string());
        }
        diff_viz.add_metadata("stream_a_decode".to_string(), stream_a.decode_id.clone());
        diff_viz.add_metadata("stream_b_decode".to_string(), stream_b.decode_id.clone());

        // Add to stream A evidence chain (primary)
        self.evidence_chain_a.add_viz(diff_viz);

        // Create evidence bundle
        let pair_evidence = ComparePairEvidence {
            frame_a_idx: frame_a,
            frame_b_idx: frame_b,
            stream_a,
            stream_b,
            diff_viz_id,
        };

        // Store in map
        self.pair_evidence_map
            .insert((frame_a, frame_b), pair_evidence.clone());

        Some(pair_evidence)
    }

    /// Create evidence for a single stream in the pair
    #[allow(dead_code)]
    fn create_stream_evidence(
        &mut self,
        frame_idx: usize,
        bit_range: BitRange,
        frame_size: usize,
        stream_label: &str,
        evidence_chain: &mut EvidenceChain,
    ) -> StreamEvidence {
        // Stage 01: Bit offset evidence
        let bit_offset_id = self.next_id();
        let bit_offset_ev = BitOffsetEvidence::new(
            bit_offset_id.clone(),
            bit_range,
            format!("compare_{}_{}", stream_label, frame_idx),
        );
        evidence_chain.add_bit_offset(bit_offset_ev);

        // Stage 02: Syntax evidence
        let syntax_id = self.next_id();
        let mut syntax_ev = SyntaxEvidence::new(
            syntax_id.clone(),
            SyntaxNodeType::FrameHeader,
            format!("frame_{}_header_{}", frame_idx, stream_label),
            bit_range,
            bit_offset_id.clone(),
        );
        syntax_ev.add_metadata("stream".to_string(), stream_label.to_string());
        syntax_ev.add_metadata("frame_size".to_string(), frame_size.to_string());
        evidence_chain.add_syntax(syntax_ev);

        // Stage 03: Decode evidence
        let decode_id = self.next_id();
        let mut decode_ev = DecodeEvidence::new(
            decode_id.clone(),
            DecodeArtifactType::YuvFrame,
            format!("frame_{}_yuv_{}", frame_idx, stream_label),
            syntax_id.clone(),
        );
        decode_ev.set_frame_indices(frame_idx, frame_idx);
        decode_ev.add_metadata("stream".to_string(), stream_label.to_string());
        evidence_chain.add_decode(decode_ev);

        StreamEvidence {
            bit_offset_id,
            syntax_id,
            decode_id,
        }
    }

    /// Get evidence for a frame pair
    pub fn get_pair_evidence(
        &self,
        frame_a_idx: usize,
        frame_b_idx: usize,
    ) -> Option<&ComparePairEvidence> {
        self.pair_evidence_map.get(&(frame_a_idx, frame_b_idx))
    }

    /// Find all pairs involving a specific frame in stream A
    pub fn find_pairs_with_frame_a(&self, frame_a_idx: usize) -> Vec<&ComparePairEvidence> {
        self.pair_evidence_map
            .values()
            .filter(|ev| ev.frame_a_idx == frame_a_idx)
            .collect()
    }

    /// Find all pairs involving a specific frame in stream B
    pub fn find_pairs_with_frame_b(&self, frame_b_idx: usize) -> Vec<&ComparePairEvidence> {
        self.pair_evidence_map
            .values()
            .filter(|ev| ev.frame_b_idx == frame_b_idx)
            .collect()
    }

    /// Get evidence chain for stream A
    pub fn evidence_chain_a(&self) -> &EvidenceChain {
        &self.evidence_chain_a
    }

    /// Get evidence chain for stream B
    pub fn evidence_chain_b(&self) -> &EvidenceChain {
        &self.evidence_chain_b
    }

    /// Get evidence chain for stream A (mutable)
    pub fn evidence_chain_a_mut(&mut self) -> &mut EvidenceChain {
        &mut self.evidence_chain_a
    }

    /// Get evidence chain for stream B (mutable)
    pub fn evidence_chain_b_mut(&mut self) -> &mut EvidenceChain {
        &mut self.evidence_chain_b
    }

    /// Clear all evidence
    pub fn clear(&mut self) {
        self.pair_evidence_map.clear();
        self.next_evidence_id = 0;
    }

    /// Get pair count
    pub fn pair_count(&self) -> usize {
        self.pair_evidence_map.len()
    }
}

impl Default for CompareEvidenceManager {
    fn default() -> Self {
        Self::new(EvidenceChain::new(), EvidenceChain::new())
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
    include!("compare_evidence_test.rs");
}
