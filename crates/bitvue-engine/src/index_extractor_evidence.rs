//! Index Extractor Evidence Integration - T1-1 Evidence Chain
//!
//! Deliverable: evidence_chain_01_bit_offset:Indexing:Core:AV1:evidence_chain
//!
//! Integrates the evidence chain system with index extraction to enable:
//! - Tracing seek points and frame metadata back to bit offsets
//! - Linking indexed frames to OBU syntax elements
//! - Bidirectional navigation: index ↔ bit offset ↔ syntax

use crate::indexing::{FrameMetadata, SeekPoint};
use crate::{
    BitOffsetEvidence, BitRange, EvidenceChain, EvidenceId, SyntaxEvidence, SyntaxNodeType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Index extractor evidence manager
///
/// Links indexed frames and seek points to their source bit offsets through
/// the evidence chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexExtractorEvidenceManager {
    /// Evidence chain for bidirectional linking
    evidence_chain: EvidenceChain,

    /// Mapping from display_idx to evidence IDs
    frame_evidence_map: HashMap<usize, IndexFrameEvidence>,

    /// Mapping from byte_offset to evidence IDs
    offset_evidence_map: HashMap<u64, EvidenceId>,

    /// Next evidence ID counter
    next_evidence_id: u64,
}

/// Evidence bundle for an indexed frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexFrameEvidence {
    /// Display index
    pub display_idx: usize,

    /// Bit offset evidence ID (where frame data starts)
    pub bit_offset_id: EvidenceId,

    /// Syntax evidence ID (OBU/frame header)
    pub syntax_id: EvidenceId,
}

impl IndexExtractorEvidenceManager {
    /// Create a new index extractor evidence manager
    pub fn new(evidence_chain: EvidenceChain) -> Self {
        Self {
            evidence_chain,
            frame_evidence_map: HashMap::new(),
            offset_evidence_map: HashMap::new(),
            next_evidence_id: 0,
        }
    }

    /// Create evidence manager with empty evidence chain
    pub fn new_empty() -> Self {
        Self::new(EvidenceChain::new())
    }

    /// Generate a new unique evidence ID
    fn next_id(&mut self) -> EvidenceId {
        let id = format!("index_ev_{}", self.next_evidence_id);
        self.next_evidence_id += 1;
        id
    }

    /// Create evidence chain for a seek point
    ///
    /// Creates the evidence chain for a keyframe seek point:
    /// bit_offset → syntax (OBU)
    pub fn create_seekpoint_evidence(&mut self, seek_point: &SeekPoint) -> IndexFrameEvidence {
        let display_idx = seek_point.display_idx;
        let byte_offset = seek_point.byte_offset;

        // Estimate size (we don't know exact size for seek points)
        // Use a placeholder size
        let estimated_size = 1024; // Will be updated when full frame metadata is available

        // Stage 01: Bit offset evidence
        let bit_offset_id = self.next_id();
        let start_bit = byte_offset * 8;
        let end_bit = (byte_offset + estimated_size as u64) * 8;
        let bit_range = BitRange::new(start_bit, end_bit);
        let mut bit_offset_ev = BitOffsetEvidence::new(
            bit_offset_id.clone(),
            bit_range,
            format!("seekpoint_{}", display_idx),
        );

        // Stage 02: Syntax evidence (keyframe OBU)
        let syntax_id = self.next_id();
        let mut syntax_ev = SyntaxEvidence::new(
            syntax_id.clone(),
            SyntaxNodeType::FrameHeader,
            format!("keyframe_{}", display_idx),
            bit_range,
            bit_offset_id.clone(),
        );
        syntax_ev.add_metadata("is_keyframe".to_string(), "true".to_string());
        syntax_ev.add_metadata("display_idx".to_string(), display_idx.to_string());
        if let Some(pts) = seek_point.pts {
            syntax_ev.add_metadata("pts".to_string(), pts.to_string());
        }

        // Link bit offset to syntax
        bit_offset_ev.link_syntax(syntax_id.clone());

        self.evidence_chain.add_bit_offset(bit_offset_ev);
        self.evidence_chain.add_syntax(syntax_ev);

        let evidence = IndexFrameEvidence {
            display_idx,
            bit_offset_id: bit_offset_id.clone(),
            syntax_id,
        };

        // Store mappings
        self.frame_evidence_map
            .insert(display_idx, evidence.clone());
        self.offset_evidence_map.insert(byte_offset, bit_offset_id);

        evidence
    }

