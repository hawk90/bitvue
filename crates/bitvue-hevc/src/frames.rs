//! HEVC/H.265 frame extraction
//!
//! Functions for extracting individual frames from HEVC bitstreams

use bitvue_core::BitvueError;
use crate::nal::{find_nal_units, parse_nal_header, NalUnitType};
use crate::parse_hevc;
use crate::slice::SliceHeader;
use serde::{Deserialize, Serialize};

/// HEVC frame data extracted from the bitstream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HevcFrame {
    /// Frame index in the stream
    pub frame_index: usize,
    /// Frame type (I, P, B)
    pub frame_type: HevcFrameType,
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
    /// Whether this is an IRAP frame (IDR, CRA, BLA)
    pub is_irap: bool,
    /// Whether this is a reference frame
    pub is_ref: bool,
    /// Temporal ID
    pub temporal_id: Option<u8>,
    /// Slice header (if available)
    pub slice_header: Option<SliceHeader>,
}

impl HevcFrame {
    /// Creates a new HevcFrameBuilder for constructing HevcFrame instances
    pub fn builder() -> HevcFrameBuilder {
        HevcFrameBuilder::default()
    }
}

/// Builder for constructing HevcFrame instances
///
/// # Example
///
/// ```
/// use bitvue_hevc::frames::{HevcFrame, HevcFrameType};
///
/// let frame = HevcFrame::builder()
///     .frame_index(0)
///     .frame_type(HevcFrameType::I)
///     .nal_data(vec![0x00, 0x00, 0x01])
///     .offset(0)
///     .size(3)
///     .poc(0)
///     .frame_num(0)
///     .is_idr(true)
///     .is_irap(true)
///     .is_ref(true)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct HevcFrameBuilder {
    frame_index: Option<usize>,
    frame_type: Option<HevcFrameType>,
    nal_data: Option<Vec<u8>>,
    offset: Option<usize>,
    size: Option<usize>,
    poc: Option<i32>,
    frame_num: Option<u32>,
    is_idr: Option<bool>,
    is_irap: Option<bool>,
    is_ref: Option<bool>,
    temporal_id: Option<u8>,
    slice_header: Option<SliceHeader>,
}

impl HevcFrameBuilder {
    /// Set the frame index
    pub fn frame_index(mut self, value: usize) -> Self {
        self.frame_index = Some(value);
        self
    }

    /// Set the frame type
    pub fn frame_type(mut self, value: HevcFrameType) -> Self {
        self.frame_type = Some(value);
        self
    }

    /// Set the NAL data
    pub fn nal_data(mut self, value: Vec<u8>) -> Self {
        self.nal_data = Some(value);
        self
    }

    /// Set the offset in the stream
    pub fn offset(mut self, value: usize) -> Self {
        self.offset = Some(value);
        self
    }

    /// Set the frame size
    pub fn size(mut self, value: usize) -> Self {
        self.size = Some(value);
        self
    }

    /// Set the POC (Picture Order Count)
    pub fn poc(mut self, value: i32) -> Self {
        self.poc = Some(value);
        self
    }

    /// Set the frame number
    pub fn frame_num(mut self, value: u32) -> Self {
        self.frame_num = Some(value);
        self
    }

    /// Set whether this is an IDR frame
    pub fn is_idr(mut self, value: bool) -> Self {
        self.is_idr = Some(value);
        self
    }

    /// Set whether this is an IRAP frame
    pub fn is_irap(mut self, value: bool) -> Self {
        self.is_irap = Some(value);
        self
    }

    /// Set whether this is a reference frame
    pub fn is_ref(mut self, value: bool) -> Self {
        self.is_ref = Some(value);
        self
    }

    /// Set the temporal ID
    pub fn temporal_id(mut self, value: u8) -> Self {
        self.temporal_id = Some(value);
        self
    }

    /// Set the slice header
    pub fn slice_header(mut self, value: SliceHeader) -> Self {
        self.slice_header = Some(value);
        self
    }

