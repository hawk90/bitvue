//! Frame Header syntax parsing (simplified for Phase 0)
//!
//! This module provides syntax tree generation for AV1 Frame Headers,
//! tracking exact bit ranges for Tri-sync functionality.
//!
//! **Phase 0 Scope**: Parses basic fields only (~5-10 syntax nodes)
//! - show_existing_frame, frame_to_show_map_idx
//! - frame_type, show_frame
//! - error_resilient_mode (conditional)
//!
//! **Deferred**: Quantization params, loop filter, segmentation, etc.
//!
//! # AV1 Specification Reference
//! - Section 5.9: Frame Header OBU
//! - Section 5.9.2: Uncompressed Header Syntax

use super::{SyntaxBuilder, TrackedBitReader};
use crate::frame_header::FrameType;
use bitvue_core::Result;

/// Parse Frame Header OBU payload with bit-level tracking (simplified)
///
/// Creates a syntax tree for the frame header, including:
/// - show_existing_frame flag
/// - frame_to_show_map_idx (if showing existing frame)
/// - frame_type (KEY, INTER, INTRA_ONLY, SWITCH)
/// - show_frame flag
/// - error_resilient_mode (conditional)
///
/// # Arguments
///
/// * `reader` - TrackedBitReader positioned at start of frame header payload
/// * `builder` - SyntaxBuilder for constructing the syntax tree
///
/// # Returns
///
/// `Ok(())` if parsing succeeds, or an error if the bitstream is malformed.
///
/// # Phase 0 Simplification
///
/// This parser only extracts the first few fields of the frame header.
/// Full frame header parsing requires sequence header context and is
/// extremely complex (100+ conditional fields). For Phase 0 tri-sync
/// demonstration, basic fields are sufficient.
///
/// # Example Syntax Tree
///
/// ```text
/// frame_header
/// ├── show_existing_frame: "0"
/// ├── frame_type: "0 (KEY_FRAME)"
/// ├── show_frame: "1"
/// └── error_resilient_mode: "1"
/// ```
pub fn parse_frame_header_syntax(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
) -> Result<()> {
    let start = reader.position();
    builder.push_container("frame_header", start);

    // show_existing_frame (1 bit) - AV1 Spec 5.9.2
    let (show_existing, range) = reader.read_bit_tracked()?;
    builder.add_field(
        "show_existing_frame",
        range,
        format!("{}", show_existing as u8),
    );

    if show_existing {
        // frame_to_show_map_idx (3 bits) - AV1 Spec 5.9.3
        let (idx, range) = reader.read_bits_tracked(3)?;
        builder.add_field("frame_to_show_map_idx", range, format!("{}", idx));

        // If showing existing frame, that's all we need to parse
        // (no frame_type or other fields follow)
        let end = reader.position();
        builder.pop_container(end);
        return Ok(());
    }

    // frame_type (2 bits) - AV1 Spec 5.9.5
    let (frame_type_bits, range) = reader.read_bits_tracked(2)?;
    let frame_type = FrameType::from_av1_bits(frame_type_bits);
    builder.add_field(
        "frame_type",
        range,
        format!("{} ({})", frame_type_bits, frame_type),
    );

    // show_frame (1 bit) - AV1 Spec 5.9.6
    let (show_frame, range) = reader.read_bit_tracked()?;
    builder.add_field("show_frame", range, format!("{}", show_frame as u8));

    // showable_frame (1 bit) - AV1 Spec 5.9.7
    // Conditional: only if !show_frame
    if !show_frame {
        let (showable, range) = reader.read_bit_tracked()?;
        builder.add_field("showable_frame", range, format!("{}", showable as u8));
    }

    // error_resilient_mode (1 bit) - AV1 Spec 5.9.8
    // Conditional: depends on frame_type and show_frame
    let _error_resilient = match (frame_type, show_frame) {
        // Implicit: always true for these cases (no bit in bitstream)
        (FrameType::Switch, _) | (FrameType::Key, true) => {
            // Add virtual node for clarity
            builder.add_field(
                "error_resilient_mode",
                bitvue_core::types::BitRange::new(reader.position(), reader.position()),
                "1 (implicit)".to_string(),
            );
            true
        }
        // Explicit bit in bitstream for other cases
        _ => {
            let (err_resilient, range) = reader.read_bit_tracked()?;
            builder.add_field(
                "error_resilient_mode",
                range,
                format!("{}", err_resilient as u8),
            );
            err_resilient
        }
    };

    // Phase 0: Stop here
    // Full frame header has 100+ more fields:
    // - disable_cdf_update
    // - allow_screen_content_tools
    // - force_integer_mv
    // - frame_size
    // - render_size
    // - allow_intrabc
    // - frame_refs_short_signaling
    // - ref_frame_idx[]
    // - allow_high_precision_mv
    // - interpolation_filter
    // - is_motion_mode_switchable
    // - use_ref_frame_mvs
    // - disable_frame_end_update_cdf
    // - tile_info
    // - quantization_params
    // - segmentation_params
    // - delta_q_params
    // - delta_lf_params
    // - loop_filter_params
    // - cdef_params
    // - lr_params
    // - tx_mode
    // - frame_reference_mode
    // - skip_mode_params
    // - ... etc

    let end = reader.position();
    builder.pop_container(end);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_core::types::BitRange;

    #[test]
    fn test_parse_show_existing_frame() {
        // show_existing_frame=1, frame_to_show_map_idx=5 (binary: 101)
        // Binary: 1 101 xxxxx
        let mut data = vec![0b1101_0000]; // show_existing=1, idx=5
        data.extend_from_slice(&[0u8; 10]); // Padding
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let result = parse_frame_header_syntax(&mut reader, &mut builder);
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        let model = builder.build();

        // Verify container
        assert!(model.get_node("obu[0].frame_header").is_some());

        // Verify show_existing_frame
        let show_existing = model
            .get_node("obu[0].frame_header.show_existing_frame")
            .unwrap();
        assert_eq!(show_existing.bit_range, BitRange::new(0, 1));
        assert_eq!(show_existing.value.as_ref().unwrap(), "1");

        // Verify frame_to_show_map_idx
        let idx = model
            .get_node("obu[0].frame_header.frame_to_show_map_idx")
            .unwrap();
        assert_eq!(idx.bit_range, BitRange::new(1, 4));
        assert_eq!(idx.value.as_ref().unwrap(), "5");

        // frame_type should NOT exist (early return for show_existing)
        assert!(model.get_node("obu[0].frame_header.frame_type").is_none());
    }

    #[test]
    fn test_parse_key_frame() {
        // show_existing_frame=0, frame_type=00 (KEY), show_frame=1
        // error_resilient_mode is implicit=1 for Key+show
        // Binary: 0 00 1
        let mut data = vec![0b0001_0000]; // show_existing=0, type=00 (KEY), show=1
        data.extend_from_slice(&[0u8; 10]); // Padding
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let result = parse_frame_header_syntax(&mut reader, &mut builder);
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        let model = builder.build();

        // Verify show_existing_frame
        let show_existing = model
            .get_node("obu[0].frame_header.show_existing_frame")
            .unwrap();
        assert_eq!(show_existing.value.as_ref().unwrap(), "0");

        // Verify frame_type
        let frame_type = model.get_node("obu[0].frame_header.frame_type").unwrap();
        assert_eq!(frame_type.bit_range, BitRange::new(1, 3));
        assert!(frame_type.value.as_ref().unwrap().contains("KEY_FRAME"));

        // Verify show_frame
        let show_frame = model.get_node("obu[0].frame_header.show_frame").unwrap();
        assert_eq!(show_frame.bit_range, BitRange::new(3, 4));
        assert_eq!(show_frame.value.as_ref().unwrap(), "1");

        // Verify error_resilient_mode (implicit)
        let err_resilient = model
            .get_node("obu[0].frame_header.error_resilient_mode")
            .unwrap();
        assert!(err_resilient.value.as_ref().unwrap().contains("implicit"));
    }

    #[test]
    fn test_parse_inter_frame() {
        // show_existing_frame=0, frame_type=01 (INTER), show_frame=1, error_resilient=0
        // Binary: 0 01 1 0
        let mut data = vec![0b0011_0000]; // show_existing=0, type=01 (INTER), show=1, err=0
        data.extend_from_slice(&[0u8; 10]); // Padding
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let result = parse_frame_header_syntax(&mut reader, &mut builder);
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        let model = builder.build();

        // Verify frame_type
        let frame_type = model.get_node("obu[0].frame_header.frame_type").unwrap();
        assert!(frame_type.value.as_ref().unwrap().contains("INTER_FRAME"));

        // Verify show_frame
        let show_frame = model.get_node("obu[0].frame_header.show_frame").unwrap();
        assert_eq!(show_frame.value.as_ref().unwrap(), "1");

        // Verify error_resilient_mode (explicit bit)
        let err_resilient = model
            .get_node("obu[0].frame_header.error_resilient_mode")
            .unwrap();
        assert_eq!(err_resilient.bit_range, BitRange::new(4, 5));
        assert_eq!(err_resilient.value.as_ref().unwrap(), "0");
    }

    #[test]
    fn test_parse_frame_not_shown() {
        // show_existing_frame=0, frame_type=01 (INTER), show_frame=0, showable_frame=1, error_resilient=0
        // Binary: 0 01 0 1 0
        let mut data = vec![0b0010_1000]; // show_existing=0, type=01, show=0, showable=1, err=0
        data.extend_from_slice(&[0u8; 10]); // Padding
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let result = parse_frame_header_syntax(&mut reader, &mut builder);
        assert!(result.is_ok(), "Parsing should succeed: {:?}", result.err());

        let model = builder.build();

        // Verify show_frame
        let show_frame = model.get_node("obu[0].frame_header.show_frame").unwrap();
        assert_eq!(show_frame.value.as_ref().unwrap(), "0");

        // Verify showable_frame (conditional on !show_frame)
        let showable = model
            .get_node("obu[0].frame_header.showable_frame")
            .unwrap();
        assert_eq!(showable.bit_range, BitRange::new(4, 5));
        assert_eq!(showable.value.as_ref().unwrap(), "1");

        // Verify error_resilient_mode
        let err_resilient = model
            .get_node("obu[0].frame_header.error_resilient_mode")
            .unwrap();
        assert_eq!(err_resilient.bit_range, BitRange::new(5, 6));
        assert_eq!(err_resilient.value.as_ref().unwrap(), "0");
    }
}
