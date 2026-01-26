//! MPEG-2 Video picture header and coding extension parsing.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Picture coding type (I, P, B, D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PictureType {
    /// Forbidden
    Forbidden = 0,
    /// I-picture (intra)
    #[default]
    I = 1,
    /// P-picture (predictive)
    P = 2,
    /// B-picture (bi-directional)
    B = 3,
    /// D-picture (DC intra-coded)
    D = 4,
}

impl PictureType {
    /// Create from raw value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => PictureType::Forbidden,
            1 => PictureType::I,
            2 => PictureType::P,
            3 => PictureType::B,
            4 => PictureType::D,
            _ => PictureType::Forbidden,
        }
    }

    /// Get name.
    pub fn name(&self) -> &'static str {
        match self {
            PictureType::Forbidden => "Forbidden",
            PictureType::I => "I",
            PictureType::P => "P",
            PictureType::B => "B",
            PictureType::D => "D",
        }
    }

    /// Check if intra-coded (I or D).
    pub fn is_intra(&self) -> bool {
        matches!(self, PictureType::I | PictureType::D)
    }
}

/// Picture structure (frame or field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PictureStructure {
    /// Reserved
    Reserved = 0,
    /// Top field
    TopField = 1,
    /// Bottom field
    BottomField = 2,
    /// Frame
    #[default]
    Frame = 3,
}

impl PictureStructure {
    /// Create from raw value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => PictureStructure::Reserved,
            1 => PictureStructure::TopField,
            2 => PictureStructure::BottomField,
            3 => PictureStructure::Frame,
            _ => PictureStructure::Reserved,
        }
    }

    /// Get name.
    pub fn name(&self) -> &'static str {
        match self {
            PictureStructure::Reserved => "Reserved",
            PictureStructure::TopField => "Top Field",
            PictureStructure::BottomField => "Bottom Field",
            PictureStructure::Frame => "Frame",
        }
    }
}

/// Picture header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PictureHeader {
    /// temporal_reference (10 bits)
    pub temporal_reference: u16,
    /// picture_coding_type (3 bits)
    pub picture_coding_type: PictureType,
    /// vbv_delay (16 bits)
    pub vbv_delay: u16,
    /// full_pel_forward_vector (for P and B pictures)
    pub full_pel_forward_vector: Option<bool>,
    /// forward_f_code (3 bits, for P and B pictures)
    pub forward_f_code: Option<u8>,
    /// full_pel_backward_vector (for B pictures)
    pub full_pel_backward_vector: Option<bool>,
    /// backward_f_code (3 bits, for B pictures)
    pub backward_f_code: Option<u8>,
    /// extra_bit_picture flags
    pub extra_information_count: u32,
}

/// Picture coding extension (MPEG-2 only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PictureCodingExtension {
    /// f_code[0][0] - forward horizontal
    pub f_code_00: u8,
    /// f_code[0][1] - forward vertical
    pub f_code_01: u8,
    /// f_code[1][0] - backward horizontal
    pub f_code_10: u8,
    /// f_code[1][1] - backward vertical
    pub f_code_11: u8,
    /// intra_dc_precision (2 bits)
    pub intra_dc_precision: u8,
    /// picture_structure (2 bits)
    pub picture_structure: PictureStructure,
    /// top_field_first
    pub top_field_first: bool,
    /// frame_pred_frame_dct
    pub frame_pred_frame_dct: bool,
    /// concealment_motion_vectors
    pub concealment_motion_vectors: bool,
    /// q_scale_type
    pub q_scale_type: bool,
    /// intra_vlc_format
    pub intra_vlc_format: bool,
    /// alternate_scan
    pub alternate_scan: bool,
    /// repeat_first_field
    pub repeat_first_field: bool,
    /// chroma_420_type
    pub chroma_420_type: bool,
    /// progressive_frame
    pub progressive_frame: bool,
    /// composite_display_flag
    pub composite_display_flag: bool,
    /// v_axis (if composite_display_flag)
    pub v_axis: Option<bool>,
    /// field_sequence (if composite_display_flag)
    pub field_sequence: Option<u8>,
    /// sub_carrier (if composite_display_flag)
    pub sub_carrier: Option<bool>,
    /// burst_amplitude (if composite_display_flag)
    pub burst_amplitude: Option<u8>,
    /// sub_carrier_phase (if composite_display_flag)
    pub sub_carrier_phase: Option<u8>,
}

