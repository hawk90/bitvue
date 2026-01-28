//! H.264/AVC frame extraction
//!
//! Functions for extracting individual frames from H.264 bitstreams

use crate::nal::{find_nal_units, parse_nal_header, NalUnitType};
use crate::parse_avc;
use crate::slice::{SliceHeader, SliceType};
use serde::{Deserialize, Serialize};

/// H.264 frame data extracted from the bitstream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvcFrame {
    /// Frame index in the stream
    pub frame_index: usize,
    /// Frame type (I, P, B)
    pub frame_type: AvcFrameType,
    /// Raw NAL unit data for this frame
    pub nal_data: Vec<u8>,
    /// Starting byte position in the stream
    pub offset: usize,
    /// Frame size in bytes
    pub size: usize,
    /// POC (Picture Order Count)
    pub poc: i32,
    /// Frame number
    pub frame_num: u32,
    /// Whether this is an IDR frame
    pub is_idr: bool,
    /// Whether this is a reference frame
    pub is_ref: bool,
    /// Slice header (if available)
    pub slice_header: Option<SliceHeader>,
}

/// H.264 frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvcFrameType {
    /// I-frame (IDR or non-IDR intra)
    I,
    /// P-frame (predicted)
    P,
    /// B-frame (bi-directional predicted)
    B,
    /// SI-frame (SP/SI switching)
    SI,
    /// SP-frame (SP/SI switching)
    SP,
    /// Unknown frame type
    Unknown,
}

impl AvcFrameType {
    /// Convert from slice type
    pub fn from_slice_type(slice_type: SliceType) -> Self {
        match slice_type {
            SliceType::I => AvcFrameType::I,
            SliceType::Si => AvcFrameType::SI,
            SliceType::P => AvcFrameType::P,
            SliceType::Sp => AvcFrameType::SP,
            SliceType::B => AvcFrameType::B,
        }
    }

    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            AvcFrameType::I => "I",
            AvcFrameType::P => "P",
            AvcFrameType::B => "B",
            AvcFrameType::SI => "SI",
            AvcFrameType::SP => "SP",
            AvcFrameType::Unknown => "Unknown",
        }
    }
}

/// Extract frames from H.264 Annex B byte stream
///
/// This function parses the byte stream and groups NAL units into frames.
/// Each frame consists of one or more NAL units (slice, slice data partitions, etc.)
pub fn extract_annex_b_frames(data: &[u8]) -> Result<Vec<AvcFrame>, String> {
    // First, parse the full stream to get slice information
    let stream = parse_avc(data).map_err(|e| format!("Failed to parse AVC stream: {:?}", e))?;

    // Find all NAL unit start positions
    let nal_positions = find_nal_units(data);

    if nal_positions.is_empty() {
        return Ok(Vec::new());
    }

    // Build NAL unit ranges (start, end) by pairing positions
    let mut nal_ranges: Vec<(usize, usize)> = Vec::new();
    for i in 0..nal_positions.len() {
        let start = nal_positions[i];
        let end = if i + 1 < nal_positions.len() {
            nal_positions[i + 1]
        } else {
            data.len()
        };
        // Adjust for start code (include start code in range)
        let adjusted_start = if start >= 4 { start - 4 } else { 0 };
        nal_ranges.push((adjusted_start, end));
    }

    let mut frames = Vec::new();
    let mut current_frame_nals: Vec<(usize, usize)> = Vec::new();
    let mut current_frame_index = 0;
    let current_poc: Option<i32> = None;
    let mut current_frame_num: Option<u32> = None;
    let mut current_is_idr = false;
    let mut current_is_ref = false;
    let mut current_frame_type = AvcFrameType::Unknown;
    let mut current_slice_header: Option<SliceHeader> = None;

    for (nal_start, nal_end) in nal_ranges {
        // Find the first byte after start code (actual NAL data)
        let nal_data_start =
            if nal_end - nal_start >= 4 && data[nal_start..nal_start + 4] == [0, 0, 0, 1] {
                nal_start + 4
            } else if nal_end - nal_start >= 3 && data[nal_start..nal_start + 3] == [0, 0, 1] {
                nal_start + 3
            } else {
                nal_start
            };

        if nal_data_start >= nal_end {
            continue;
        }

        // Parse NAL header from first byte of NAL data
        let nal_header = match parse_nal_header(data[nal_data_start]) {
            Ok(header) => header,
            Err(_) => continue,
        };

        let nal_type = nal_header.nal_unit_type;

        // Check if this is a VCL NAL (Video Coding Layer)
        if nal_type.is_vcl() {
            // This is a slice NAL
            let is_idr = nal_type == NalUnitType::IdrSlice;
            let is_ref = nal_header.nal_ref_idc != 0;

            // Check if this starts a new frame
            let new_frame = if current_frame_nals.is_empty() {
                true
            } else if is_idr != current_is_idr {
                true // IDR boundary
            } else if let Some(slice) = &current_slice_header {
                // Try to parse the current slice header
                if let Ok(new_slice) = crate::slice::parse_slice_header(
                    &data[nal_data_start + 1..nal_end],
                    &stream.sps_map,
                    &stream.pps_map,
                    nal_type,
                    nal_header.nal_ref_idc,
                ) {
                    // New frame if frame_num changes or first_mb_in_slice == 0
                    new_slice.frame_num != slice.frame_num || new_slice.first_mb_in_slice == 0
                } else {
                    false
                }
            } else {
                false
            };

            if new_frame && !current_frame_nals.is_empty() {
                // Finalize previous frame
                if let Some(frame) = build_frame_from_nals(
                    current_frame_index,
                    &current_frame_nals,
                    data,
                    current_poc.unwrap_or(0),
                    current_frame_num.unwrap_or(0),
                    current_is_idr,
                    current_is_ref,
                    current_frame_type,
                    current_slice_header.clone(),
                ) {
                    frames.push(frame);
                }
                current_frame_index += 1;
                current_frame_nals.clear();
            }

            // Update frame state
            current_is_idr = is_idr;
            current_is_ref = is_ref;

            // Try to parse slice header for frame type
            if let Ok(slice) = crate::slice::parse_slice_header(
                &data[nal_data_start + 1..nal_end],
                &stream.sps_map,
                &stream.pps_map,
                nal_type,
                nal_header.nal_ref_idc,
            ) {
                if current_frame_nals.is_empty() {
                    current_frame_num = Some(slice.frame_num);
                    current_slice_header = Some(slice.clone());
                }

                // Determine frame type from slice type
                current_frame_type = AvcFrameType::from_slice_type(slice.slice_type);
            }

            current_frame_nals.push((nal_start, nal_end));
        } else if !current_frame_nals.is_empty() {
            // Non-VCL NAL after some VCL NALs
            if nal_type == NalUnitType::Aud {
                // AUD definitely ends the current frame
                if let Some(frame) = build_frame_from_nals(
                    current_frame_index,
                    &current_frame_nals,
                    data,
                    current_poc.unwrap_or(0),
                    current_frame_num.unwrap_or(0),
                    current_is_idr,
                    current_is_ref,
                    current_frame_type,
                    current_slice_header.clone(),
                ) {
                    frames.push(frame);
                }
                current_frame_index += 1;
                current_frame_nals.clear();
            }
        }
    }

    // Don't forget the last frame
    if !current_frame_nals.is_empty() {
        // Find POC from parsed slices
        let poc = stream
            .slices
            .get(current_frame_index)
            .map(|s| s.poc)
            .unwrap_or(0);

        if let Some(frame) = build_frame_from_nals(
            current_frame_index,
            &current_frame_nals,
            data,
            poc,
            current_frame_num.unwrap_or(0),
            current_is_idr,
            current_is_ref,
            current_frame_type,
            current_slice_header.clone(),
        ) {
            frames.push(frame);
        }
    }

    Ok(frames)
}

