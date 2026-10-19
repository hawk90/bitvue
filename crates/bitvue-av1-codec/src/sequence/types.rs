//! The data an AV1 sequence header is made of: profile, timing, decoder model, operating points and `SequenceHeader`.

use serde::{Deserialize, Serialize};

use super::color::ColorConfig;

/// AV1 Profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Av1Profile {
    /// Main profile (0)
    Main,
    /// High profile (1)
    High,
    /// Professional profile (2)
    Professional,
    /// Reserved profile
    Reserved(u8),
}

impl Av1Profile {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Av1Profile::Main,
            1 => Av1Profile::High,
            2 => Av1Profile::Professional,
            v => Av1Profile::Reserved(v),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Av1Profile::Main => "Main",
            Av1Profile::High => "High",
            Av1Profile::Professional => "Professional",
            Av1Profile::Reserved(_) => "Reserved",
        }
    }
}

impl std::fmt::Display for Av1Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    pub num_units_in_display_tick: u32,
    pub time_scale: u32,
    pub equal_picture_interval: bool,
    pub num_ticks_per_picture: Option<u32>,
}

/// Decoder model info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoderModelInfo {
    pub buffer_delay_length_minus_1: u8,
    pub num_units_in_decoding_tick: u32,
    pub buffer_removal_time_length_minus_1: u8,
    pub frame_presentation_time_length_minus_1: u8,
}

/// Operating point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingPoint {
    pub idc: u16,
    pub seq_level_idx: u8,
    pub seq_tier: bool,
    pub decoder_model_present: bool,
    pub initial_display_delay: Option<u8>,
}

/// Parsed Sequence Header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceHeader {
    pub profile: Av1Profile,
    pub still_picture: bool,
    pub reduced_still_picture_header: bool,

    pub timing_info: Option<TimingInfo>,
    pub decoder_model_info: Option<DecoderModelInfo>,
    pub operating_points: Vec<OperatingPoint>,

    pub frame_width_bits_minus_1: u8,
    pub frame_height_bits_minus_1: u8,
    pub max_frame_width: u32,
    pub max_frame_height: u32,

    pub frame_id_numbers_present: bool,
    pub delta_frame_id_length_minus_2: Option<u8>,
    pub additional_frame_id_length_minus_1: Option<u8>,

    pub use_128x128_superblock: bool,
    pub enable_filter_intra: bool,
    pub enable_intra_edge_filter: bool,
    pub enable_interintra_compound: bool,
    pub enable_masked_compound: bool,
    pub enable_warped_motion: bool,
    pub enable_dual_filter: bool,
    pub enable_order_hint: bool,
    pub enable_jnt_comp: bool,
    pub enable_ref_frame_mvs: bool,
    pub seq_choose_screen_content_tools: bool,
    pub seq_force_screen_content_tools: u8,
    pub seq_choose_integer_mv: bool,
    pub seq_force_integer_mv: u8,
    pub order_hint_bits_minus_1: Option<u8>,

    pub enable_superres: bool,
    pub enable_cdef: bool,
    pub enable_restoration: bool,

    pub color_config: ColorConfig,
    pub film_grain_params_present: bool,
}

impl SequenceHeader {
    /// Returns the sequence level index from the first operating point
    pub fn level(&self) -> u8 {
        self.operating_points
            .first()
            .map(|op| op.seq_level_idx)
            .unwrap_or(0)
    }

    /// Returns the width
    pub fn width(&self) -> u32 {
        self.max_frame_width
    }

    /// Returns the height
    pub fn height(&self) -> u32 {
        self.max_frame_height
    }

    /// Returns the bit depth
    pub fn bit_depth(&self) -> u8 {
        self.color_config.bit_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_map_to_their_names_and_reserved_values_keep_the_number() {
        for (value, profile, name) in [
            (0u8, Av1Profile::Main, "Main"),
            (1, Av1Profile::High, "High"),
            (2, Av1Profile::Professional, "Professional"),
        ] {
            assert_eq!(Av1Profile::from_u8(value), profile);
            assert_eq!(profile.name(), name);
            assert_eq!(profile.to_string(), name, "Display is the name");
        }
        for value in [3u8, 7, 255] {
            let p = Av1Profile::from_u8(value);
            assert_eq!(p, Av1Profile::Reserved(value));
            assert_eq!(p.to_string(), "Reserved");
        }
    }
}
