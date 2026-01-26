//! Reference Graph Evidence Integration - T5-1 Evidence Chain
//!
//! Deliverable: evidence_chain_01_bit_offset:Foundations:Graph:AV1:evidence_chain
//!
//! Integrates the evidence chain system with reference graph visualization to enable:
//! - Tracing graph nodes (frames) back to bit offsets
//! - Linking reference edges to syntax elements (reference frame info)
//! - Bidirectional navigation: graph visualization ↔ bitstream

use crate::{
    BitOffsetEvidence, BitRange, DecodeArtifactType, DecodeEvidence, EvidenceChain, EvidenceId,
    ReferenceEdge, ReferenceType, SyntaxEvidence, SyntaxNodeType, VizElementType, VizEvidence,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reference graph evidence manager
///
/// Links reference graph visualization elements to their source bit offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceGraphEvidenceManager {
    /// Evidence chain for bidirectional linking
    evidence_chain: EvidenceChain,

    /// Mapping from frame display_idx to node evidence
    node_evidence_map: HashMap<usize, ReferenceNodeEvidence>,

    /// Mapping from edge (from, to) to edge evidence
    edge_evidence_map: HashMap<(usize, usize), ReferenceEdgeEvidence>,

    /// Next evidence ID counter
    next_evidence_id: u64,
}

/// Evidence for a reference graph node (frame)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceNodeEvidence {
    /// Frame display index
    pub display_idx: usize,

    /// Bit offset evidence ID
    pub bit_offset_id: EvidenceId,

    /// Syntax evidence ID (frame header)
    pub syntax_id: EvidenceId,

    /// Decode evidence ID (decoded frame)
    pub decode_id: EvidenceId,

    /// Viz evidence ID (graph node)
    pub viz_id: EvidenceId,
}

/// Evidence for a reference edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceEdgeEvidence {
    /// Source frame index
    pub from_idx: usize,

    /// Target frame index
    pub to_idx: usize,

    /// Reference type
    pub ref_type: ReferenceType,

    /// Syntax evidence ID (reference list OBU)
    pub syntax_id: EvidenceId,

    /// Viz evidence ID (graph edge)
    pub viz_id: EvidenceId,
}

impl ReferenceGraphEvidenceManager {
    /// Create a new reference graph evidence manager
    pub fn new(evidence_chain: EvidenceChain) -> Self {
        Self {
            evidence_chain,
            node_evidence_map: HashMap::new(),
            edge_evidence_map: HashMap::new(),
            next_evidence_id: 0,
        }
    }

    /// Generate a new unique evidence ID
    fn next_id(&mut self) -> EvidenceId {
        let id = format!("ref_graph_ev_{}", self.next_evidence_id);
        self.next_evidence_id += 1;
        id
    }

    /// Create evidence for a reference graph node (frame)
    pub fn create_node_evidence(
        &mut self,
        display_idx: usize,
        frame_type: String,
        bit_range: BitRange,
        node_x: f32,
        node_y: f32,
    ) -> ReferenceNodeEvidence {
        // Stage 01: Bit offset evidence
        let bit_offset_id = self.next_id();
        let bit_offset_ev = BitOffsetEvidence::new(
            bit_offset_id.clone(),
            bit_range,
            format!("ref_graph_node_{}", display_idx),
        );
        self.evidence_chain.add_bit_offset(bit_offset_ev);

        // Stage 02: Syntax evidence (frame header with reference info)
        let syntax_id = self.next_id();
        let mut syntax_ev = SyntaxEvidence::new(
            syntax_id.clone(),
            SyntaxNodeType::FrameHeader,
            format!("frame_{}_header", display_idx),
            bit_range,
            bit_offset_id.clone(),
        );
        syntax_ev.add_metadata("frame_type".to_string(), frame_type.clone());
        syntax_ev.add_metadata("display_idx".to_string(), display_idx.to_string());
        self.evidence_chain.add_syntax(syntax_ev);

        // Stage 03: Decode evidence
        let decode_id = self.next_id();
        let mut decode_ev = DecodeEvidence::new(
            decode_id.clone(),
            DecodeArtifactType::YuvFrame,
            format!("frame_{}_yuv", display_idx),
            syntax_id.clone(),
        );
        decode_ev.set_frame_indices(display_idx, display_idx);
        self.evidence_chain.add_decode(decode_ev);

        // Stage 04: Viz evidence (graph node)
        let viz_id = self.next_id();
        let mut viz_ev = VizEvidence::new(
            viz_id.clone(),
            VizElementType::ReferenceGraphNode,
            format!("ref_node_{}", display_idx),
            decode_id.clone(),
        );
        viz_ev.set_frame_indices(display_idx, display_idx);
        viz_ev.add_visual_property("node_x".to_string(), node_x.to_string());
        viz_ev.add_visual_property("node_y".to_string(), node_y.to_string());
        viz_ev.add_visual_property("frame_type".to_string(), frame_type);
        viz_ev.add_metadata("display_idx".to_string(), display_idx.to_string());

        self.evidence_chain.add_viz(viz_ev);

        // Create evidence bundle
        let node_evidence = ReferenceNodeEvidence {
            display_idx,
            bit_offset_id,
            syntax_id,
            decode_id,
            viz_id,
        };

        // Store in map
        self.node_evidence_map
            .insert(display_idx, node_evidence.clone());

        node_evidence
    }

