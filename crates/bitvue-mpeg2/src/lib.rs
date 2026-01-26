//! MPEG-2 Video bitstream parser for bitvue.
//!
//! This crate provides parsing capabilities for MPEG-2 Video (ISO/IEC 13818-2)
//! bitstreams, extracting sequence headers, GOP headers, and picture data.
//!
//! # Features
//!
//! - Start code parsing
//! - Sequence header parsing
//! - Sequence extension parsing
//! - GOP (Group of Pictures) header parsing
//! - Picture header and coding extension parsing
//! - Slice header parsing
//!
//! # Example
//!
//! ```ignore
//! use bitvue_mpeg2::{parse_mpeg2, Mpeg2Stream};
//!
//! let data: &[u8] = &[/* MPEG-2 bitstream data */];
//! let stream = parse_mpeg2(data)?;
//!
//! println!("Dimensions: {:?}", stream.dimensions());
//! println!("Frame count: {}", stream.pictures.len());
//! ```

pub mod bitreader;
pub mod error;
pub mod gop;
pub mod picture;
pub mod sequence;
pub mod slice;
pub mod start_code;

pub use bitreader::BitReader;
pub use error::{Mpeg2Error, Result};
pub use gop::GopHeader;
pub use picture::{PictureCodingExtension, PictureHeader, PictureType};
pub use sequence::{ChromaFormat, SequenceExtension, SequenceHeader};
pub use slice::SliceHeader;
pub use start_code::{find_start_codes, StartCode, StartCodeType};

use serde::{Deserialize, Serialize};

/// Parsed MPEG-2 Video stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mpeg2Stream {
    /// Sequence headers found in the stream.
    pub sequence_headers: Vec<ParsedSequence>,
    /// GOP headers.
    pub gop_headers: Vec<ParsedGop>,
    /// Pictures (frames).
    pub pictures: Vec<ParsedPicture>,
    /// All start codes with their byte offsets.
    pub start_codes: Vec<(usize, StartCode)>,
}

/// Parsed sequence with header and optional extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSequence {
    /// Byte offset in stream.
    pub offset: usize,
    /// Sequence header.
    pub header: SequenceHeader,
    /// Sequence extension (for MPEG-2, not MPEG-1).
    pub extension: Option<SequenceExtension>,
}

/// Parsed GOP header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedGop {
    /// Byte offset in stream.
    pub offset: usize,
    /// GOP header data.
    pub header: GopHeader,
}

/// Parsed picture with header and extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPicture {
    /// Byte offset in stream.
    pub offset: usize,
    /// Picture header.
    pub header: PictureHeader,
    /// Picture coding extension (for MPEG-2).
    pub coding_extension: Option<PictureCodingExtension>,
    /// Temporal reference.
    pub temporal_reference: u16,
    /// Picture type.
    pub picture_type: PictureType,
    /// Slice headers for this picture.
    pub slices: Vec<ParsedSlice>,
}

/// Parsed slice header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSlice {
    /// Byte offset in stream.
    pub offset: usize,
    /// Slice vertical position (1-175 for SD, higher for HD).
    pub slice_vertical_position: u8,
    /// Slice header data.
    pub header: SliceHeader,
}

