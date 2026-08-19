//! VC-3/DNxHD frame extraction.

use crate::error::Result;
use crate::segment::scan_segments;
use serde::{Deserialize, Serialize};

/// A single VC-3/DNxHD frame record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vc3Frame {
    pub frame_index: usize,
    pub comp_id: String,
    pub is_dnxhr: bool,
    pub width: u32,
    pub height: u32,
    pub bits_per_component: u8,
    pub chroma_sampling: u16,
    pub frame_size: u32,
    pub mb_cols: u32,
    pub mb_rows: u32,
    pub bits_per_mb: u32,
    pub offset: usize,
}

impl Vc3Frame {
    pub fn frame_type_str(&self) -> &'static str {
        if self.is_dnxhr {
            "DNxHR"
        } else {
            "DNxHD"
        }
    }
}

pub struct ExtractResult {
    pub frames: Vec<Vc3Frame>,
    pub parse_errors: usize,
}

pub fn extract_vc3_frames(data: &[u8], limit: usize) -> Result<ExtractResult> {
    let max = if limit == 0 { usize::MAX } else { limit };
    let headers = scan_segments(data);

    let frames = headers
        .into_iter()
        .take(max)
        .enumerate()
        .map(|(i, h)| Vc3Frame {
            frame_index: i,
            comp_id: h.comp_id.name().to_string(),
            is_dnxhr: h.comp_id.is_dnxhr(),
            width: h.width,
            height: h.height,
            bits_per_component: h.bits_per_component,
            chroma_sampling: h.chroma_sampling,
            frame_size: h.frame_size,
            mb_cols: h.mb_cols,
            mb_rows: h.mb_rows,
            bits_per_mb: h.bits_per_mb,
            offset: h.offset,
        })
        .collect();

    Ok(ExtractResult {
        frames,
        parse_errors: 0,
    })
}