impl PictureCodingExtension {
    /// Get intra DC precision in bits (8-11).
    pub fn intra_dc_precision_bits(&self) -> u8 {
        8 + self.intra_dc_precision
    }
}

/// Parse picture header from data after start code.
pub fn parse_picture_header(data: &[u8]) -> Result<PictureHeader> {
    let mut reader = BitReader::new(data);

    let temporal_reference = reader.read_bits(10)? as u16;
    let picture_coding_type = PictureType::from_u8(reader.read_bits(3)? as u8);
    let vbv_delay = reader.read_bits(16)? as u16;

    let mut full_pel_forward_vector = None;
    let mut forward_f_code = None;
    let mut full_pel_backward_vector = None;
    let mut backward_f_code = None;

    // P and B pictures have forward motion vectors
    if matches!(picture_coding_type, PictureType::P | PictureType::B) {
        full_pel_forward_vector = Some(reader.read_flag()?);
        forward_f_code = Some(reader.read_bits(3)? as u8);
    }

    // B pictures have backward motion vectors
    if matches!(picture_coding_type, PictureType::B) {
        full_pel_backward_vector = Some(reader.read_flag()?);
        backward_f_code = Some(reader.read_bits(3)? as u8);
    }

    // Skip extra_bit_picture data
    let mut extra_information_count = 0;
    while reader.has_more_data() && reader.read_flag()? {
        let _extra_information_picture = reader.read_bits(8)?;
        extra_information_count += 1;
    }

    Ok(PictureHeader {
        temporal_reference,
        picture_coding_type,
        vbv_delay,
        full_pel_forward_vector,
        forward_f_code,
        full_pel_backward_vector,
        backward_f_code,
        extra_information_count,
    })
}

/// Parse picture coding extension from data after extension start code.
pub fn parse_picture_coding_extension(data: &[u8]) -> Result<PictureCodingExtension> {
    let mut reader = BitReader::new(data);

    // extension_start_code_identifier (4 bits) - should be 8
    let _ext_id = reader.read_bits(4)?;

    let f_code_00 = reader.read_bits(4)? as u8;
    let f_code_01 = reader.read_bits(4)? as u8;
    let f_code_10 = reader.read_bits(4)? as u8;
    let f_code_11 = reader.read_bits(4)? as u8;
    let intra_dc_precision = reader.read_bits(2)? as u8;
    let picture_structure = PictureStructure::from_u8(reader.read_bits(2)? as u8);
    let top_field_first = reader.read_flag()?;
    let frame_pred_frame_dct = reader.read_flag()?;
    let concealment_motion_vectors = reader.read_flag()?;
    let q_scale_type = reader.read_flag()?;
    let intra_vlc_format = reader.read_flag()?;
    let alternate_scan = reader.read_flag()?;
    let repeat_first_field = reader.read_flag()?;
    let chroma_420_type = reader.read_flag()?;
    let progressive_frame = reader.read_flag()?;
    let composite_display_flag = reader.read_flag()?;

    let (v_axis, field_sequence, sub_carrier, burst_amplitude, sub_carrier_phase) =
        if composite_display_flag {
            (
                Some(reader.read_flag()?),
                Some(reader.read_bits(3)? as u8),
                Some(reader.read_flag()?),
                Some(reader.read_bits(7)? as u8),
                Some(reader.read_bits(8)? as u8),
            )
        } else {
            (None, None, None, None, None)
        };

    Ok(PictureCodingExtension {
        f_code_00,
        f_code_01,
        f_code_10,
        f_code_11,
        intra_dc_precision,
        picture_structure,
        top_field_first,
        frame_pred_frame_dct,
        concealment_motion_vectors,
        q_scale_type,
        intra_vlc_format,
        alternate_scan,
        repeat_first_field,
        chroma_420_type,
        progressive_frame,
        composite_display_flag,
        v_axis,
        field_sequence,
        sub_carrier,
        burst_amplitude,
        sub_carrier_phase,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_picture_type() {
        assert_eq!(PictureType::from_u8(1), PictureType::I);
        assert_eq!(PictureType::from_u8(2), PictureType::P);
        assert_eq!(PictureType::from_u8(3), PictureType::B);

        assert!(PictureType::I.is_intra());
        assert!(!PictureType::P.is_intra());
        assert!(!PictureType::B.is_intra());
    }

    #[test]
    fn test_picture_structure() {
        assert_eq!(PictureStructure::from_u8(3), PictureStructure::Frame);
        assert_eq!(PictureStructure::from_u8(1), PictureStructure::TopField);
    }
}