impl Mpeg2Stream {
    /// Get video dimensions from the first sequence header.
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        self.sequence_headers.first().map(|seq| {
            let mut width = seq.header.horizontal_size_value as u32;
            let mut height = seq.header.vertical_size_value as u32;

            // Apply extension bits for HD
            if let Some(ref ext) = seq.extension {
                width |= (ext.horizontal_size_extension as u32) << 12;
                height |= (ext.vertical_size_extension as u32) << 12;
            }

            (width, height)
        })
    }

    /// Get frame rate.
    pub fn frame_rate(&self) -> Option<f64> {
        self.sequence_headers.first().map(|seq| {
            seq.header.frame_rate()
        })
    }

    /// Get aspect ratio string.
    pub fn aspect_ratio(&self) -> Option<&'static str> {
        self.sequence_headers.first().map(|seq| {
            seq.header.aspect_ratio_string()
        })
    }

    /// Get bit rate in bits per second.
    pub fn bit_rate(&self) -> Option<u32> {
        self.sequence_headers.first().and_then(|seq| {
            let base = (seq.header.bit_rate_value as u32) * 400;
            if let Some(ref ext) = seq.extension {
                Some(base | ((ext.bit_rate_extension as u32) << 18))
            } else {
                Some(base)
            }
        })
    }

    /// Check if this is MPEG-2 (has extension) or MPEG-1.
    pub fn is_mpeg2(&self) -> bool {
        self.sequence_headers.first()
            .map(|seq| seq.extension.is_some())
            .unwrap_or(false)
    }

    /// Get profile and level (MPEG-2 only).
    pub fn profile_level(&self) -> Option<(u8, u8)> {
        self.sequence_headers.first()
            .and_then(|seq| seq.extension.as_ref())
            .map(|ext| (ext.profile_and_level_indication >> 4, ext.profile_and_level_indication & 0x0F))
    }

    /// Count I, P, B frames.
    pub fn frame_type_counts(&self) -> (usize, usize, usize) {
        let mut i_count = 0;
        let mut p_count = 0;
        let mut b_count = 0;

        for pic in &self.pictures {
            match pic.picture_type {
                PictureType::I => i_count += 1,
                PictureType::P => p_count += 1,
                PictureType::B => b_count += 1,
                _ => {}
            }
        }

        (i_count, p_count, b_count)
    }
}

/// Parse MPEG-2 Video elementary stream.
pub fn parse_mpeg2(data: &[u8]) -> Result<Mpeg2Stream> {
    let start_codes = find_start_codes(data);

    let mut sequence_headers = Vec::new();
    let mut gop_headers = Vec::new();
    let mut pictures = Vec::new();

    let mut current_picture: Option<ParsedPicture> = None;
    let mut pending_sequence: Option<ParsedSequence> = None;

    for i in 0..start_codes.len() {
        let (offset, sc) = &start_codes[i];

        // Calculate payload end
        let payload_start = offset + 4; // After start code
        let payload_end = if i + 1 < start_codes.len() {
            start_codes[i + 1].0
        } else {
            data.len()
        };

        if payload_start >= payload_end || payload_start >= data.len() {
            continue;
        }

        let payload = &data[payload_start..payload_end.min(data.len())];

        match sc.code_type {
            StartCodeType::SequenceHeader => {
                if let Ok(header) = sequence::parse_sequence_header(payload) {
                    pending_sequence = Some(ParsedSequence {
                        offset: *offset,
                        header,
                        extension: None,
                    });
                }
            }
            StartCodeType::Extension => {
                if !payload.is_empty() {
                    let ext_id = payload[0] >> 4;

                    // Sequence extension
                    if ext_id == 1 {
                        if let Some(ref mut seq) = pending_sequence {
                            if let Ok(ext) = sequence::parse_sequence_extension(payload) {
                                seq.extension = Some(ext);
                            }
                        }
                    }
                    // Picture coding extension
                    else if ext_id == 8 {
                        if let Some(ref mut pic) = current_picture {
                            if let Ok(ext) = picture::parse_picture_coding_extension(payload) {
                                pic.coding_extension = Some(ext);
                            }
                        }
                    }
                }
            }
            StartCodeType::GroupOfPictures => {
                // Finalize pending sequence
                if let Some(seq) = pending_sequence.take() {
                    sequence_headers.push(seq);
                }

                if let Ok(header) = gop::parse_gop_header(payload) {
                    gop_headers.push(ParsedGop {
                        offset: *offset,
                        header,
                    });
                }
            }
            StartCodeType::Picture => {
                // Finalize pending sequence
                if let Some(seq) = pending_sequence.take() {
                    sequence_headers.push(seq);
                }

                // Finalize previous picture
                if let Some(pic) = current_picture.take() {
                    pictures.push(pic);
                }

                if let Ok(header) = picture::parse_picture_header(payload) {
                    current_picture = Some(ParsedPicture {
                        offset: *offset,
                        temporal_reference: header.temporal_reference,
                        picture_type: header.picture_coding_type,
                        header,
                        coding_extension: None,
                        slices: Vec::new(),
                    });
                }
            }
            StartCodeType::Slice(vertical_pos) => {
                if let Some(ref mut pic) = current_picture {
                    if let Ok(header) = slice::parse_slice_header(payload) {
                        pic.slices.push(ParsedSlice {
                            offset: *offset,
                            slice_vertical_position: vertical_pos,
                            header,
                        });
                    }
                }
            }
            StartCodeType::SequenceEnd => {
                // Finalize everything
                if let Some(seq) = pending_sequence.take() {
                    sequence_headers.push(seq);
                }
                if let Some(pic) = current_picture.take() {
                    pictures.push(pic);
                }
            }
            _ => {}
        }
    }

    // Finalize any remaining data
    if let Some(seq) = pending_sequence.take() {
        sequence_headers.push(seq);
    }
    if let Some(pic) = current_picture.take() {
        pictures.push(pic);
    }

    Ok(Mpeg2Stream {
        sequence_headers,
        gop_headers,
        pictures,
        start_codes,
    })
}

