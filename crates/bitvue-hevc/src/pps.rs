//! HEVC Picture Parameter Set (PPS) parsing.
//!
//! PPS contains picture-level parameters referenced by slice headers.
//! It is defined in ITU-T H.265 Section 7.3.2.3.

use crate::bitreader::BitReader;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// HEVC Picture Parameter Set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pps {
    /// PPS ID (0-63).
    pub pps_pic_parameter_set_id: u8,
    /// Referenced SPS ID (0-15).
    pub pps_seq_parameter_set_id: u8,
    /// Dependent slice segments enabled.
    pub dependent_slice_segments_enabled_flag: bool,
    /// Output flag present in slice header.
    pub output_flag_present_flag: bool,
    /// Number of extra slice header bits.
    pub num_extra_slice_header_bits: u8,
    /// Sign data hiding enabled.
    pub sign_data_hiding_enabled_flag: bool,
    /// CABAC init present in slice header.
    pub cabac_init_present_flag: bool,
    /// Number of reference pictures in list 0 default.
    pub num_ref_idx_l0_default_active_minus1: u8,
    /// Number of reference pictures in list 1 default.
    pub num_ref_idx_l1_default_active_minus1: u8,
    /// Initial QP value.
    pub init_qp_minus26: i8,
    /// Constrained intra prediction.
    pub constrained_intra_pred_flag: bool,
    /// Transform skip enabled.
    pub transform_skip_enabled_flag: bool,
    /// CU QP delta enabled.
    pub cu_qp_delta_enabled_flag: bool,
    /// Diff CU QP delta depth.
    pub diff_cu_qp_delta_depth: u8,
    /// CB QP offset.
    pub pps_cb_qp_offset: i8,
    /// CR QP offset.
    pub pps_cr_qp_offset: i8,
    /// Slice chroma QP offsets present.
    pub pps_slice_chroma_qp_offsets_present_flag: bool,
    /// Weighted prediction enabled.
    pub weighted_pred_flag: bool,
    /// Weighted biprediction enabled.
    pub weighted_bipred_flag: bool,
    /// Transquant bypass enabled.
    pub transquant_bypass_enabled_flag: bool,
    /// Tiles enabled.
    pub tiles_enabled_flag: bool,
    /// Entropy coding sync enabled (WPP).
    pub entropy_coding_sync_enabled_flag: bool,
    /// Tile configuration.
    pub tile_config: Option<TileConfig>,
    /// Loop filter across tiles enabled.
    pub loop_filter_across_tiles_enabled_flag: bool,
    /// Loop filter across slices enabled.
    pub pps_loop_filter_across_slices_enabled_flag: bool,
    /// Deblocking filter control present.
    pub deblocking_filter_control_present_flag: bool,
    /// Deblocking filter override enabled.
    pub deblocking_filter_override_enabled_flag: bool,
    /// Deblocking filter disabled.
    pub pps_deblocking_filter_disabled_flag: bool,
    /// Beta offset div 2.
    pub pps_beta_offset_div2: i8,
    /// TC offset div 2.
    pub pps_tc_offset_div2: i8,
    /// Scaling list data present.
    pub pps_scaling_list_data_present_flag: bool,
    /// Lists modification present.
    pub lists_modification_present_flag: bool,
    /// Log2 parallel merge level.
    pub log2_parallel_merge_level_minus2: u8,
    /// Slice segment header extension present.
    pub slice_segment_header_extension_present_flag: bool,
    /// PPS extension present.
    pub pps_extension_present_flag: bool,
    /// PPS range extension flag.
    pub pps_range_extension_flag: bool,
    /// PPS multilayer extension flag.
    pub pps_multilayer_extension_flag: bool,
    /// PPS 3D extension flag.
    pub pps_3d_extension_flag: bool,
    /// PPS SCC extension flag.
    pub pps_scc_extension_flag: bool,
}

/// Tile configuration from PPS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TileConfig {
    /// Number of tile columns minus 1.
    pub num_tile_columns_minus1: u16,
    /// Number of tile rows minus 1.
    pub num_tile_rows_minus1: u16,
    /// Uniform tile spacing.
    pub uniform_spacing_flag: bool,
    /// Column widths (in CTBs) if not uniform.
    pub column_width_minus1: Vec<u16>,
    /// Row heights (in CTBs) if not uniform.
    pub row_height_minus1: Vec<u16>,
}

impl TileConfig {
    /// Get number of tile columns.
    pub fn num_columns(&self) -> u16 {
        self.num_tile_columns_minus1 + 1
    }

    /// Get number of tile rows.
    pub fn num_rows(&self) -> u16 {
        self.num_tile_rows_minus1 + 1
    }

    /// Get total number of tiles.
    pub fn num_tiles(&self) -> u32 {
        self.num_columns() as u32 * self.num_rows() as u32
    }
}