    /// Build the HevcFrame
    ///
    /// # Panics
    ///
    /// Panics if required fields (frame_index, frame_type, offset, size, poc, frame_num, is_idr, is_irap, is_ref) are not set.
    pub fn build(self) -> HevcFrame {
        HevcFrame {
            frame_index: self.frame_index.expect("frame_index is required"),
            frame_type: self.frame_type.expect("frame_type is required"),
            nal_data: self.nal_data.unwrap_or_default(),
            offset: self.offset.expect("offset is required"),
            size: self.size.expect("size is required"),
            poc: self.poc.expect("poc is required"),
            frame_num: self.frame_num.expect("frame_num is required"),
            is_idr: self.is_idr.expect("is_idr is required"),
            is_irap: self.is_irap.expect("is_irap is required"),
            is_ref: self.is_ref.expect("is_ref is required"),
            temporal_id: self.temporal_id,
            slice_header: self.slice_header,
        }
    }
}

/// HEVC frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HevcFrameType {
    /// I-frame (IDR or non-IDR intra)
    I,
    /// P-frame (predicted)
    P,
    /// B-frame (bi-directional predicted)
    B,
    /// Unknown frame type
    Unknown,
}

impl HevcFrameType {
    /// Convert from slice type
    pub fn from_slice_type(slice_type: &str) -> Self {
        match slice_type {
            "I" => HevcFrameType::I,
            "P" => HevcFrameType::P,
            "B" => HevcFrameType::B,
            _ => HevcFrameType::Unknown,
        }
    }

    /// Get display string
    pub fn as_str(&self) -> &'static str {
        match self {
            HevcFrameType::I => "I",
            HevcFrameType::P => "P",
            HevcFrameType::B => "B",
            HevcFrameType::Unknown => "Unknown",
        }
    }
}

