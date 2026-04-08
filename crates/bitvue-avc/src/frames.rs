//! H.264/AVC frame extraction
//!
//! Functions for extracting individual frames from H.264 bitstreams

use crate::nal::{find_nal_units, parse_nal_header, NalUnitType};
use crate::parse_avc;
use crate::slice::{SliceHeader, SliceType};
use bitvue_core::BitvueError;
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
    /// Frame width in luma samples (0 if unknown)
    pub width: u32,
    /// Frame height in luma samples (0 if unknown)
    pub height: u32,
}

impl AvcFrame {
    /// Creates a new AvcFrameBuilder for constructing AvcFrame instances
    pub fn builder() -> AvcFrameBuilder {
        AvcFrameBuilder::default()
    }
}

/// Builder for constructing AvcFrame instances
///
/// # Example
///
/// ```
/// use bitvue_avc::frames::{AvcFrame, AvcFrameType};
///
/// let frame = AvcFrame::builder()
///     .frame_index(0)
///     .frame_type(AvcFrameType::I)
///     .nal_data(vec![0x00, 0x00, 0x01])
///     .offset(0)
///     .size(3)
///     .poc(0)
///     .frame_num(0)
///     .is_idr(true)
///     .is_ref(true)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct AvcFrameBuilder {
    frame_index: Option<usize>,
    frame_type: Option<AvcFrameType>,
    nal_data: Option<Vec<u8>>,
    offset: Option<usize>,
    size: Option<usize>,
    poc: Option<i32>,
    frame_num: Option<u32>,
    is_idr: Option<bool>,
    is_ref: Option<bool>,
    slice_header: Option<SliceHeader>,
    width: Option<u32>,
    height: Option<u32>,
}

impl AvcFrameBuilder {
    /// Set the frame index
    pub fn frame_index(mut self, value: usize) -> Self {
        self.frame_index = Some(value);
        self
    }