impl Default for Pps {
    fn default() -> Self {
        Self {
            pps_pic_parameter_set_id: 0,
            pps_seq_parameter_set_id: 0,
            dependent_slice_segments_enabled_flag: false,
            output_flag_present_flag: false,
            num_extra_slice_header_bits: 0,
            sign_data_hiding_enabled_flag: false,
            cabac_init_present_flag: false,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
            init_qp_minus26: 0,
            constrained_intra_pred_flag: false,
            transform_skip_enabled_flag: false,
            cu_qp_delta_enabled_flag: false,
            diff_cu_qp_delta_depth: 0,
            pps_cb_qp_offset: 0,
            pps_cr_qp_offset: 0,
            pps_slice_chroma_qp_offsets_present_flag: false,
            weighted_pred_flag: false,
            weighted_bipred_flag: false,
            transquant_bypass_enabled_flag: false,
            tiles_enabled_flag: false,
            entropy_coding_sync_enabled_flag: false,
            tile_config: None,
            loop_filter_across_tiles_enabled_flag: true,
            pps_loop_filter_across_slices_enabled_flag: false,
            deblocking_filter_control_present_flag: false,
            deblocking_filter_override_enabled_flag: false,
            pps_deblocking_filter_disabled_flag: false,
            pps_beta_offset_div2: 0,
            pps_tc_offset_div2: 0,
            pps_scaling_list_data_present_flag: false,
            lists_modification_present_flag: false,
            log2_parallel_merge_level_minus2: 0,
            slice_segment_header_extension_present_flag: false,
            pps_extension_present_flag: false,
            pps_range_extension_flag: false,
            pps_multilayer_extension_flag: false,
            pps_3d_extension_flag: false,
            pps_scc_extension_flag: false,
        }
    }
}

impl Pps {
    /// Get initial QP value (26 + init_qp_minus26).
    pub fn init_qp(&self) -> i8 {
        26 + self.init_qp_minus26
    }

    /// Check if tiles are used.
    pub fn has_tiles(&self) -> bool {
        self.tiles_enabled_flag && self.tile_config.is_some()
    }

    /// Get number of tiles if enabled.
    pub fn num_tiles(&self) -> Option<u32> {
        self.tile_config.as_ref().map(|tc| tc.num_tiles())
    }

    /// Check if WPP (Wavefront Parallel Processing) is enabled.
    pub fn wpp_enabled(&self) -> bool {
        self.entropy_coding_sync_enabled_flag
    }
}

