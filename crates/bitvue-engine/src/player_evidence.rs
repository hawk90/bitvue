//! Player Overlay Evidence Chain - T0-2
//!
//! Deliverable: evidence_chain_01_bit_offset:Foundations:PlayerOverlay:AV1:evidence_chain
//!
//! Per COORDINATE_SYSTEM_CONTRACT.md:
//! - Canonical pipeline: screen_px → video_rect_norm → coded_px → block_idx
//! - Evidence chain links: screen → coded → block → decode → syntax → bit_offset
//!
//! This module integrates the coordinate transform system with the evidence chain
//! to enable lossless bidirectional linking between player overlay visualization
//! elements and their source bit offsets in the bitstream.

use crate::{
    BlockIdx, CodedPx, CoordinateTransformer, EvidenceChain, EvidenceId, ScreenPx, VizElementType,
    VizEvidence,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Player overlay evidence manager
///
/// Links player overlay visualization elements (screen coordinates) back to
/// their source bit offsets through the evidence chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerEvidenceManager {
    /// Coordinate transformer for screen↔coded↔block mapping
    transformer: CoordinateTransformer,

    /// Evidence chain for bidirectional bit_offset↔syntax↔decode↔viz linking
    evidence_chain: EvidenceChain,

    /// Next evidence ID counter
    next_evidence_id: u64,
}

impl PlayerEvidenceManager {
    /// Create a new player evidence manager
    pub fn new(transformer: CoordinateTransformer, evidence_chain: EvidenceChain) -> Self {
        Self {
            transformer,
            evidence_chain,
            next_evidence_id: 0,
        }
    }

    /// Generate a new unique evidence ID
    fn next_id(&mut self) -> EvidenceId {
        let id = format!("player_viz_{}", self.next_evidence_id);
        self.next_evidence_id += 1;
        id
    }

    /// Update the coordinate transformer (called when zoom/pan changes)
    pub fn update_transformer(&mut self, transformer: CoordinateTransformer) {
        self.transformer = transformer;
    }

    /// Create evidence for a pixel hover event
    ///
    /// Links screen pixel → coded pixel → decode → syntax → bit offset
    pub fn create_pixel_evidence(
        &mut self,
        screen: ScreenPx,
        frame_idx: usize,
        decode_evidence_id: EvidenceId,
    ) -> Option<VizEvidence> {
        // Transform screen → coded
        let coded = self.transformer.screen_to_coded(screen)?;
        let (coded_x, coded_y) = coded.round();

        // Create VizEvidence for pixel
        let viz_evidence = VizEvidence {
            id: self.next_id(),
            element_type: VizElementType::Custom("pixel_hover".to_string()),
            element_label: format!("pixel_{}_{}", coded_x, coded_y),
            frame_idx: Some(frame_idx),
            display_idx: Some(frame_idx),
            decode_link: decode_evidence_id,
            screen_rect: Some((screen.x, screen.y, 1.0, 1.0)),
            coded_rect: Some((coded_x, coded_y, 1, 1)),
            temporal_pos: None,
            visual_properties: {
                let mut props = HashMap::new();
                props.insert("type".to_string(), "hover".to_string());
                props
            },
            metadata: HashMap::new(),
        };

        // Add to evidence chain
        self.evidence_chain.viz_index.add(viz_evidence.clone());

        Some(viz_evidence)
    }

    /// Create evidence for a block hover event
    ///
    /// Links screen pixel → block → decode → syntax → bit offset
    pub fn create_block_evidence(
        &mut self,
        screen: ScreenPx,
        frame_idx: usize,
        block_size: u32,
        decode_evidence_id: EvidenceId,
    ) -> Option<VizEvidence> {
        // Transform screen → coded → block
        let coded = self.transformer.screen_to_coded(screen)?;
        let block = self.transformer.coded_to_block(coded, Some(block_size));

        // Get block bounds in coded space
        let block_coded = self.transformer.block_to_coded(block, Some(block_size));
        let (coded_x, coded_y) = block_coded.round();

        // Get block bounds in screen space
        let block_screen = self.transformer.block_to_screen(block, Some(block_size));
        let block_screen_end = self.transformer.coded_to_screen(CodedPx::new(
            block_coded.x + block_size as f32,
            block_coded.y + block_size as f32,
        ));
        let screen_w = block_screen_end.x - block_screen.x;
        let screen_h = block_screen_end.y - block_screen.y;

        // Create VizEvidence for block
        let viz_evidence = VizEvidence {
            id: self.next_id(),
            element_type: VizElementType::Custom("block_hover".to_string()),
            element_label: format!("block_{}_{}_{}", block.col, block.row, block_size),
            frame_idx: Some(frame_idx),
            display_idx: Some(frame_idx),
            decode_link: decode_evidence_id,
            screen_rect: Some((block_screen.x, block_screen.y, screen_w, screen_h)),
            coded_rect: Some((coded_x, coded_y, block_size, block_size)),
            temporal_pos: None,
            visual_properties: {
                let mut props = HashMap::new();
                props.insert("block_col".to_string(), block.col.to_string());
                props.insert("block_row".to_string(), block.row.to_string());
                props.insert("block_size".to_string(), block_size.to_string());
                props
            },
            metadata: HashMap::new(),
        };

        // Add to evidence chain
        self.evidence_chain.viz_index.add(viz_evidence.clone());

        Some(viz_evidence)
    }