    /// Set the frame type
    pub fn frame_type(mut self, value: AvcFrameType) -> Self {
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

    /// Set whether this is a reference frame
    pub fn is_ref(mut self, value: bool) -> Self {
        self.is_ref = Some(value);
        self
    }

    /// Set the slice header
    pub fn slice_header(mut self, value: SliceHeader) -> Self {
        self.slice_header = Some(value);
        self
    }

    /// Set the frame width in luma samples
    pub fn width(mut self, value: u32) -> Self {
        self.width = Some(value);
        self
    }

    /// Set the frame height in luma samples
    pub fn height(mut self, value: u32) -> Self {
        self.height = Some(value);
        self
    }

    /// Build the AvcFrame
    ///
    /// Returns an error if required fields are not set.
    pub fn build(self) -> Result<AvcFrame, String> {
        Ok(AvcFrame {
            frame_index: self
                .frame_index
                .ok_or_else(|| "frame_index is required".to_string())?,
            frame_type: self
                .frame_type
                .ok_or_else(|| "frame_type is required".to_string())?,
            nal_data: self.nal_data.unwrap_or_default(),
            offset: self
                .offset
                .ok_or_else(|| "offset is required".to_string())?,
            size: self.size.ok_or_else(|| "size is required".to_string())?,
            poc: self.poc.ok_or_else(|| "poc is required".to_string())?,
            frame_num: self
                .frame_num
                .ok_or_else(|| "frame_num is required".to_string())?,
            is_idr: self
                .is_idr
                .ok_or_else(|| "is_idr is required".to_string())?,
            is_ref: self
                .is_ref
                .ok_or_else(|| "is_ref is required".to_string())?,
            slice_header: self.slice_header,
            width: self.width.unwrap_or(0),
            height: self.height.unwrap_or(0),
        })
    }
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
pub fn extract_annex_b_frames(data: &[u8]) -> Result<Vec<AvcFrame>, BitvueError> {
    // First, parse the full stream to get slice information
    let stream = parse_avc(data).map_err(|e| BitvueError::Parse {
        offset: 0,
        message: e.to_string(),
    })?;

    // Extract frame dimensions from the first SPS in the stream
    let (stream_width, stream_height) = stream
        .sps_map
        .values()
        .next()
        .map(|sps| (sps.pic_width(), sps.pic_height()))
        .unwrap_or((0, 0));

    // Find all NAL unit start positions
    let nal_positions = find_nal_units(data);

    if nal_positions.is_empty() {
        return Ok(Vec::new());
    }

    // Build NAL unit ranges (start, end) by pairing positions
    // OPTIMIZATION: Use iterator chain instead of manual indexing for better performance
    let nal_ranges: Vec<(usize, usize)> = nal_positions
        .iter()
        .zip(
            nal_positions
                .iter()
                .skip(1)
                .chain(std::iter::once(&data.len())),
        )
        .map(|(&start, &end)| {
            // Adjust for start code (include start code in range)
            let adjusted_start = start.saturating_sub(4);
            (adjusted_start, end)
        })
        .collect();

    let mut frames = Vec::new();
    let mut current_frame_nals: Vec<(usize, usize)> = Vec::new();
    let mut current_frame_index = 0;
    let _current_poc: Option<i32> = None;
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
            let new_frame = if current_frame_nals.is_empty() || is_idr != current_is_idr {
                true // First NAL or IDR boundary
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
                // OPTIMIZATION: Use take() to move the Option instead of cloning
                let slice_header = current_slice_header.take();
                if let Some(frame) = build_frame_from_nals(
                    current_frame_index,
                    &current_frame_nals,
                    data,
                    0,
                    current_frame_num.unwrap_or(0),
                    current_is_idr,
                    current_is_ref,
                    current_frame_type,
                    slice_header,
                    stream_width,
                    stream_height,
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
                    // OPTIMIZATION: Move the slice instead of cloning since we own it
                    current_slice_header = Some(slice);
                } else {
                    // Determine frame type from slice type without storing the header
                    current_frame_type = AvcFrameType::from_slice_type(slice.slice_type);
                }
            }

            current_frame_nals.push((nal_start, nal_end));
        } else if !current_frame_nals.is_empty() {
            // Non-VCL NAL after some VCL NALs
            if nal_type == NalUnitType::Aud {
                // AUD definitely ends the current frame
                // OPTIMIZATION: Use take() to move the Option instead of cloning
                let slice_header = current_slice_header.take();
                if let Some(frame) = build_frame_from_nals(
                    current_frame_index,
                    &current_frame_nals,
                    data,
                    0,
                    current_frame_num.unwrap_or(0),
                    current_is_idr,
                    current_is_ref,
                    current_frame_type,
                    slice_header,
                    stream_width,
                    stream_height,
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

        // OPTIMIZATION: Use take() to move the Option instead of cloning
        let slice_header = current_slice_header.take();
        if let Some(frame) = build_frame_from_nals(
            current_frame_index,
            &current_frame_nals,
            data,
            poc,
            current_frame_num.unwrap_or(0),
            current_is_idr,
            current_is_ref,
            current_frame_type,
            slice_header,
            stream_width,
            stream_height,
        ) {
            frames.push(frame);
        }
    }

    Ok(frames)
}

/// Build a frame from collected NAL unit positions
#[allow(clippy::too_many_arguments)]
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
    width: u32,
    height: u32,
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
        width,
        height,
    })
}

/// Extract a single frame by index from Annex B byte stream
pub fn extract_frame_at_index(data: &[u8], frame_index: usize) -> Option<AvcFrame> {
    let frames = extract_annex_b_frames(data).ok()?;
    frames.get(frame_index).cloned()
}

