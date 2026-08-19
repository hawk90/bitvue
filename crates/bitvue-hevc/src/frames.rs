//! HEVC/H.265 frame extraction
//!
//! Functions for extracting individual frames from HEVC bitstreams

use crate::nal::{find_nal_units, parse_nal_header, NalUnitType};
use crate::parse_hevc;
use crate::slice::SliceHeader;
use bitvue_engine::BitvueError;
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
    /// Frame width in luma samples (0 if unknown)
    pub width: u32,
    /// Frame height in luma samples (0 if unknown)
    pub height: u32,
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
    width: Option<u32>,
    height: Option<u32>,
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
            width: self.width.unwrap_or(0),
            height: self.height.unwrap_or(0),
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

    // Extract frame dimensions from the first SPS in the stream
    let (stream_width, stream_height) = stream
        .sps_map
        .values()
        .next()
        .map(|sps| {
            (
                sps.pic_width_in_luma_samples,
                sps.pic_height_in_luma_samples,
            )
        })
        .unwrap_or((0, 0));

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
    let mut current_poc: i32 = 0;
    let mut current_frame_num: Option<u32> = None;
    let mut current_is_idr = false;
    let mut current_is_irap = false;
    let mut current_is_ref = false;
    let mut current_frame_type = HevcFrameType::Unknown;
    let mut current_temporal_id: Option<u8> = None;
    let mut current_slice_header: Option<SliceHeader> = None;

    // POC unwrapping state (H.265 spec §8.3.1)
    let mut prev_poc_msb: i32 = 0;
    let mut prev_poc_lsb: i32 = 0;

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

            // Try to read first_slice_segment_in_pic_flag directly from the NAL data
            // This is the first bit after the NAL header (2 bytes)
            let first_slice_flag = if nal_data_start + 3 <= nal_end {
                // NAL header is 2 bytes, then first_slice_segment_in_pic_flag is the first bit
                data[nal_data_start + 2] & 0x80 != 0
            } else {
                true // Assume new frame if we can't read the flag
            };

            // Check if this starts a new frame
            let new_frame =
                if current_frame_nals.is_empty() || is_idr != current_is_idr || first_slice_flag {
                    true // First VCL NAL, IDR boundary, or first_slice flag
                } else if let Some(slice) = &current_slice_header {
                    // Try to parse slice header for additional checks
                    if let Ok(new_slice) = crate::slice::parse_slice_header(
                        &data[nal_data_start + 1..nal_end],
                        &stream.sps_map,
                        &stream.pps_map,
                        nal_type,
                    ) {
                        new_slice.slice_pic_parameter_set_id != slice.slice_pic_parameter_set_id
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
                    current_poc,
                    current_frame_num.unwrap_or(0),
                    current_is_idr,
                    current_is_irap,
                    current_is_ref,
                    current_frame_type,
                    current_temporal_id,
                    current_slice_header.clone(),
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
            current_is_irap = is_irap;
            current_is_ref = is_ref;
            current_temporal_id = temporal_id;

            // Try to parse slice header for frame type and POC
            if let Ok(slice) = crate::slice::parse_slice_header(
                &data[nal_data_start + 1..nal_end],
                &stream.sps_map,
                &stream.pps_map,
                nal_type,
            ) {
                if current_frame_nals.is_empty() {
                    current_frame_num = Some(slice.slice_pic_order_cnt_lsb);
                    current_slice_header = Some(slice.clone());

                    // Calculate POC per H.265 spec §8.3.1
                    current_poc = if is_idr {
                        // IDR resets POC to 0
                        prev_poc_msb = 0;
                        prev_poc_lsb = 0;
                        0
                    } else if is_irap {
                        // Other IRAP frames: POC = poc_lsb (no MSB wrap)
                        let poc_lsb = slice.slice_pic_order_cnt_lsb as i32;
                        prev_poc_msb = 0;
                        prev_poc_lsb = poc_lsb;
                        poc_lsb
                    } else {
                        // Non-IRAP: unwrap poc_lsb using MaxPicOrderCntLsb from SPS
                        let sps = stream
                            .pps_map
                            .get(&slice.slice_pic_parameter_set_id)
                            .and_then(|pps| stream.sps_map.get(&pps.pps_seq_parameter_set_id));

                        if let Some(sps) = sps {
                            let max_poc_lsb =
                                1i32 << (sps.log2_max_pic_order_cnt_lsb_minus4 as i32 + 4);
                            let poc_lsb = slice.slice_pic_order_cnt_lsb as i32;

                            let poc_msb = if poc_lsb < prev_poc_lsb
                                && (prev_poc_lsb - poc_lsb) >= (max_poc_lsb / 2)
                            {
                                prev_poc_msb + max_poc_lsb
                            } else if poc_lsb > prev_poc_lsb
                                && (poc_lsb - prev_poc_lsb) > (max_poc_lsb / 2)
                            {
                                prev_poc_msb - max_poc_lsb
                            } else {
                                prev_poc_msb
                            };

                            // Only reference frames update the prev POC state
                            if is_ref {
                                prev_poc_msb = poc_msb;
                                prev_poc_lsb = poc_lsb;
                            }

                            poc_msb + poc_lsb
                        } else {
                            // SPS not found — fall back to raw lsb
                            slice.slice_pic_order_cnt_lsb as i32
                        }
                    };
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
                    current_poc,
                    current_frame_num.unwrap_or(0),
                    current_is_idr,
                    current_is_irap,
                    current_is_ref,
                    current_frame_type,
                    current_temporal_id,
                    current_slice_header.clone(),
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
        if let Some(frame) = build_frame_from_nals(
            current_frame_index,
            &current_frame_nals,
            data,
            current_poc,
            current_frame_num.unwrap_or(0),
            current_is_idr,
            current_is_irap,
            current_is_ref,
            current_frame_type,
            current_temporal_id,
            current_slice_header.clone(),
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
    is_irap: bool,
    is_ref: bool,
    frame_type: HevcFrameType,
    temporal_id: Option<u8>,
    slice_header: Option<SliceHeader>,
    width: u32,
    height: u32,
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
        width,
        height,
    })
}

/// Extract a single frame by index from Annex B byte stream
pub fn extract_frame_at_index(data: &[u8], frame_index: usize) -> Option<HevcFrame> {
    let frames = extract_annex_b_frames(data).ok()?;
    frames.get(frame_index).cloned()
}

/// Build a minimal Sps with only the dimension fields populated.
/// Used for overlay extraction when only the frame's stored dimensions are available
/// (the real SPS is at stream level, not per-frame).
fn build_minimal_sps(width: u32, height: u32) -> crate::sps::Sps {
    use crate::sps::{ChromaFormat, Profile, ProfileTierLevel};
    crate::sps::Sps {
        sps_video_parameter_set_id: 0,
        sps_max_sub_layers_minus1: 0,
        sps_temporal_id_nesting_flag: true,
        profile_tier_level: ProfileTierLevel {
            general_profile_space: 0,
            general_tier_flag: false,
            general_profile_idc: Profile::Main,
            general_profile_compatibility_flags: 0,
            general_progressive_source_flag: true,
            general_interlaced_source_flag: false,
            general_non_packed_constraint_flag: true,
            general_frame_only_constraint_flag: true,
            general_level_idc: 0,
        },
        sps_seq_parameter_set_id: 0,
        chroma_format_idc: ChromaFormat::Chroma420,
        separate_colour_plane_flag: false,
        pic_width_in_luma_samples: width,
        pic_height_in_luma_samples: height,
        conformance_window_flag: false,
        conf_win_left_offset: 0,
        conf_win_right_offset: 0,
        conf_win_top_offset: 0,
        conf_win_bottom_offset: 0,
        bit_depth_luma_minus8: 0,
        bit_depth_chroma_minus8: 0,
        log2_max_pic_order_cnt_lsb_minus4: 0,
        sps_sub_layer_ordering_info_present_flag: false,
        sps_max_dec_pic_buffering_minus1: vec![0],
        sps_max_num_reorder_pics: vec![0],
        sps_max_latency_increase_plus1: vec![0],
        log2_min_luma_coding_block_size_minus3: 0,
        log2_diff_max_min_luma_coding_block_size: 0,
        log2_min_luma_transform_block_size_minus2: 0,
        log2_diff_max_min_luma_transform_block_size: 0,
        max_transform_hierarchy_depth_inter: 0,
        max_transform_hierarchy_depth_intra: 0,
        scaling_list_enabled_flag: false,
        amp_enabled_flag: false,
        sample_adaptive_offset_enabled_flag: false,
        pcm_enabled_flag: false,
        num_short_term_ref_pic_sets: 0,
        long_term_ref_pics_present_flag: false,
        num_long_term_ref_pics_sps: 0,
        sps_temporal_mvp_enabled_flag: false,
        strong_intra_smoothing_enabled_flag: false,
        vui_parameters_present_flag: false,
        vui_parameters: None,
    }
}

/// Convert HevcFrame to UnitNode format for bitvue-engine
pub fn hevc_frame_to_unit_node(frame: &HevcFrame, _stream_id: u8) -> bitvue_engine::UnitNode {
    use bitvue_engine::qp_extraction::QpData;

    // Extract QP from slice header if available
    let qp_avg = frame
        .slice_header
        .as_ref()
        .and_then(|header| QpData::from_hevc_slice(26, header.slice_qp_delta as i32).qp_avg);

    // POC calculation: use slice_pic_order_cnt_lsb directly as a basic POC value.
    // Full calculation would require MaxPicOrderCntLsb from SPS (2^(log2_max_pic_order_cnt_lsb_minus4+4))
    // to unwrap the modular arithmetic, but the LSB value is correct for streams that
    // don't wrap (poc_lsb < MaxPicOrderCntLsb/2) and for IDR frames (poc = 0).
    let poc = frame
        .slice_header
        .as_ref()
        .map(|h| h.slice_pic_order_cnt_lsb as i32)
        .unwrap_or(frame.poc);

    // Motion vector grid: parse the frame's NAL units and use extract_mv_grid to get
    // per-block motion vectors and prediction modes. A minimal Sps is synthesized from
    // the frame's stored dimensions (populated from the real SPS during extraction).
    let mv_grid = frame.slice_header.as_ref().and_then(|header| {
        if header.slice_type.is_inter() {
            let coded_w = if frame.width > 0 { frame.width } else { 1920 };
            let coded_h = if frame.height > 0 { frame.height } else { 1080 };
            let sps = build_minimal_sps(coded_w, coded_h);
            let nal_units = crate::nal::parse_nal_units(&frame.nal_data).unwrap_or_default();
            crate::overlay_extraction::extract_mv_grid(&nal_units, &sps).ok()
        } else {
            None
        }
    });

    // Reference frame slots: derive from the number of active reference indices in the
    // slice header. L0 list is active for P and B slices; L1 list is active only for B.
    let ref_frames = frame.slice_header.as_ref().and_then(|header| {
        if header.slice_type.is_inter() {
            let l0_count = header.num_ref_idx_l0_active_minus1 as usize + 1;
            let l1_count = if header.slice_type == crate::slice::SliceType::B {
                header.num_ref_idx_l1_active_minus1 as usize + 1
            } else {
                0
            };
            // Return slot indices 0..total as a proxy for the referenced frame indices
            let total = l0_count + l1_count;
            Some((0..total).collect::<Vec<usize>>())
        } else {
            None
        }
    });

    bitvue_engine::UnitNode {
        key: bitvue_engine::UnitKey {
            stream: bitvue_engine::StreamId::A,
            unit_type: "FRAME".to_string(),
            offset: frame.offset as u64,
            size: frame.size,
        },
        unit_type: std::sync::Arc::from("FRAME"),
        offset: frame.offset as u64,
        size: frame.size,
        frame_index: Some(frame.frame_index),
        frame_type: Some(std::sync::Arc::from(frame.frame_type.as_str())),
        pts: Some(poc as u64),
        dts: None,
        display_name: std::sync::Arc::from(format!(
            "Frame {} ({})",
            frame.frame_index,
            frame.frame_type.as_str()
        )),
        children: Vec::new(),
        qp_avg,
        mv_grid,
        temporal_id: frame.temporal_id,
        ref_frames,
        ref_slots: None,
    }
}

/// Convert multiple HevcFrames to UnitNode format
pub fn hevc_frames_to_unit_nodes(frames: &[HevcFrame]) -> Vec<bitvue_engine::UnitNode> {
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
