//! Semantic Evidence Layer - Extended Evidence Chain Layer 3
//!
//! Codec-specific semantic evidence that captures the "meaning" of syntax
//! elements beyond their bit representation.
//!
//! Per PERFECT_VISUALIZATION_SPEC: Layer 3 bridges syntax structure to
//! codec-specific semantics for intelligent visualization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::evidence::EvidenceId;

/// Codec type for semantic evidence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Codec {
    Av1,
    H264,
    Hevc,
    Vp9,
    Vvc,
    Avs3,
}

// ═══════════════════════════════════════════════════════════════════════════
// AV1 Semantic Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// AV1 CDF (Cumulative Distribution Function) update evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1CdfUpdate {
    pub context_id: u32,
    pub symbol: u8,
    pub old_probability: f32,
    pub new_probability: f32,
    pub divergence_from_default: f32,
}

/// AV1 tile group structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1TileGroupInfo {
    pub tile_start_idx: u32,
    pub tile_end_idx: u32,
    pub tile_count: u32,
    pub is_uniform_spacing: bool,
}

/// AV1 superres information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1SuperresInfo {
    pub enabled: bool,
    pub coded_width: u32,
    pub upscaled_width: u32,
    pub scale_denominator: u8,
}

/// AV1 film grain parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Av1FilmGrainInfo {
    pub enabled: bool,
    pub grain_seed: u16,
    pub num_y_points: u8,
    pub num_cb_points: u8,
    pub num_cr_points: u8,
    pub chroma_scaling_from_luma: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// H.264 Semantic Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// H.264 Picture Order Count tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264PocInfo {
    pub poc_type: u8,
    pub poc_lsb: u32,
    pub poc_msb: i32,
    pub top_field_poc: i32,
    pub bottom_field_poc: i32,
    pub wrap_count: u32,
    pub is_wrap_boundary: bool,
}

/// H.264 MMCO (Memory Management Control Operation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum H264MmcoOperation {
    MarkShortTermUnused {
        difference_of_pic_nums: u32,
    },
    MarkLongTermUnused {
        long_term_pic_num: u32,
    },
    AssignLongTermIdx {
        difference_of_pic_nums: u32,
        long_term_frame_idx: u32,
    },
    SetMaxLongTermIdx {
        max_long_term_frame_idx_plus1: u32,
    },
    MarkAllUnused,
    MarkCurrentAsLongTerm {
        long_term_frame_idx: u32,
    },
}

/// H.264 DPB sliding window event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264DpbSliding {
    pub evicted_frame_num: u32,
    pub evicted_poc: i32,
    pub reason: String,
    pub dpb_fullness_before: u8,
    pub dpb_fullness_after: u8,
}

/// H.264 B-frame pyramid level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H264BPyramidInfo {
    pub pyramid_depth: u8,
    pub current_level: u8,
    pub is_reference: bool,
    pub parent_poc: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════
// HEVC Semantic Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// HEVC NAL unit type for IRAP detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HevcNalType {
    TrailN,
    TrailR,
    TsaN,
    TsaR,
    StsaN,
    StsaR,
    RadlN,
    RadlR,
    RaslN,
    RaslR,
    BlaWLp,
    BlaWRadl,
    BlaNLp,
    IdrWRadl,
    IdrNLp,
    CraNut,
    Other(u8),
}

/// HEVC IRAP (Intra Random Access Point) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcIrapInfo {
    pub nal_type: HevcNalType,
    pub is_irap: bool,
    pub is_idr: bool,
    pub is_cra: bool,
    pub is_bla: bool,
    pub no_rasl_output_flag: bool,
    pub associated_rasl_count: u32,
}

/// HEVC temporal layer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcTemporalInfo {
    pub temporal_id: u8,
    pub max_temporal_layers: u8,
    pub discardable: bool,
    pub sub_layer_non_ref: bool,
}

/// HEVC CTU (Coding Tree Unit) boundary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcCtuBoundary {
    pub ctu_addr: u32,
    pub slice_idx: u32,
    pub tile_idx: u32,
    pub is_slice_boundary: bool,
    pub is_tile_boundary: bool,
    pub dependent_slice: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// VP9 Semantic Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// VP9 superframe information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9SuperframeInfo {
    pub is_superframe: bool,
    pub frame_count: u8,
    pub frame_sizes: Vec<u32>,
    pub total_size: u32,
}

/// VP9 reference frame buffer state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9RefBufferInfo {
    pub last_ref_idx: u8,
    pub golden_ref_idx: u8,
    pub altref_ref_idx: u8,
    pub refresh_frame_flags: u8,
    pub ref_frame_sign_bias: [bool; 4],
}

