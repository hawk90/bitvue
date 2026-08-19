//! AVS3 frame extraction from raw bitstream.
//!
//! Scans the byte stream for AVS3 start codes, parses sequence and picture
//! headers, and returns a list of `Avs3Frame` records for downstream use.

use crate::error::Result;
use crate::nal::{scan_nal_units, Sci};
use crate::picture_header::{parse_i_picture_header, parse_pb_picture_header, PictureType};
use crate::sequence_header::{parse_sequence_header, SequenceHeader};
use serde::{Deserialize, Serialize};

/// A single AVS3 access unit (picture).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avs3Frame {
    /// Frame index in the stream.
    pub frame_index: usize,
    /// Picture type (I / P / B).
    pub picture_type: PictureType,
    /// Base QP from picture header.
    pub qp: u8,
    /// Frame display order (from picture header).
    pub display_delay: u32,
    /// Byte offset of picture header start code.
    pub offset: usize,
    /// Byte size of this access unit (start code to next start code).
    pub size: usize,
    /// Raw bytes of the picture NAL unit (including SCI).
    pub data: Vec<u8>,
    /// ESAO enabled for this picture.
    pub esao_enable: bool,
    /// CCSAO enabled for this picture.
    pub ccsao_enable: bool,
}

impl Avs3Frame {
    /// Returns `true` if this is an intra (key) frame.
    pub fn is_key_frame(&self) -> bool {
        self.picture_type == PictureType::I
    }

    /// Frame type as a short string ("I", "P", "B").
    pub fn frame_type_str(&self) -> &'static str {
        match self.picture_type {
            PictureType::I => "I",
            PictureType::P => "P",
            PictureType::B => "B",
        }
    }
}

/// Result of extracting all frames from a byte buffer.
pub struct ExtractResult {
    pub frames: Vec<Avs3Frame>,
    /// The most-recently-seen sequence header (if any).
    pub sequence_header: Option<SequenceHeader>,
    /// Number of parse errors encountered (non-fatal).
    pub parse_errors: usize,
}

/// Extract AVS3 frames from a raw bitstream buffer.
///
/// Returns all frames found, up to `limit` (0 = unlimited).
pub fn extract_avs3_frames(data: &[u8], limit: usize) -> Result<ExtractResult> {
    let units = scan_nal_units(data);
    let max = if limit == 0 { usize::MAX } else { limit };

    let mut frames = Vec::new();
    let mut seq_header: Option<SequenceHeader> = None;
    let mut parse_errors = 0usize;
    let mut frame_index = 0usize;

    for unit in &units {
        if frames.len() >= max {
            break;
        }

        match unit.sci {
            Sci::SequenceHeader => match parse_sequence_header(unit.data) {
                Ok(sh) => seq_header = Some(sh),
                Err(e) => {
                    abseil::vlog!(1, "AVS3: seq header parse error: {e}");
                    parse_errors += 1;
                }
            },
            Sci::IFrame => {
                match parse_i_picture_header(unit.data) {
                    Ok(ph) => {
                        frames.push(Avs3Frame {
                            frame_index,
                            picture_type: PictureType::I,
                            qp: ph.picture_qp,
                            display_delay: ph.display_delay,
                            offset: unit.offset,
                            size: unit.data.len() + 3, // include 3-byte start code
                            data: unit.data.to_vec(),
                            esao_enable: ph.esao_enable,
                            ccsao_enable: ph.ccsao_enable,
                        });
                        frame_index += 1;
                    }
                    Err(e) => {
                        abseil::vlog!(
                            1,
                            "AVS3: I-frame header parse error at {:#x}: {e}",
                            unit.offset
                        );
                        parse_errors += 1;
                    }
                }
            }
            Sci::PBFrame => match parse_pb_picture_header(unit.data) {
                Ok(ph) => {
                    frames.push(Avs3Frame {
                        frame_index,
                        picture_type: ph.picture_type,
                        qp: ph.picture_qp,
                        display_delay: ph.display_delay,
                        offset: unit.offset,
                        size: unit.data.len() + 3,
                        data: unit.data.to_vec(),
                        esao_enable: ph.esao_enable,
                        ccsao_enable: ph.ccsao_enable,
                    });
                    frame_index += 1;
                }
                Err(e) => {
                    abseil::vlog!(
                        1,
                        "AVS3: P/B-frame header parse error at {:#x}: {e}",
                        unit.offset
                    );
                    parse_errors += 1;
                }
            },
            // Extension, UserData, SequenceEnd → skip
            _ => {}
        }
    }

    Ok(ExtractResult {
        frames,
        sequence_header: seq_header,
        parse_errors,
    })
}