/// Build a frame from collected NAL unit positions
fn build_frame_from_nals(
    frame_index: usize,
    nal_positions: &[(usize, usize)],
    data: &[u8],
    poc: i32,
    frame_num: u32,
    is_idr: bool,
    is_ref: bool,
    frame_type: AvcFrameType,
    slice_header: Option<SliceHeader>,
) -> Option<AvcFrame> {
    if nal_positions.is_empty() {
        return None;
    }

    let offset = nal_positions[0].0;
    let mut size = 0;
    let mut nal_data = Vec::new();

    for (start, end) in nal_positions {
        size += end - start;
        nal_data.extend_from_slice(&data[*start..*end]);
    }

    Some(AvcFrame {
        frame_index,
        frame_type,
        nal_data,
        offset,
        size,
        poc,
        frame_num,
        is_idr,
        is_ref,
        slice_header,
    })
}

/// Extract a single frame by index from Annex B byte stream
pub fn extract_frame_at_index(data: &[u8], frame_index: usize) -> Option<AvcFrame> {
    let frames = extract_annex_b_frames(data).ok()?;
    frames.get(frame_index).cloned()
}

/// Convert AvcFrame to UnitNode format for bitvue-core
pub fn avc_frame_to_unit_node(frame: &AvcFrame, _stream_id: u8) -> bitvue_core::UnitNode {
    bitvue_core::UnitNode {
        key: bitvue_core::UnitKey {
            stream: bitvue_core::StreamId::A,
            unit_type: "FRAME".to_string(),
            offset: frame.offset as u64,
            size: frame.size,
        },
        unit_type: "FRAME".to_string(),
        offset: frame.offset as u64,
        size: frame.size,
        frame_index: Some(frame.frame_index),
        frame_type: Some(frame.frame_type.as_str().to_string()),
        pts: Some(frame.poc as u64),
        dts: None,
        display_name: format!(
            "Frame {} ({})",
            frame.frame_index,
            frame.frame_type.as_str()
        ),
        children: Vec::new(),
        qp_avg: None,  // TODO: Extract from slice data
        mv_grid: None, // TODO: Extract from slice data
        temporal_id: None,
        ref_frames: None, // TODO: Calculate from slice header
        ref_slots: None,
    }
}

/// Convert multiple AvcFrames to UnitNode format
pub fn avc_frames_to_unit_nodes(frames: &[AvcFrame]) -> Vec<bitvue_core::UnitNode> {
    frames
        .iter()
        .map(|f| avc_frame_to_unit_node(f, 0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_empty() {
        let data: &[u8] = &[];
        let frames = extract_annex_b_frames(data);
        assert!(frames.is_ok());
        assert!(frames.unwrap().is_empty());
    }

    #[test]
    fn test_frame_type_display() {
        assert_eq!(AvcFrameType::I.as_str(), "I");
        assert_eq!(AvcFrameType::P.as_str(), "P");
        assert_eq!(AvcFrameType::B.as_str(), "B");
    }

    #[test]
    fn test_frame_type_from_slice() {
        assert_eq!(AvcFrameType::from_slice_type(SliceType::I), AvcFrameType::I);
        assert_eq!(AvcFrameType::from_slice_type(SliceType::P), AvcFrameType::P);
        assert_eq!(AvcFrameType::from_slice_type(SliceType::B), AvcFrameType::B);
    }
}
