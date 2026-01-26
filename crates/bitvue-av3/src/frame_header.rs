//! Frame Header parsing for AV3.

use crate::bitreader::BitReader;
use crate::error::{Av3Error, Result};
use serde::{Deserialize, Serialize};

/// Frame types for AV3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameType {
    /// Key frame
    KeyFrame = 0,
    /// Inter frame
    InterFrame = 1,
    /// Intra only frame
    IntraOnlyFrame = 2,
    /// Switch frame (AV3 addition)
    SwitchFrame = 3,
    /// Show existing frame
    ShowExistingFrame = 4,
}

impl FrameType {
    /// Check if this is a key frame.
    pub fn is_key_frame(&self) -> bool {
        matches!(self, FrameType::KeyFrame)
    }

    /// Check if this is an intra frame.
    pub fn is_intra(&self) -> bool {
        matches!(self, FrameType::KeyFrame | FrameType::IntraOnlyFrame)
    }

    /// Check if this is an inter frame.
    pub fn is_inter(&self) -> bool {
        matches!(self, FrameType::InterFrame)
    }
}

/// Primary reference frame for AV3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimaryRefFrame {
    /// Primary reference frame index
    pub index: u8,
}

impl PrimaryRefFrame {
    /// Create new primary ref frame.
    pub fn new(index: u8) -> Self {
        Self { index }
    }

    /// None (no primary reference)
    pub fn none() -> Self {
        Self { index: 7 }
    }
}

/// Reference frame info.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceFrame {
    /// Reference frame index
    pub index: u8,
    /// Reference frame type
    pub frame_type: FrameType,
    /// Reference frame width
    pub width: u32,
    /// Reference frame height
    pub height: u32,
}

/// Frame Header for AV3.
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Frame type
    pub frame_type: FrameType,
    /// Frame is shown
    pub show_frame: bool,
    /// Frame size
    pub width: u32,
    pub height: u32,
    /// Render size
    pub render_width: u32,
    pub render_height: u32,
    /// Super block size (64 or 128)
    pub sb_size: u8,
    /// Frame ID
    pub frame_id: u64,
    /// Current frame context
    pub current_frame_id: u64,
    /// Primary reference frame
    pub primary_ref_frame: PrimaryRefFrame,
    /// Reference frames
    pub ref_frames: [Option<ReferenceFrame>; 8],
    /// Order hint
    pub order_hint: u32,
    /// Global motion params (simplified)
    pub global_motion: [bool; 8],
    /// Loop filter level
    pub loop_filter_level: u8,
    /// Loop filter sharpness
    pub loop_filter_sharpness: u8,
    /// CDEF damping
    pub cdef_damping: u8,
    /// CDEF bits
    pub cdef_bits: u8,
    /// Quantization base index
    pub base_q_idx: u8,
    /// Delta Q present
    pub delta_q_present: bool,
    /// Delta LF present
    pub delta_lf_present: bool,
    /// QP Y DC delta
    pub qp_y_dc_delta_q: i8,
    /// QP U DC delta
    pub qp_u_dc_delta_q: i8,
    /// QP U AC delta
    pub qp_u_ac_delta_q: i8,
    /// QP V DC delta
    pub qp_v_dc_delta_q: i8,
    /// QP V AC delta
    pub qp_v_ac_delta_q: i8,
    /// CDEF Y strength
    pub cdef_y_strength: u8,
    /// CDEF UV strength
    pub cdef_uv_strength: u8,
    /// Segmentation enabled
    pub segmentation_enabled: bool,
    /// AV3 features
    pub enable_superres: bool,
    pub enable_restoration: bool,
    pub enable_cdef: bool,
}

impl Default for FrameHeader {
    fn default() -> Self {
        Self {
            frame_type: FrameType::InterFrame,
            show_frame: true,
            width: 1920,
            height: 1080,
            render_width: 1920,
            render_height: 1080,
            sb_size: 128,
            frame_id: 0,
            current_frame_id: 0,
            primary_ref_frame: PrimaryRefFrame::none(),
            ref_frames: [None; 8],
            order_hint: 0,
            global_motion: [false; 8],
            loop_filter_level: 0,
            loop_filter_sharpness: 0,
            cdef_damping: 3,
            cdef_bits: 0,
            base_q_idx: 32,
            delta_q_present: false,
            delta_lf_present: false,
            qp_y_dc_delta_q: 0,
            qp_u_dc_delta_q: 0,
            qp_u_ac_delta_q: 0,
            qp_v_dc_delta_q: 0,
            qp_v_ac_delta_q: 0,
            cdef_y_strength: 0,
            cdef_uv_strength: 0,
            segmentation_enabled: false,
            enable_superres: false,
            enable_restoration: true,
            enable_cdef: true,
        }
    }
}

/// Parse frame header from OBU payload.
pub fn parse_frame_header(data: &[u8]) -> Result<FrameHeader> {
    let mut reader = BitReader::new(data);

    // Read frame type (2 bits)
    let frame_type_raw = reader.read_bits(2)?;
    let frame_type = match frame_type_raw {
        0 => FrameType::KeyFrame,
        1 => FrameType::InterFrame,
        2 => FrameType::IntraOnlyFrame,
        3 => FrameType::ShowExistingFrame,
        _ => return Err(Av3Error::InvalidData("Invalid frame type".to_string())),
    };

    let show_frame = reader.read_bit()?;

    // Use defaults for rest (simplified parsing)
    Ok(FrameHeader {
        frame_type,
        show_frame,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type() {
        assert!(FrameType::KeyFrame.is_key_frame());
        assert!(FrameType::KeyFrame.is_intra());
        assert!(!FrameType::KeyFrame.is_inter());

        assert!(FrameType::InterFrame.is_inter());
        assert!(!FrameType::InterFrame.is_intra());
    }

    #[test]
    fn test_primary_ref_frame() {
        let pr = PrimaryRefFrame::new(0);
        assert_eq!(pr.index, 0);

        let pr = PrimaryRefFrame::none();
        assert_eq!(pr.index, 7);
    }
}