/// Extract frames from HEVC Annex B byte stream
///
/// This function parses the byte stream and groups NAL units into frames.
/// Each frame consists of one or more VCL NAL units (slice segments).
pub fn extract_annex_b_frames(data: &[u8]) -> Result<Vec<HevcFrame>, BitvueError> {
    // First, parse the full stream to get slice information
    let stream = parse_hevc(data).map_err(|e| BitvueError::Parse {
        offset: 0,
        message: e.to_string(),
    })?;

    // Find all NAL unit start positions
    let nal_positions = find_nal_units(data);

    if nal_positions.is_empty() {
        return Ok(Vec::new());
    }

    // find_nal_units already returns (start, end) tuples, so use them directly
    let nal_ranges = nal_positions;

    let mut frames = Vec::new();
    let mut current_frame_nals: Vec<(usize, usize)> = Vec::new();
    let mut current_frame_index = 0;
    let current_poc: Option<i32> = None;
    let mut current_frame_num: Option<u32> = None;
    let mut current_is_idr = false;
    let mut current_is_irap = false;
    let mut current_is_ref = false;
    let mut current_frame_type = HevcFrameType::Unknown;
    let mut current_temporal_id: Option<u8> = None;
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

        // Parse NAL header from NAL data (need at least 2 bytes for header)
        if nal_data_start + 2 > nal_end {
            continue;
        }
        let nal_header = match parse_nal_header(&data[nal_data_start..nal_end]) {
            Ok(header) => header,
            Err(_) => continue,
        };

        let nal_type = nal_header.nal_unit_type;

        // Check if this is a VCL NAL (Video Coding Layer)
        if nal_type.is_vcl() {
            // This is a slice segment
            let is_idr = nal_type.is_idr();
            let is_irap = nal_type.is_irap();
            let is_ref = nal_type.is_reference();
            let temporal_id = Some(nal_header.temporal_id());

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
                ) {
                    // New frame if first_slice_segment_in_pic_flag is set
                    new_slice.first_slice_segment_in_pic_flag
                        || new_slice.slice_pic_parameter_set_id != slice.slice_pic_parameter_set_id
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
                    current_is_irap,
                    current_is_ref,
                    current_frame_type,
                    current_temporal_id,
                    current_slice_header.clone(),
                ) {
                    frames.push(frame);
                }
                current_frame_index += 1;
                current_frame_nals.clear();
            }

            // Update frame state
            current_is_idr = is_idr;
            current_is_irap = is_irap;
            current_is_ref = is_ref;
            current_temporal_id = temporal_id;

            // Try to parse slice header for frame type
            if let Ok(slice) = crate::slice::parse_slice_header(
                &data[nal_data_start + 1..nal_end],
                &stream.sps_map,
                &stream.pps_map,
                nal_type,
            ) {
                if current_frame_nals.is_empty() {
                    current_frame_num = Some(slice.slice_pic_order_cnt_lsb as u32);
                    current_slice_header = Some(slice.clone());
                }

                // Determine frame type from slice type
                current_frame_type = HevcFrameType::from_slice_type(slice.slice_type.as_str());
            }

            current_frame_nals.push((nal_start, nal_end));
        } else if !current_frame_nals.is_empty() {
            // Non-VCL NAL after some VCL NALs
            if nal_type == NalUnitType::AudNut {
                // AUD definitely ends the current frame
                if let Some(frame) = build_frame_from_nals(
                    current_frame_index,
                    &current_frame_nals,
                    data,
                    current_poc.unwrap_or(0),
                    current_frame_num.unwrap_or(0),
                    current_is_idr,
                    current_is_irap,
                    current_is_ref,
                    current_frame_type,
                    current_temporal_id,
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
            current_is_irap,
            current_is_ref,
            current_frame_type,
            current_temporal_id,
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
    is_irap: bool,
    is_ref: bool,
    frame_type: HevcFrameType,
    temporal_id: Option<u8>,
    slice_header: Option<SliceHeader>,
) -> Option<HevcFrame> {
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

    Some(HevcFrame {
        frame_index,
        frame_type,
        nal_data,
        offset,
        size,
        poc,
        frame_num,
        is_idr,
        is_irap,
        is_ref,
        temporal_id,
        slice_header,
    })
}

/// Extract a single frame by index from Annex B byte stream
pub fn extract_frame_at_index(data: &[u8], frame_index: usize) -> Option<HevcFrame> {
    let frames = extract_annex_b_frames(data).ok()?;
    frames.get(frame_index).cloned()
}

/// Convert HevcFrame to UnitNode format for bitvue-core
pub fn hevc_frame_to_unit_node(frame: &HevcFrame, _stream_id: u8) -> bitvue_core::UnitNode {
    use bitvue_core::qp_extraction::QpData;

    // Extract QP from slice header if available
    let qp_avg = frame.slice_header.as_ref().and_then(|header| {
        // Note: This only includes slice_qp_delta, not init_qp_minus26
        // For accurate QP, we need access to PPS data
        Some(QpData::from_hevc_slice(26, header.slice_qp_delta as i32).qp_avg)
    }).flatten();

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
        qp_avg,
        mv_grid: None, // TODO: Extract from slice data
        temporal_id: frame.temporal_id,
        ref_frames: None, // TODO: Calculate from slice header
        ref_slots: None,
    }
}

/// Convert multiple HevcFrames to UnitNode format
pub fn hevc_frames_to_unit_nodes(frames: &[HevcFrame]) -> Vec<bitvue_core::UnitNode> {
    frames
        .iter()
        .map(|f| hevc_frame_to_unit_node(f, 0))
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
        assert_eq!(HevcFrameType::I.as_str(), "I");
        assert_eq!(HevcFrameType::P.as_str(), "P");
        assert_eq!(HevcFrameType::B.as_str(), "B");
    }

    #[test]
    fn test_frame_type_from_slice() {
        assert_eq!(HevcFrameType::from_slice_type("I"), HevcFrameType::I);
        assert_eq!(HevcFrameType::from_slice_type("P"), HevcFrameType::P);
        assert_eq!(HevcFrameType::from_slice_type("B"), HevcFrameType::B);
    }
}