    /// Create evidence for a QP heatmap cell
    ///
    /// Links QP cell → block → decode → syntax → bit offset
    pub fn create_qp_heatmap_evidence(
        &mut self,
        block: BlockIdx,
        qp_value: u8,
        frame_idx: usize,
        block_size: u32,
        decode_evidence_id: EvidenceId,
    ) -> VizEvidence {
        // Get block bounds in coded space
        let block_coded = self.transformer.block_to_coded(block, Some(block_size));
        let (coded_x, coded_y) = block_coded.round();

        // Get block bounds in screen space
        let block_screen = self.transformer.block_to_screen(block, Some(block_size));
        let block_screen_end = self.transformer.coded_to_screen(CodedPx::new(
            block_coded.x + block_size as f32,
            block_coded.y + block_size as f32,
        ));
        let screen_w = block_screen_end.x - block_screen.x;
        let screen_h = block_screen_end.y - block_screen.y;

        // Create VizEvidence for QP heatmap
        let viz_evidence = VizEvidence {
            id: self.next_id(),
            element_type: VizElementType::QpHeatmap,
            element_label: format!("qp_{}_{}_f{}", block.col, block.row, frame_idx),
            frame_idx: Some(frame_idx),
            display_idx: Some(frame_idx),
            decode_link: decode_evidence_id,
            screen_rect: Some((block_screen.x, block_screen.y, screen_w, screen_h)),
            coded_rect: Some((coded_x, coded_y, block_size, block_size)),
            temporal_pos: None,
            visual_properties: {
                let mut props = HashMap::new();
                props.insert("qp_value".to_string(), qp_value.to_string());
                props.insert("block_col".to_string(), block.col.to_string());
                props.insert("block_row".to_string(), block.row.to_string());
                props
            },
            metadata: HashMap::new(),
        };

        // Add to evidence chain
        self.evidence_chain.viz_index.add(viz_evidence.clone());

        viz_evidence
    }

    /// Create evidence for a motion vector overlay
    ///
    /// Links MV overlay → block → decode → syntax → bit offset
    pub fn create_mv_overlay_evidence(
        &mut self,
        block: BlockIdx,
        mv_x: f32,
        mv_y: f32,
        frame_idx: usize,
        block_size: u32,
        decode_evidence_id: EvidenceId,
    ) -> VizEvidence {
        // Get block bounds in coded space
        let block_coded = self.transformer.block_to_coded(block, Some(block_size));
        let (coded_x, coded_y) = block_coded.round();

        // Get block bounds in screen space
        let block_screen = self.transformer.block_to_screen(block, Some(block_size));
        let block_screen_end = self.transformer.coded_to_screen(CodedPx::new(
            block_coded.x + block_size as f32,
            block_coded.y + block_size as f32,
        ));
        let screen_w = block_screen_end.x - block_screen.x;
        let screen_h = block_screen_end.y - block_screen.y;

        // Create VizEvidence for MV overlay
        let viz_evidence = VizEvidence {
            id: self.next_id(),
            element_type: VizElementType::MotionVectorOverlay,
            element_label: format!("mv_{}_{}_f{}", block.col, block.row, frame_idx),
            frame_idx: Some(frame_idx),
            display_idx: Some(frame_idx),
            decode_link: decode_evidence_id,
            screen_rect: Some((block_screen.x, block_screen.y, screen_w, screen_h)),
            coded_rect: Some((coded_x, coded_y, block_size, block_size)),
            temporal_pos: None,
            visual_properties: {
                let mut props = HashMap::new();
                props.insert("mv_x".to_string(), format!("{:.2}", mv_x));
                props.insert("mv_y".to_string(), format!("{:.2}", mv_y));
                props.insert(
                    "mv_magnitude".to_string(),
                    format!("{:.2}", (mv_x * mv_x + mv_y * mv_y).sqrt()),
                );
                props.insert("block_col".to_string(), block.col.to_string());
                props.insert("block_row".to_string(), block.row.to_string());
                props
            },
            metadata: HashMap::new(),
        };

        // Add to evidence chain
        self.evidence_chain.viz_index.add(viz_evidence.clone());

        viz_evidence
    }