    /// Create evidence chain for a full frame metadata entry
    ///
    /// Creates the complete evidence chain with accurate size information:
    /// bit_offset → syntax (OBU with frame type)
    pub fn create_frame_metadata_evidence(&mut self, frame: &FrameMetadata) -> IndexFrameEvidence {
        let display_idx = frame.display_idx;
        let byte_offset = frame.byte_offset;
        let size_bytes = frame.size as usize;

        // Stage 01: Bit offset evidence
        let bit_offset_id = self.next_id();
        let start_bit = byte_offset * 8;
        let end_bit = (byte_offset + size_bytes as u64) * 8;
        let bit_range = BitRange::new(start_bit, end_bit);
        let mut bit_offset_ev = BitOffsetEvidence::new(
            bit_offset_id.clone(),
            bit_range,
            format!("frame_{}", display_idx),
        );

        // Stage 02: Syntax evidence (frame OBU)
        let syntax_id = self.next_id();
        // Both keyframes and inter frames use FrameHeader in AV1
        let node_type = SyntaxNodeType::FrameHeader;

        let mut syntax_ev = SyntaxEvidence::new(
            syntax_id.clone(),
            node_type,
            format!(
                "frame_{}_{}",
                display_idx,
                if frame.is_keyframe { "I" } else { "P" }
            ),
            bit_range,
            bit_offset_id.clone(),
        );

        // Add metadata
        syntax_ev.add_metadata("is_keyframe".to_string(), frame.is_keyframe.to_string());
        syntax_ev.add_metadata("display_idx".to_string(), display_idx.to_string());
        syntax_ev.add_metadata("decode_idx".to_string(), frame.decode_idx.to_string());
        syntax_ev.add_metadata("size_bytes".to_string(), size_bytes.to_string());

        if let Some(pts) = frame.pts {
            syntax_ev.add_metadata("pts".to_string(), pts.to_string());
        }
        if let Some(dts) = frame.dts {
            syntax_ev.add_metadata("dts".to_string(), dts.to_string());
        }
        if let Some(ref frame_type) = frame.frame_type {
            syntax_ev.add_metadata("frame_type".to_string(), frame_type.clone());
        }

        // Link bit offset to syntax
        bit_offset_ev.link_syntax(syntax_id.clone());

        self.evidence_chain.add_bit_offset(bit_offset_ev);
        self.evidence_chain.add_syntax(syntax_ev);

        let evidence = IndexFrameEvidence {
            display_idx,
            bit_offset_id: bit_offset_id.clone(),
            syntax_id,
        };

        // Store mappings
        self.frame_evidence_map
            .insert(display_idx, evidence.clone());
        self.offset_evidence_map.insert(byte_offset, bit_offset_id);

        evidence
    }