/// Parse PPS from RBSP data.
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pps(data: &[u8]) -> Result<Pps> {
    let mut reader = BitReader::new(data);
    let mut pps = Pps::default();

    // pps_pic_parameter_set_id (ue(v))
    pps.pps_pic_parameter_set_id = reader.read_ue()? as u8;

    // pps_seq_parameter_set_id (ue(v))
    pps.pps_seq_parameter_set_id = reader.read_ue()? as u8;

    // dependent_slice_segments_enabled_flag (1 bit)
    pps.dependent_slice_segments_enabled_flag = reader.read_bit()?;

    // output_flag_present_flag (1 bit)
    pps.output_flag_present_flag = reader.read_bit()?;

    // num_extra_slice_header_bits (3 bits)
    pps.num_extra_slice_header_bits = reader.read_bits(3)? as u8;

    // sign_data_hiding_enabled_flag (1 bit)
    pps.sign_data_hiding_enabled_flag = reader.read_bit()?;

    // cabac_init_present_flag (1 bit)
    pps.cabac_init_present_flag = reader.read_bit()?;

    // num_ref_idx_l0_default_active_minus1 (ue(v))
    pps.num_ref_idx_l0_default_active_minus1 = reader.read_ue()? as u8;

    // num_ref_idx_l1_default_active_minus1 (ue(v))
    pps.num_ref_idx_l1_default_active_minus1 = reader.read_ue()? as u8;

    // init_qp_minus26 (se(v))
    pps.init_qp_minus26 = reader.read_se()? as i8;

    // constrained_intra_pred_flag (1 bit)
    pps.constrained_intra_pred_flag = reader.read_bit()?;

    // transform_skip_enabled_flag (1 bit)
    pps.transform_skip_enabled_flag = reader.read_bit()?;

    // cu_qp_delta_enabled_flag (1 bit)
    pps.cu_qp_delta_enabled_flag = reader.read_bit()?;

    if pps.cu_qp_delta_enabled_flag {
        // diff_cu_qp_delta_depth (ue(v))
        pps.diff_cu_qp_delta_depth = reader.read_ue()? as u8;
    }

    // pps_cb_qp_offset (se(v))
    pps.pps_cb_qp_offset = reader.read_se()? as i8;

    // pps_cr_qp_offset (se(v))
    pps.pps_cr_qp_offset = reader.read_se()? as i8;

    // pps_slice_chroma_qp_offsets_present_flag (1 bit)
    pps.pps_slice_chroma_qp_offsets_present_flag = reader.read_bit()?;

    // weighted_pred_flag (1 bit)
    pps.weighted_pred_flag = reader.read_bit()?;

    // weighted_bipred_flag (1 bit)
    pps.weighted_bipred_flag = reader.read_bit()?;

    // transquant_bypass_enabled_flag (1 bit)
    pps.transquant_bypass_enabled_flag = reader.read_bit()?;

    // tiles_enabled_flag (1 bit)
    pps.tiles_enabled_flag = reader.read_bit()?;

    // entropy_coding_sync_enabled_flag (1 bit)
    pps.entropy_coding_sync_enabled_flag = reader.read_bit()?;

    if pps.tiles_enabled_flag {
        // num_tile_columns_minus1 (ue(v))
        let num_tile_columns_minus1 = reader.read_ue()? as u16;

        // num_tile_rows_minus1 (ue(v))
        let num_tile_rows_minus1 = reader.read_ue()? as u16;

        // uniform_spacing_flag (1 bit)
        let uniform_spacing_flag = reader.read_bit()?;

        let mut tile_config = TileConfig {
            num_tile_columns_minus1,
            num_tile_rows_minus1,
            uniform_spacing_flag,
            ..Default::default()
        };

        if !tile_config.uniform_spacing_flag {
            // column_width_minus1
            for _ in 0..tile_config.num_tile_columns_minus1 {
                tile_config
                    .column_width_minus1
                    .push(reader.read_ue()? as u16);
            }
            // row_height_minus1
            for _ in 0..tile_config.num_tile_rows_minus1 {
                tile_config.row_height_minus1.push(reader.read_ue()? as u16);
            }
        }

        // loop_filter_across_tiles_enabled_flag (1 bit)
        pps.loop_filter_across_tiles_enabled_flag = reader.read_bit()?;

        pps.tile_config = Some(tile_config);
    }

    // pps_loop_filter_across_slices_enabled_flag (1 bit)
    pps.pps_loop_filter_across_slices_enabled_flag = reader.read_bit()?;

    // deblocking_filter_control_present_flag (1 bit)
    pps.deblocking_filter_control_present_flag = reader.read_bit()?;

    if pps.deblocking_filter_control_present_flag {
        // deblocking_filter_override_enabled_flag (1 bit)
        pps.deblocking_filter_override_enabled_flag = reader.read_bit()?;

        // pps_deblocking_filter_disabled_flag (1 bit)
        pps.pps_deblocking_filter_disabled_flag = reader.read_bit()?;

        if !pps.pps_deblocking_filter_disabled_flag {
            // pps_beta_offset_div2 (se(v))
            pps.pps_beta_offset_div2 = reader.read_se()? as i8;

            // pps_tc_offset_div2 (se(v))
            pps.pps_tc_offset_div2 = reader.read_se()? as i8;
        }
    }

    // pps_scaling_list_data_present_flag (1 bit)
    pps.pps_scaling_list_data_present_flag = reader.read_bit()?;

    if pps.pps_scaling_list_data_present_flag {
        // Skip scaling_list_data() - complex structure
        // Full implementation would parse this
    }

    // lists_modification_present_flag (1 bit)
    pps.lists_modification_present_flag = reader.read_bit()?;

    // log2_parallel_merge_level_minus2 (ue(v))
    pps.log2_parallel_merge_level_minus2 = reader.read_ue()? as u8;

    // slice_segment_header_extension_present_flag (1 bit)
    pps.slice_segment_header_extension_present_flag = reader.read_bit()?;

    // pps_extension_present_flag (1 bit)
    pps.pps_extension_present_flag = reader.read_bit()?;

    if pps.pps_extension_present_flag {
        // pps_range_extension_flag (1 bit)
        pps.pps_range_extension_flag = reader.read_bit()?;

        // pps_multilayer_extension_flag (1 bit)
        pps.pps_multilayer_extension_flag = reader.read_bit()?;

        // pps_3d_extension_flag (1 bit)
        pps.pps_3d_extension_flag = reader.read_bit()?;

        // pps_scc_extension_flag (1 bit)
        pps.pps_scc_extension_flag = reader.read_bit()?;

        // Skip extension bits (4 bits)
        let _ = reader.read_bits(4)?;
    }

    Ok(pps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pps_defaults() {
        let pps = Pps::default();
        assert_eq!(pps.init_qp(), 26);
        assert!(!pps.has_tiles());
        assert!(!pps.wpp_enabled());
    }

    #[test]
    fn test_tile_config() {
        let mut tile_config = TileConfig::default();
        tile_config.num_tile_columns_minus1 = 3;
        tile_config.num_tile_rows_minus1 = 2;

        assert_eq!(tile_config.num_columns(), 4);
        assert_eq!(tile_config.num_rows(), 3);
        assert_eq!(tile_config.num_tiles(), 12);
    }
}
