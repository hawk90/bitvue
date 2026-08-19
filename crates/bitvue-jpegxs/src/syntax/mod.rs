//! JPEG XS syntax entry extraction for the Syntax panel.

use crate::frames::JpegXsFrame;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyntaxEntry {
    pub key: String,
    pub value: String,
    pub byte_offset: Option<usize>,
}

/// Build a flat list of syntax entries from a parsed frame.
pub fn frame_syntax(frame: &JpegXsFrame) -> Vec<SyntaxEntry> {
    vec![
        SyntaxEntry {
            key: "Width".into(),
            value: frame.width.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "Height".into(),
            value: frame.height.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "NumComps".into(),
            value: frame.num_comps.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "BitDepth".into(),
            value: frame.bit_depth.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "DecompH".into(),
            value: frame.decomp_h.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "DecompV".into(),
            value: frame.decomp_v.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "PrecinctCols".into(),
            value: frame.precinct_cols.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "PrecinctRows".into(),
            value: frame.precinct_rows.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "BppX1000".into(),
            value: frame.bpp_x1000.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "Profile".into(),
            value: format!("0x{:02X}", frame.profile),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "MCTType".into(),
            value: frame.mct_type.clone(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "NLTPresent".into(),
            value: frame.nlt_present.to_string(),
            byte_offset: None,
        },
        SyntaxEntry {
            key: "NumSlices".into(),
            value: frame.num_slices.to_string(),
            byte_offset: None,
        },
    ]
}