/// VP9 segmentation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vp9SegmentationInfo {
    pub enabled: bool,
    pub update_map: bool,
    pub update_data: bool,
    pub abs_or_delta_update: bool,
    pub segment_feature_active: [[bool; 4]; 8],
    pub segment_feature_data: [[i16; 4]; 8],
}

// ═══════════════════════════════════════════════════════════════════════════
// VVC Semantic Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// VVC GDR (Gradual Decoding Refresh) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VvcGdrInfo {
    pub is_gdr: bool,
    pub recovery_poc_cnt: u32,
    pub no_output_before_recovery: bool,
    pub gradual_refresh_line: u32,
}

/// VVC RPL (Reference Picture List) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VvcRplInfo {
    pub num_ref_entries: [u8; 2],
    pub ltrp_in_header_flag: bool,
    pub inter_layer_ref_present: bool,
    pub ref_pic_list: Vec<VvcRefPicEntry>,
}

/// VVC reference picture entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VvcRefPicEntry {
    pub is_long_term: bool,
    pub is_inter_layer: bool,
    pub poc_delta: i32,
    pub layer_id: u8,
}

/// VVC ALF (Adaptive Loop Filter) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VvcAlfInfo {
    pub alf_enabled: bool,
    pub num_alf_aps_ids_luma: u8,
    pub alf_chroma_idc: u8,
    pub alf_cc_cb_enabled: bool,
    pub alf_cc_cr_enabled: bool,
}

/// VVC subpicture information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VvcSubpicInfo {
    pub subpic_idx: u32,
    pub subpic_id: u32,
    pub ctu_top_left_x: u32,
    pub ctu_top_left_y: u32,
    pub width_in_ctus: u32,
    pub height_in_ctus: u32,
    pub treated_as_pic: bool,
    pub loop_filter_across_boundary: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// Unified Semantic Evidence
// ═══════════════════════════════════════════════════════════════════════════

/// Unified semantic evidence container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticEvidence {
    // AV1
    Av1Cdf(Av1CdfUpdate),
    Av1TileGroup(Av1TileGroupInfo),
    Av1Superres(Av1SuperresInfo),
    Av1FilmGrain(Av1FilmGrainInfo),

    // H.264
    H264Poc(H264PocInfo),
    H264Mmco(H264MmcoOperation),
    H264DpbSliding(H264DpbSliding),
    H264BPyramid(H264BPyramidInfo),

    // HEVC
    HevcIrap(HevcIrapInfo),
    HevcTemporal(HevcTemporalInfo),
    HevcCtuBoundary(HevcCtuBoundary),

    // VP9
    Vp9Superframe(Vp9SuperframeInfo),
    Vp9RefBuffer(Vp9RefBufferInfo),
    Vp9Segmentation(Vp9SegmentationInfo),

    // VVC
    VvcGdr(VvcGdrInfo),
    VvcRpl(VvcRplInfo),
    VvcAlf(VvcAlfInfo),
    VvcSubpic(VvcSubpicInfo),

    // Generic
    Custom {
        codec: Codec,
        name: String,
        data: HashMap<String, String>,
    },
}

