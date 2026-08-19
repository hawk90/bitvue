//! JPEG XS frame extraction from a raw codestream.

use crate::error::{JpegXsError, Result};
use crate::marker::{markers, scan_markers};
use crate::mct::{parse_mct, MctParams};
use crate::nlt::{parse_nlt, NltParams};
use crate::picture_header::{parse_pih, PictureHeader};
use serde::{Deserialize, Serialize};

/// Extracted JPEG XS frame metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JpegXsFrame {
    /// Frame index (0-based).
    pub frame_index: usize,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Number of colour components.
    pub num_comps: u8,
    /// Bit depth per component.
    pub bit_depth: u8,
    /// Number of horizontal wavelet decomposition levels.
    pub decomp_h: u8,
    /// Number of vertical wavelet decomposition levels.
    pub decomp_v: u8,
    /// Number of precinct columns.
    pub precinct_cols: u32,
    /// Number of precinct rows.
    pub precinct_rows: u32,
    /// Precinct width in pixels.
    pub precinct_width: u32,
    /// Precinct height in pixels.
    pub precinct_height: u32,
    /// Target bits-per-pixel × 1000.
    pub bpp_x1000: u32,
    /// Profile byte.
    pub profile: u8,
    /// MCT type string (for serialisation).
    pub mct_type: String,
    /// Whether NLT is present.
    pub nlt_present: bool,
    /// Byte offset of the SOC marker in the input buffer.
    pub offset: usize,
    /// Total byte size of this frame (SOC to EOC inclusive).
    pub size: usize,
    /// Number of slice-data (SLD) segments found.
    pub num_slices: usize,
}

/// Result of extracting JPEG XS frames from a buffer.
pub struct ExtractResult {
    pub frames: Vec<JpegXsFrame>,
    pub parse_errors: usize,
}

/// Extract JPEG XS frames from a raw codestream buffer.
///
/// Supports single-frame and multi-frame (streaming) codestreams.
/// Returns at most `limit` frames (0 = unlimited).
pub fn extract_jpegxs_frames(data: &[u8], limit: usize) -> Result<ExtractResult> {
    let max = if limit == 0 { usize::MAX } else { limit };
    let segments = scan_markers(data);
    if segments.is_empty() {
        return Ok(ExtractResult {
            frames: Vec::new(),
            parse_errors: 0,
        });
    }

    // Check SOC
    if segments[0].marker != markers::SOC {
        return Err(JpegXsError::MissingMarker("SOC"));
    }

    let mut frames = Vec::new();
    let mut parse_errors = 0usize;
    let mut frame_index = 0usize;

    // State within a single frame parse
    let mut soc_offset: Option<usize> = None;
    let mut pih: Option<PictureHeader> = None;
    let mut mct: Option<MctParams> = None;
    let mut nlt: Option<NltParams> = None;
    let mut num_slices = 0usize;

    for seg in &segments {
        match seg.marker {
            markers::SOC => {
                soc_offset = Some(seg.offset);
                pih = None;
                mct = None;
                nlt = None;
                num_slices = 0;
            }
            markers::PIH => match parse_pih(seg.data) {
                Ok(p) => pih = Some(p),
                Err(_) => parse_errors += 1,
            },
            markers::MCT => match parse_mct(seg.data) {
                Ok(m) => mct = Some(m),
                Err(_) => parse_errors += 1,
            },
            markers::NLT => {
                let nc = pih.as_ref().map(|p| p.num_comps as usize).unwrap_or(3);
                match parse_nlt(seg.data, nc) {
                    Ok(n) => nlt = Some(n),
                    Err(_) => parse_errors += 1,
                }
            }
            markers::SLD => {
                num_slices += 1;
            }
            markers::EOC => {
                if frames.len() >= max {
                    break;
                }
                if let (Some(start), Some(ph)) = (soc_offset, pih.as_ref()) {
                    let end = seg.offset + 2; // include EOC marker bytes
                    let size = end.saturating_sub(start);
                    let (pcols, prows, pw, ph_px) = ph
                        .slice_geom
                        .map(|g| (g.num_cols, g.num_rows, g.slice_width, g.slice_height))
                        .unwrap_or((0, 0, 0, 0));

                    let mct_type_str = mct
                        .as_ref()
                        .map(|m| {
                            format!("{:?}", m.mct_type)
                                .trim_start_matches("Mct")
                                .to_string()
                        })
                        .unwrap_or_else(|| "None".to_string());

                    frames.push(JpegXsFrame {
                        frame_index,
                        width: ph.width,
                        height: ph.height,
                        num_comps: ph.num_comps,
                        bit_depth: ph.bit_depth,
                        decomp_h: ph.decomp.horizontal,
                        decomp_v: ph.decomp.vertical,
                        precinct_cols: pcols,
                        precinct_rows: prows,
                        precinct_width: pw,
                        precinct_height: ph_px,
                        bpp_x1000: ph.bpp_x1000,
                        profile: ph.profile,
                        mct_type: mct_type_str,
                        nlt_present: nlt.is_some(),
                        offset: start,
                        size,
                        num_slices,
                    });
                    frame_index += 1;
                } else {
                    parse_errors += 1;
                }
                soc_offset = None;
                pih = None;
                mct = None;
                nlt = None;
                num_slices = 0;
            }
            _ => {}
        }
    }

    Ok(ExtractResult {
        frames,
        parse_errors,
    })
}