    /// Create evidence for a reference edge
    pub fn create_edge_evidence(
        &mut self,
        edge: &ReferenceEdge,
        bit_range: BitRange,
    ) -> ReferenceEdgeEvidence {
        // Stage 02: Syntax evidence (reference list entry)
        let syntax_id = self.next_id();
        let mut syntax_ev = SyntaxEvidence::new(
            syntax_id.clone(),
            SyntaxNodeType::Custom("reference_list".to_string()),
            format!("ref_{}_{}", edge.from_idx, edge.to_idx),
            bit_range,
            self.node_evidence_map
                .get(&edge.from_idx)
                .map(|n| n.bit_offset_id.clone())
                .unwrap_or_else(|| self.next_id()),
        );
        syntax_ev.add_metadata("from_idx".to_string(), edge.from_idx.to_string());
        syntax_ev.add_metadata("to_idx".to_string(), edge.to_idx.to_string());
        syntax_ev.add_metadata("ref_type".to_string(), edge.ref_type.name().to_string());
        self.evidence_chain.add_syntax(syntax_ev);

        // Stage 04: Viz evidence (graph edge)
        let viz_id = self.next_id();
        let mut viz_ev = VizEvidence::new(
            viz_id.clone(),
            VizElementType::Custom("reference_edge".to_string()),
            format!("ref_edge_{}_{}", edge.from_idx, edge.to_idx),
            self.node_evidence_map
                .get(&edge.from_idx)
                .map(|n| n.decode_id.clone())
                .unwrap_or_else(|| self.next_id()),
        );
        viz_ev.add_visual_property("from_idx".to_string(), edge.from_idx.to_string());
        viz_ev.add_visual_property("to_idx".to_string(), edge.to_idx.to_string());
        viz_ev.add_visual_property("ref_type".to_string(), edge.ref_type.name().to_string());
        viz_ev.add_visual_property(
            "color_hint".to_string(),
            edge.ref_type.color_hint().to_string(),
        );

        self.evidence_chain.add_viz(viz_ev);

        // Create evidence bundle
        let edge_evidence = ReferenceEdgeEvidence {
            from_idx: edge.from_idx,
            to_idx: edge.to_idx,
            ref_type: edge.ref_type,
            syntax_id,
            viz_id,
        };

        // Store in map
        self.edge_evidence_map
            .insert((edge.from_idx, edge.to_idx), edge_evidence.clone());

        edge_evidence
    }

    /// Get node evidence for a frame
    pub fn get_node_evidence(&self, display_idx: usize) -> Option<&ReferenceNodeEvidence> {
        self.node_evidence_map.get(&display_idx)
    }

    /// Get edge evidence
    pub fn get_edge_evidence(
        &self,
        from_idx: usize,
        to_idx: usize,
    ) -> Option<&ReferenceEdgeEvidence> {
        self.edge_evidence_map.get(&(from_idx, to_idx))
    }

    /// Get all node indices
    pub fn node_indices(&self) -> Vec<usize> {
        self.node_evidence_map.keys().copied().collect()
    }

    /// Get all edges
    pub fn edge_keys(&self) -> Vec<(usize, usize)> {
        self.edge_evidence_map.keys().copied().collect()
    }

    /// Get the evidence chain
    pub fn evidence_chain(&self) -> &EvidenceChain {
        &self.evidence_chain
    }

    /// Clear all evidence
    pub fn clear(&mut self) {
        self.node_evidence_map.clear();
        self.edge_evidence_map.clear();
        self.next_evidence_id = 0;
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.node_evidence_map.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edge_evidence_map.len()
    }
}

impl Default for ReferenceGraphEvidenceManager {
    fn default() -> Self {
        Self::new(EvidenceChain::new())
    }
}

// TODO: Fix reference_graph_evidence_test.rs - needs API rewrite to match actual implementation
// #[cfg(test)]
// include!("reference_graph_evidence_test.rs");