/// Quick parse to extract basic stream info.
pub fn parse_mpeg2_quick(data: &[u8]) -> Result<Mpeg2QuickInfo> {
    let start_codes = find_start_codes(data);

    let mut info = Mpeg2QuickInfo {
        is_mpeg2: false,
        width: None,
        height: None,
        frame_rate: None,
        bit_rate: None,
        i_frame_count: 0,
        p_frame_count: 0,
        b_frame_count: 0,
        total_frames: 0,
        gop_count: 0,
    };

    let mut found_sequence = false;

    for i in 0..start_codes.len() {
        let (offset, sc) = &start_codes[i];

        let payload_start = offset + 4;
        let payload_end = if i + 1 < start_codes.len() {
            start_codes[i + 1].0
        } else {
            data.len()
        };

        if payload_start >= payload_end || payload_start >= data.len() {
            continue;
        }

        let payload = &data[payload_start..payload_end.min(data.len())];

        match sc.code_type {
            StartCodeType::SequenceHeader if !found_sequence => {
                if let Ok(header) = sequence::parse_sequence_header(payload) {
                    info.width = Some(header.horizontal_size_value as u32);
                    info.height = Some(header.vertical_size_value as u32);
                    info.frame_rate = Some(header.frame_rate());
                    info.bit_rate = Some((header.bit_rate_value as u32) * 400);
                    found_sequence = true;
                }
            }
            StartCodeType::Extension if found_sequence && !info.is_mpeg2 => {
                if !payload.is_empty() && (payload[0] >> 4) == 1 {
                    info.is_mpeg2 = true;
                    if let Ok(ext) = sequence::parse_sequence_extension(payload) {
                        if let (Some(w), Some(h)) = (info.width, info.height) {
                            info.width = Some(w | ((ext.horizontal_size_extension as u32) << 12));
                            info.height = Some(h | ((ext.vertical_size_extension as u32) << 12));
                        }
                    }
                }
            }
            StartCodeType::GroupOfPictures => {
                info.gop_count += 1;
            }
            StartCodeType::Picture => {
                if let Ok(header) = picture::parse_picture_header(payload) {
                    info.total_frames += 1;
                    match header.picture_coding_type {
                        PictureType::I => info.i_frame_count += 1,
                        PictureType::P => info.p_frame_count += 1,
                        PictureType::B => info.b_frame_count += 1,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(info)
}

/// Quick stream info without full parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mpeg2QuickInfo {
    /// Is this MPEG-2 (has extensions) or MPEG-1?
    pub is_mpeg2: bool,
    /// Video width.
    pub width: Option<u32>,
    /// Video height.
    pub height: Option<u32>,
    /// Frame rate.
    pub frame_rate: Option<f64>,
    /// Bit rate in bps.
    pub bit_rate: Option<u32>,
    /// I-frame count.
    pub i_frame_count: usize,
    /// P-frame count.
    pub p_frame_count: usize,
    /// B-frame count.
    pub b_frame_count: usize,
    /// Total frame count.
    pub total_frames: usize,
    /// GOP count.
    pub gop_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stream() {
        let data: &[u8] = &[];
        let stream = parse_mpeg2(data).unwrap();
        assert_eq!(stream.pictures.len(), 0);
        assert_eq!(stream.sequence_headers.len(), 0);
    }
}