    /// Update existing seek point evidence with accurate size from full metadata
    ///
    /// When full indexing completes, we can update the estimated sizes by creating new evidence
    pub fn update_seekpoint_size(&mut self, display_idx: usize, actual_size: usize) {
        if let Some(old_evidence) = self.frame_evidence_map.get(&display_idx).cloned() {
            // Get the byte offset from old evidence
            if let Some(bit_ev) = self
                .evidence_chain
                .bit_offset_index
                .find_by_id(&old_evidence.bit_offset_id)
            {
                let byte_offset = bit_ev.byte_offset;

                // Create new bit offset evidence with accurate size
                let new_bit_offset_id = self.next_id();
                let start_bit = byte_offset * 8;
                let end_bit = (byte_offset + actual_size as u64) * 8;
                let new_bit_range = BitRange::new(start_bit, end_bit);
                let mut new_bit_offset_ev = BitOffsetEvidence::new(
                    new_bit_offset_id.clone(),
                    new_bit_range,
                    format!("seekpoint_{}_updated", display_idx),
                );

                // Create new syntax evidence with accurate size
                let new_syntax_id = self.next_id();
                let mut new_syntax_ev = SyntaxEvidence::new(
                    new_syntax_id.clone(),
                    SyntaxNodeType::FrameHeader,
                    format!("keyframe_{}_updated", display_idx),
                    new_bit_range,
                    new_bit_offset_id.clone(),
                );

                // Copy metadata from old syntax evidence
                if let Some(old_syntax_ev) = self
                    .evidence_chain
                    .syntax_index
                    .find_by_id(&old_evidence.syntax_id)
                {
                    for (k, v) in &old_syntax_ev.metadata {
                        new_syntax_ev.add_metadata(k.clone(), v.clone());
                    }
                }
                new_syntax_ev.add_metadata("size_bytes".to_string(), actual_size.to_string());

                // Link bit offset to syntax
                new_bit_offset_ev.link_syntax(new_syntax_id.clone());

                // Add new evidence to chain
                self.evidence_chain.add_bit_offset(new_bit_offset_ev);
                self.evidence_chain.add_syntax(new_syntax_ev);

                // Update mapping to point to new evidence
                let new_evidence = IndexFrameEvidence {
                    display_idx,
                    bit_offset_id: new_bit_offset_id.clone(),
                    syntax_id: new_syntax_id,
                };
                self.frame_evidence_map.insert(display_idx, new_evidence);
                self.offset_evidence_map
                    .insert(byte_offset, new_bit_offset_id);
            }
        }
    }

    /// Get evidence for a frame by display index
    pub fn get_frame_evidence(&self, display_idx: usize) -> Option<&IndexFrameEvidence> {
        self.frame_evidence_map.get(&display_idx)
    }

    /// Get evidence for a frame by byte offset
    pub fn get_evidence_by_offset(&self, byte_offset: u64) -> Option<EvidenceId> {
        self.offset_evidence_map.get(&byte_offset).cloned()
    }

    /// Trace from display index to bit offset
    ///
    /// Returns the bit range for a given display index
    pub fn trace_to_bit_offset(&self, display_idx: usize) -> Option<BitRange> {
        let evidence = self.frame_evidence_map.get(&display_idx)?;
        let bit_ev = self
            .evidence_chain
            .bit_offset_index
            .find_by_id(&evidence.bit_offset_id)?;
        Some(bit_ev.bit_range)
    }

    /// Trace from byte offset to display index
    ///
    /// Returns the display index for a given byte offset
    pub fn trace_to_display_idx(&self, byte_offset: u64) -> Option<usize> {
        // Find bit offset evidence containing this byte
        let bit_offset_bits = byte_offset * 8;
        let bit_ev = self
            .evidence_chain
            .bit_offset_index
            .find_by_bit_offset(bit_offset_bits)?;

        // Find syntax evidence linked to this bit offset
        let syntax_id = bit_ev.syntax_link.as_ref()?;
        let syntax_ev = self.evidence_chain.syntax_index.find_by_id(syntax_id)?;

        // Extract display_idx from metadata
        syntax_ev.metadata.get("display_idx")?.parse().ok()
    }

    /// Get evidence chain (immutable)
    pub fn evidence_chain(&self) -> &EvidenceChain {
        &self.evidence_chain
    }

    /// Get evidence chain (mutable)
    pub fn evidence_chain_mut(&mut self) -> &mut EvidenceChain {
        &mut self.evidence_chain
    }

    /// Get frame evidence count
    pub fn frame_count(&self) -> usize {
        self.frame_evidence_map.len()
    }

    /// Get all frame evidence (sorted by display_idx)
    pub fn all_frame_evidence(&self) -> Vec<&IndexFrameEvidence> {
        let mut entries: Vec<_> = self.frame_evidence_map.values().collect();
        entries.sort_by_key(|e| e.display_idx);
        entries
    }

    /// Clear all evidence
    pub fn clear(&mut self) {
        self.frame_evidence_map.clear();
        self.offset_evidence_map.clear();
        self.evidence_chain = EvidenceChain::new();
        self.next_evidence_id = 0;
    }
}

impl Default for IndexExtractorEvidenceManager {
    fn default() -> Self {
        Self::new_empty()
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
    include!("index_extractor_evidence_test.rs");
}