impl SemanticEvidence {
    /// Get the codec this evidence belongs to
    pub fn codec(&self) -> Codec {
        match self {
            Self::Av1Cdf(_)
            | Self::Av1TileGroup(_)
            | Self::Av1Superres(_)
            | Self::Av1FilmGrain(_) => Codec::Av1,
            Self::H264Poc(_)
            | Self::H264Mmco(_)
            | Self::H264DpbSliding(_)
            | Self::H264BPyramid(_) => Codec::H264,
            Self::HevcIrap(_) | Self::HevcTemporal(_) | Self::HevcCtuBoundary(_) => Codec::Hevc,
            Self::Vp9Superframe(_) | Self::Vp9RefBuffer(_) | Self::Vp9Segmentation(_) => Codec::Vp9,
            Self::VvcGdr(_) | Self::VvcRpl(_) | Self::VvcAlf(_) | Self::VvcSubpic(_) => Codec::Vvc,
            Self::Custom { codec, .. } => *codec,
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            Self::Av1Cdf(c) => format!(
                "CDF update: context {} divergence {:.2}",
                c.context_id, c.divergence_from_default
            ),
            Self::Av1TileGroup(t) => {
                format!("Tile group: tiles {}-{}", t.tile_start_idx, t.tile_end_idx)
            }
            Self::Av1Superres(s) => {
                if s.enabled {
                    format!("Superres: {}→{}", s.coded_width, s.upscaled_width)
                } else {
                    "Superres: disabled".into()
                }
            }
            Self::Av1FilmGrain(f) => {
                if f.enabled {
                    format!("Film grain: seed={}", f.grain_seed)
                } else {
                    "Film grain: disabled".into()
                }
            }

            Self::H264Poc(p) => format!(
                "POC: {} (LSB={}, wrap #{})",
                p.poc_lsb as i32 + p.poc_msb,
                p.poc_lsb,
                p.wrap_count
            ),
            Self::H264Mmco(m) => format!("MMCO: {:?}", m),
            Self::H264DpbSliding(d) => format!(
                "DPB evict: frame_num={} ({})",
                d.evicted_frame_num, d.reason
            ),
            Self::H264BPyramid(b) => {
                format!("B-pyramid: level {}/{}", b.current_level, b.pyramid_depth)
            }

            Self::HevcIrap(i) => format!(
                "IRAP: {:?} (is_idr={}, is_cra={})",
                i.nal_type, i.is_idr, i.is_cra
            ),
            Self::HevcTemporal(t) => format!(
                "Temporal: layer {} of {}",
                t.temporal_id, t.max_temporal_layers
            ),
            Self::HevcCtuBoundary(c) => format!(
                "CTU {}: slice={}, tile={}",
                c.ctu_addr, c.slice_idx, c.tile_idx
            ),

            Self::Vp9Superframe(s) => {
                if s.is_superframe {
                    format!("Superframe: {} frames", s.frame_count)
                } else {
                    "Not a superframe".into()
                }
            }
            Self::Vp9RefBuffer(r) => format!(
                "Refs: last={}, golden={}, altref={}",
                r.last_ref_idx, r.golden_ref_idx, r.altref_ref_idx
            ),
            Self::Vp9Segmentation(s) => {
                if s.enabled {
                    "Segmentation: enabled".into()
                } else {
                    "Segmentation: disabled".into()
                }
            }

            Self::VvcGdr(g) => {
                if g.is_gdr {
                    format!("GDR: recovery POC={}", g.recovery_poc_cnt)
                } else {
                    "Not GDR".into()
                }
            }
            Self::VvcRpl(r) => format!(
                "RPL: {} L0 refs, {} L1 refs",
                r.num_ref_entries[0], r.num_ref_entries[1]
            ),
            Self::VvcAlf(a) => {
                if a.alf_enabled {
                    "ALF: enabled".into()
                } else {
                    "ALF: disabled".into()
                }
            }
            Self::VvcSubpic(s) => format!(
                "Subpic {}: {}x{} CTUs",
                s.subpic_idx, s.width_in_ctus, s.height_in_ctus
            ),

            Self::Custom { name, .. } => format!("Custom: {}", name),
        }
    }
}

/// Semantic evidence record with links to syntax layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEvidenceRecord {
    /// Unique evidence ID
    pub id: EvidenceId,

    /// Frame display index
    pub display_idx: u64,

    /// The semantic evidence
    pub evidence: SemanticEvidence,

    /// Link to syntax evidence
    pub syntax_link: EvidenceId,

    /// Additional context metadata
    pub metadata: HashMap<String, String>,
}

impl SemanticEvidenceRecord {
    pub fn new(
        id: EvidenceId,
        display_idx: u64,
        evidence: SemanticEvidence,
        syntax_link: EvidenceId,
    ) -> Self {
        Self {
            id,
            display_idx,
            evidence,
            syntax_link,
            metadata: HashMap::new(),
        }
    }

    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Semantic evidence index for fast lookup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticIndex {
    evidence: Vec<SemanticEvidenceRecord>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, record: SemanticEvidenceRecord) {
        self.evidence.push(record);
    }

    pub fn find_by_id(&self, id: &str) -> Option<&SemanticEvidenceRecord> {
        self.evidence.iter().find(|e| e.id == id)
    }

    pub fn find_by_display_idx(&self, display_idx: u64) -> Vec<&SemanticEvidenceRecord> {
        self.evidence
            .iter()
            .filter(|e| e.display_idx == display_idx)
            .collect()
    }

    pub fn find_by_codec(&self, codec: Codec) -> Vec<&SemanticEvidenceRecord> {
        self.evidence
            .iter()
            .filter(|e| e.evidence.codec() == codec)
            .collect()
    }

    pub fn find_by_syntax_link(&self, syntax_id: &str) -> Vec<&SemanticEvidenceRecord> {
        self.evidence
            .iter()
            .filter(|e| e.syntax_link == syntax_id)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }

    pub fn all(&self) -> &[SemanticEvidenceRecord] {
        &self.evidence
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("semantic_evidence_test.rs");