    /// Create evidence for a partition grid overlay
    ///
    /// Links partition → block → decode → syntax → bit offset
    pub fn create_partition_evidence(
        &mut self,
        block: BlockIdx,
        partition_type: String,
        frame_idx: usize,
        block_size: u32,
        decode_evidence_id: EvidenceId,
    ) -> VizEvidence {
        // Get block bounds in coded space
        let block_coded = self.transformer.block_to_coded(block, Some(block_size));
        let (coded_x, coded_y) = block_coded.round();

        // Get block bounds in screen space
        let block_screen = self.transformer.block_to_screen(block, Some(block_size));
        let block_screen_end = self.transformer.coded_to_screen(CodedPx::new(
            block_coded.x + block_size as f32,
            block_coded.y + block_size as f32,
        ));
        let screen_w = block_screen_end.x - block_screen.x;
        let screen_h = block_screen_end.y - block_screen.y;

        // Create VizEvidence for partition
        let viz_evidence = VizEvidence {
            id: self.next_id(),
            element_type: VizElementType::PartitionGridOverlay,
            element_label: format!("partition_{}_{}_f{}", block.col, block.row, frame_idx),
            frame_idx: Some(frame_idx),
            display_idx: Some(frame_idx),
            decode_link: decode_evidence_id,
            screen_rect: Some((block_screen.x, block_screen.y, screen_w, screen_h)),
            coded_rect: Some((coded_x, coded_y, block_size, block_size)),
            temporal_pos: None,
            visual_properties: {
                let mut props = HashMap::new();
                props.insert("partition_type".to_string(), partition_type);
                props.insert("block_col".to_string(), block.col.to_string());
                props.insert("block_row".to_string(), block.row.to_string());
                props
            },
            metadata: HashMap::new(),
        };

        // Add to evidence chain
        self.evidence_chain.viz_index.add(viz_evidence.clone());

        viz_evidence
    }

    /// Find VizEvidence at screen coordinates
    ///
    /// Returns all overlapping viz elements at the given screen position
    pub fn find_at_screen(&self, screen: ScreenPx) -> Vec<&VizEvidence> {
        self.evidence_chain
            .viz_index
            .find_at_screen_point(screen.x, screen.y)
    }

    /// Find syntax evidence from screen coordinates
    ///
    /// Traverses: screen → viz → decode → syntax → bit_offset
    pub fn screen_to_syntax(&self, screen: ScreenPx) -> Vec<EvidenceId> {
        let viz_evidences = self.find_at_screen(screen);

        viz_evidences
            .into_iter()
            .filter_map(|viz| {
                self.evidence_chain
                    .decode_index
                    .find_by_id(&viz.decode_link)
                    .map(|decode| decode.syntax_link.clone())
            })
            .collect()
    }

    /// Find bit offset from screen coordinates
    ///
    /// Traverses: screen → viz → decode → syntax → bit_offset
    pub fn screen_to_bit_offset(&self, screen: ScreenPx) -> Vec<u64> {
        let syntax_ids = self.screen_to_syntax(screen);

        syntax_ids
            .into_iter()
            .filter_map(|syntax_id| {
                self.evidence_chain
                    .syntax_index
                    .find_by_id(&syntax_id)
                    .and_then(|syntax| {
                        self.evidence_chain
                            .bit_offset_index
                            .find_by_id(&syntax.bit_offset_link)
                            .map(|bit| bit.bit_range.start_bit)
                    })
            })
            .collect()
    }

    /// Get the coordinate transformer
    pub fn transformer(&self) -> &CoordinateTransformer {
        &self.transformer
    }

    /// Get the evidence chain
    pub fn evidence_chain(&self) -> &EvidenceChain {
        &self.evidence_chain
    }

    /// Clear all viz evidence (e.g., when changing frames)
    pub fn clear_viz_evidence(&mut self) {
        self.evidence_chain.viz_index = crate::VizIndex::new();
        self.next_evidence_id = 0;
    }

    /// Get count of viz evidence entries
    pub fn viz_evidence_count(&self) -> usize {
        self.evidence_chain.viz_index.len()
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("player_evidence_test.rs");
