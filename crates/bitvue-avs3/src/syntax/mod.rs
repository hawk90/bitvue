//! AVS3 syntax tree construction for the Syntax Detail Panel.
//!
//! Returns plain `(key, value)` pairs rather than bitvue-engine `UnitNode`,
//! keeping this crate free of the stream-state dependency.

use crate::frames::Avs3Frame;
use crate::sequence_header::SequenceHeader;
use serde::{Deserialize, Serialize};

/// A key-value syntax entry used by the Syntax Detail Panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxEntry {
    pub key: String,
    pub value: String,
    pub byte_offset: usize,
}

impl SyntaxEntry {
    fn new(key: impl Into<String>, value: impl Into<String>, offset: usize) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            byte_offset: offset,
        }
    }
}

/// Build syntax entries for a `SequenceHeader`.
pub fn sequence_header_syntax(sh: &SequenceHeader, offset: usize) -> Vec<SyntaxEntry> {
    vec![
        SyntaxEntry::new("profile_id", format!("{:?}", sh.profile), offset + 1),
        SyntaxEntry::new("level_id", format!("{}", sh.level), offset + 2),
        SyntaxEntry::new(
            "progressive_sequence",
            format!("{}", sh.progressive_sequence as u8),
            offset + 3,
        ),
        SyntaxEntry::new("horizontal_size", format!("{}", sh.width), offset + 3),
        SyntaxEntry::new("vertical_size", format!("{}", sh.height), offset + 5),
        SyntaxEntry::new(
            "chroma_format",
            format!("{:?}", sh.chroma_format),
            offset + 7,
        ),
        SyntaxEntry::new(
            "bit_depth_luma",
            format!("{}", sh.bit_depth_luma),
            offset + 8,
        ),
        SyntaxEntry::new(
            "frame_rate_code",
            format!("{}", sh.frame_rate_code),
            offset + 9,
        ),
        SyntaxEntry::new(
            "deblocking_filter_flag",
            format!("{}", sh.deblocking_filter_flag as u8),
            offset + 10,
        ),
        SyntaxEntry::new(
            "sample_adaptive_offset_enabled",
            format!("{}", sh.sample_adaptive_offset_enabled as u8),
            offset + 11,
        ),
        SyntaxEntry::new(
            "adaptive_leveling_filter_enabled (ESAO)",
            format!("{}", sh.adaptive_leveling_filter_enabled as u8),
            offset + 12,
        ),
        SyntaxEntry::new(
            "cross_component_prediction_enabled (CCSAO)",
            format!("{}", sh.cross_component_prediction_enabled as u8),
            offset + 13,
        ),
        SyntaxEntry::new(
            "adaptive_loop_filter_enabled",
            format!("{}", sh.adaptive_loop_filter_enabled as u8),
            offset + 14,
        ),
    ]
}

/// Build syntax entries for a picture header.
pub fn picture_header_syntax(frame: &Avs3Frame) -> Vec<SyntaxEntry> {
    vec![
        SyntaxEntry::new(
            "picture_type",
            format!("{:?}", frame.picture_type),
            frame.offset + 1,
        ),
        SyntaxEntry::new("picture_qp", format!("{}", frame.qp), frame.offset + 3),
        SyntaxEntry::new(
            "display_delay",
            format!("{}", frame.display_delay),
            frame.offset + 4,
        ),
        SyntaxEntry::new(
            "esao_enable",
            format!("{}", frame.esao_enable as u8),
            frame.offset + 6,
        ),
        SyntaxEntry::new(
            "ccsao_enable",
            format!("{}", frame.ccsao_enable as u8),
            frame.offset + 7,
        ),
    ]
}