/// Build a minimal AVC Sps with only the dimension fields populated.
/// Used for overlay extraction when only the frame's stored dimensions are available.
fn build_minimal_avc_sps(width: u32, height: u32) -> crate::sps::Sps {
    use crate::sps::{ChromaFormat, ProfileIdc};
    // Derive MB counts from luma sample dimensions (each MB is 16x16).
    // Fall back to 1920x1080 (119, 67) when dimensions are unknown.
    let (pic_width_in_mbs_minus1, pic_height_in_map_units_minus1) = if width > 0 && height > 0 {
        (
            (width / 16).saturating_sub(1),
            (height / 16).saturating_sub(1),
        )
    } else {
        (119, 67) // 1920x1080 fallback
    };
    crate::sps::Sps {
        profile_idc: ProfileIdc::High,
        constraint_set0_flag: false,
        constraint_set1_flag: false,
        constraint_set2_flag: false,
        constraint_set3_flag: false,
        constraint_set4_flag: false,
        constraint_set5_flag: false,
        level_idc: 40,
        seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Yuv420,
        separate_colour_plane_flag: false,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        qpprime_y_zero_transform_bypass_flag: false,
        seq_scaling_matrix_present_flag: false,
        log2_max_frame_num_minus4: 0,
        pic_order_cnt_type: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        delta_pic_order_always_zero_flag: false,
        offset_for_non_ref_pic: 0,
        offset_for_top_to_bottom_field: 0,
        num_ref_frames_in_pic_order_cnt_cycle: 0,
        offset_for_ref_frame: vec![],
        max_num_ref_frames: 2,
        gaps_in_frame_num_value_allowed_flag: false,
        pic_width_in_mbs_minus1,
        pic_height_in_map_units_minus1,
        frame_mbs_only_flag: true,
        mb_adaptive_frame_field_flag: false,
        direct_8x8_inference_flag: true,
        frame_cropping_flag: false,
        frame_crop_left_offset: 0,
        frame_crop_right_offset: 0,
        frame_crop_top_offset: 0,
        frame_crop_bottom_offset: 0,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    }
}

/// Convert AvcFrame to UnitNode format for bitvue-core
pub fn avc_frame_to_unit_node(frame: &AvcFrame, _stream_id: u8) -> bitvue_core::UnitNode {
    use bitvue_core::qp_extraction::QpData;

    // Extract QP from slice header if available
    let qp_avg = frame
        .slice_header
        .as_ref()
        .and_then(|header| QpData::from_avc_slice(26, header.slice_qp_delta).qp_avg);

    // Motion vector grid: parse the frame's NAL units and use extract_mv_grid to get
    // per-macroblock motion vectors and prediction modes. A minimal Sps is synthesized
    // from the frame's stored dimensions (populated from the real SPS during extraction).
    let mv_grid = frame.slice_header.as_ref().and_then(|header| {
        if !header.slice_type.is_intra() {
            let sps = build_minimal_avc_sps(frame.width, frame.height);
            let nal_units = crate::nal::parse_nal_units(&frame.nal_data).unwrap_or_default();
            crate::overlay_extraction::extract_mv_grid(&nal_units, &sps).ok()
        } else {
            None
        }
    });

    // Derive reference frame slot indices from slice header
    let ref_frames = frame.slice_header.as_ref().and_then(|header| {
        if !header.slice_type.is_intra() {
            let l0 = header.num_ref_idx_l0_active_minus1 as usize + 1;
            let l1 = if header.slice_type.is_b() {
                header.num_ref_idx_l1_active_minus1 as usize + 1
            } else {
                0
            };
            Some((0..(l0 + l1)).collect::<Vec<usize>>())
        } else {
            None
        }
    });

    bitvue_core::UnitNode {
        key: bitvue_core::UnitKey {
            stream: bitvue_core::StreamId::A,
            unit_type: "FRAME".to_string(),
            offset: frame.offset as u64,
            size: frame.size,
        },
        unit_type: std::sync::Arc::from("FRAME"),
        offset: frame.offset as u64,
        size: frame.size,
        frame_index: Some(frame.frame_index),
        frame_type: Some(std::sync::Arc::from(frame.frame_type.as_str())),
        pts: Some(frame.poc as u64),
        dts: None,
        display_name: std::sync::Arc::from(format!(
            "Frame {} ({})",
            frame.frame_index,
            frame.frame_type.as_str()
        )),
        children: Vec::new(),
        qp_avg,
        mv_grid,
        temporal_id: None,
        ref_frames,
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
