//! OBU Header syntax parsing with bit-level tracking
//!
//! Parses AV1 OBU headers and creates detailed syntax trees with exact bit ranges.

use super::{SyntaxBuilder, TrackedBitReader};
use crate::obu::ObuType;
use bitvue_core::{BitvueError, Result};

/// Maximum bytes for a valid LEB128 in AV1 (8 bytes = 56 bits max)
const MAX_LEB128_BYTES: usize = 8;

/// Parse OBU header with bit-level tracking
///
/// AV1 Spec Section 5.3.2:
/// ```text
/// obu_header() {
///     obu_forbidden_bit            f(1)
///     obu_type                     f(4)
///     obu_extension_flag           f(1)
///     obu_has_size_field           f(1)
///     obu_reserved_1bit            f(1)
///     if (obu_extension_flag == 1) {
///         obu_extension_header()
///     }
/// }
/// ```
///
/// # Returns
///
/// Returns a tuple of (OBU type, has_size_field flag)
pub fn parse_obu_header_syntax(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
) -> Result<(ObuType, bool)> {
    let start = reader.position();
    builder.push_container("obu_header", start);

    // obu_forbidden_bit (1 bit) - must be 0
    let (forbidden, range) = reader.read_bit_tracked()?;
    builder.add_field("obu_forbidden_bit", range, format!("{}", forbidden as u8));

    // Validate forbidden bit
    if forbidden {
        return Err(bitvue_core::BitvueError::Parse {
            offset: range.start_bit,
            message: "obu_forbidden_bit must be 0".to_string(),
        });
    }

    // obu_type (4 bits)
    let (type_val, range) = reader.read_bits_tracked(4)?;
    let obu_type = ObuType::from_u8(type_val as u8)?;
    builder.add_field(
        "obu_type",
        range,
        format!("{} ({})", type_val, obu_type.name()),
    );

    // obu_extension_flag (1 bit)
    let (has_ext, range) = reader.read_bit_tracked()?;
    builder.add_field("obu_extension_flag", range, format!("{}", has_ext as u8));

    // obu_has_size_field (1 bit)
    let (has_size, range) = reader.read_bit_tracked()?;
    builder.add_field("obu_has_size_field", range, format!("{}", has_size as u8));

    // obu_reserved_1bit (1 bit) - must be 0
    let (reserved, range) = reader.read_bit_tracked()?;
    builder.add_field("obu_reserved_1bit", range, format!("{}", reserved as u8));

    // Conditional: extension header
    if has_ext {
        parse_extension_header(reader, builder)?;
    }

    let end = reader.position();
    builder.pop_container(end);

    Ok((obu_type, has_size))
}

/// Parse OBU extension header
///
/// AV1 Spec Section 5.3.3:
/// ```text
/// obu_extension_header() {
///     temporal_id                      f(3)
///     spatial_id                       f(2)
///     extension_header_reserved_3bits  f(3)
/// }
/// ```
fn parse_extension_header(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
) -> Result<()> {
    let start = reader.position();
    builder.push_container("obu_extension_header", start);

    // temporal_id (3 bits)
    let (temporal_id, range) = reader.read_bits_tracked(3)?;
    builder.add_field("temporal_id", range, format!("{}", temporal_id));

    // spatial_id (2 bits)
    let (spatial_id, range) = reader.read_bits_tracked(2)?;
    builder.add_field("spatial_id", range, format!("{}", spatial_id));

    // extension_header_reserved_3bits (3 bits) - should be 0
    let (reserved, range) = reader.read_bits_tracked(3)?;
    builder.add_field(
        "extension_header_reserved_3bits",
        range,
        format!("{}", reserved),
    );

    let end = reader.position();
    builder.pop_container(end);

    Ok(())
}

/// Parse LEB128 size field with bit-level tracking
///
/// AV1 uses unsigned LEB128 encoding for OBU size fields.
/// Each byte has 7 bits of data (LSB first) and 1 continuation bit (MSB).
///
/// Returns the decoded size value.
pub fn parse_leb128_size_syntax(
    reader: &mut TrackedBitReader,
    builder: &mut SyntaxBuilder,
) -> Result<u64> {
    let start = reader.position();
    builder.push_container("obu_size", start);

    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut byte_index = 0;

    loop {
        if byte_index >= MAX_LEB128_BYTES {
            return Err(BitvueError::Parse {
                offset: reader.position(),
                message: format!("LEB128 exceeded maximum {} bytes", MAX_LEB128_BYTES),
            });
        }

        // Read one byte
        let (byte, range) = reader.read_byte_tracked()?;

        // Extract 7 data bits
        let data_bits = (byte & 0x7F) as u64;
        let has_continuation = (byte & 0x80) != 0;

        // Check for overflow before shifting
        if shift >= 64 || (shift > 0 && data_bits > (u64::MAX >> shift)) {
            return Err(BitvueError::Parse {
                offset: range.start_bit,
                message: "LEB128 value overflow".to_string(),
            });
        }

        value |= data_bits << shift;
        shift += 7;

        // Create syntax node for this byte
        let field_name = format!("size_byte[{}]", byte_index);
        let value_str = format!(
            "0x{:02X} (data: {}, continue: {})",
            byte,
            data_bits,
            if has_continuation { 1 } else { 0 }
        );
        builder.add_field(&field_name, range, value_str);

        byte_index += 1;

        // If no continuation bit, we're done
        if !has_continuation {
            break;
        }
    }

    let end = reader.position();
    builder.pop_container(end);

    // Add a summary field showing the decoded size value
    let summary_range = bitvue_core::types::BitRange::new(start, end);
    builder.add_field("obu_size_value", summary_range, format!("{} bytes", value));

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_core::types::BitRange;

    #[test]
    fn test_parse_temporal_delimiter_header() {
        // Temporal delimiter OBU header: 0x12
        // Binary: 0001 0010
        //   forbidden_bit: 0
        //   obu_type: 0010 (2 = TEMPORAL_DELIMITER)
        //   extension_flag: 0
        //   has_size: 1
        //   reserved: 0
        let data = [0x12];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let (obu_type, has_size) = parse_obu_header_syntax(&mut reader, &mut builder).unwrap();

        assert_eq!(obu_type, ObuType::TemporalDelimiter);
        assert!(has_size);

        let model = builder.build();

        // Check header container
        let header = model.get_node("obu[0].obu_header").unwrap();
        assert_eq!(header.bit_range, BitRange::new(0, 8));

        // Check obu_type field
        let type_node = model.get_node("obu[0].obu_header.obu_type").unwrap();
        assert_eq!(type_node.bit_range, BitRange::new(1, 5));
        assert!(type_node
            .value
            .as_ref()
            .unwrap()
            .contains("TEMPORAL_DELIMITER"));
    }

    #[test]
    fn test_parse_sequence_header_with_extension() {
        // Sequence header with extension: 0x0A 0x0B
        // Byte 1: 0000 1010
        //   forbidden_bit: 0
        //   obu_type: 0001 (1 = SEQUENCE_HEADER)
        //   extension_flag: 0
        //   has_size: 1
        //   reserved: 0
        // No extension header in this example
        let data = [0x0A];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let (obu_type, has_size) = parse_obu_header_syntax(&mut reader, &mut builder).unwrap();

        assert_eq!(obu_type, ObuType::SequenceHeader);
        assert!(has_size);

        let model = builder.build();
        assert!(model.get_node("obu[0].obu_header").is_some());
    }

    #[test]
    fn test_parse_header_with_extension() {
        // Header with extension: 0x16 0xA8
        // Byte 1: 0001 0110
        //   forbidden_bit: 0
        //   obu_type: 0010 (2 = TEMPORAL_DELIMITER)
        //   extension_flag: 1
        //   has_size: 1
        //   reserved: 0
        // Byte 2: 1010 1000
        //   temporal_id: 101 (5)
        //   spatial_id: 01 (1)
        //   reserved: 000
        let data = [0x16, 0xA8];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let (_obu_type, has_size) = parse_obu_header_syntax(&mut reader, &mut builder).unwrap();
        assert!(has_size);

        let model = builder.build();

        // Check extension header exists
        let ext_header = model.get_node("obu[0].obu_header.obu_extension_header");
        assert!(ext_header.is_some());

        // Check temporal_id
        let temporal = model
            .get_node("obu[0].obu_header.obu_extension_header.temporal_id")
            .unwrap();
        assert_eq!(temporal.bit_range, BitRange::new(8, 11));
        assert_eq!(temporal.value.as_ref().unwrap(), "5");

        // Check spatial_id
        let spatial = model
            .get_node("obu[0].obu_header.obu_extension_header.spatial_id")
            .unwrap();
        assert_eq!(spatial.bit_range, BitRange::new(11, 13));
        assert_eq!(spatial.value.as_ref().unwrap(), "1");
    }

    #[test]
    fn test_forbidden_bit_validation() {
        // Invalid header with forbidden_bit = 1: 0x92
        // Binary: 1001 0010
        let data = [0x92];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let result = parse_obu_header_syntax(&mut reader, &mut builder);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_leb128_single_byte() {
        // Size = 127 (0x7F) - single byte, no continuation
        let data = [0x7F];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let size = parse_leb128_size_syntax(&mut reader, &mut builder).unwrap();
        assert_eq!(size, 127);

        let model = builder.build();

        // Check container
        let container = model.get_node("obu[0].obu_size").unwrap();
        assert_eq!(container.bit_range, BitRange::new(0, 8));

        // Check byte node
        let byte_node = model.get_node("obu[0].obu_size.size_byte[0]").unwrap();
        assert_eq!(byte_node.bit_range, BitRange::new(0, 8));
        assert!(byte_node.value.as_ref().unwrap().contains("0x7F"));
        assert!(byte_node.value.as_ref().unwrap().contains("data: 127"));
        assert!(byte_node.value.as_ref().unwrap().contains("continue: 0"));

        // Check summary value node
        let value_node = model.get_node("obu[0].obu_size_value").unwrap();
        assert_eq!(value_node.value.as_ref().unwrap(), "127 bytes");
    }

    #[test]
    fn test_parse_leb128_two_bytes() {
        // Size = 128 (0x80 0x01)
        // Byte 0: 10000000 -> data: 0, continue: 1
        // Byte 1: 00000001 -> data: 1, continue: 0
        // Value: 0 + (1 << 7) = 128
        let data = [0x80, 0x01];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let size = parse_leb128_size_syntax(&mut reader, &mut builder).unwrap();
        assert_eq!(size, 128);

        let model = builder.build();

        // Check container spans both bytes
        let container = model.get_node("obu[0].obu_size").unwrap();
        assert_eq!(container.bit_range, BitRange::new(0, 16));

        // Check first byte
        let byte0 = model.get_node("obu[0].obu_size.size_byte[0]").unwrap();
        assert_eq!(byte0.bit_range, BitRange::new(0, 8));
        assert!(byte0.value.as_ref().unwrap().contains("0x80"));
        assert!(byte0.value.as_ref().unwrap().contains("data: 0"));
        assert!(byte0.value.as_ref().unwrap().contains("continue: 1"));

        // Check second byte
        let byte1 = model.get_node("obu[0].obu_size.size_byte[1]").unwrap();
        assert_eq!(byte1.bit_range, BitRange::new(8, 16));
        assert!(byte1.value.as_ref().unwrap().contains("0x01"));
        assert!(byte1.value.as_ref().unwrap().contains("data: 1"));
        assert!(byte1.value.as_ref().unwrap().contains("continue: 0"));

        // Check summary
        let value_node = model.get_node("obu[0].obu_size_value").unwrap();
        assert_eq!(value_node.value.as_ref().unwrap(), "128 bytes");
    }

    #[test]
    fn test_parse_leb128_three_bytes() {
        // Size = 16384 (0x80 0x80 0x01)
        // Byte 0: data: 0, continue: 1
        // Byte 1: data: 0, continue: 1
        // Byte 2: data: 1, continue: 0
        // Value: 0 + (0 << 7) + (1 << 14) = 16384
        let data = [0x80, 0x80, 0x01];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let size = parse_leb128_size_syntax(&mut reader, &mut builder).unwrap();
        assert_eq!(size, 16384);

        let model = builder.build();

        // Check container spans all three bytes
        let container = model.get_node("obu[0].obu_size").unwrap();
        assert_eq!(container.bit_range, BitRange::new(0, 24));

        // Verify all three bytes exist
        assert!(model.get_node("obu[0].obu_size.size_byte[0]").is_some());
        assert!(model.get_node("obu[0].obu_size.size_byte[1]").is_some());
        assert!(model.get_node("obu[0].obu_size.size_byte[2]").is_some());
    }

    #[test]
    fn test_parse_leb128_with_offset() {
        // Test with non-zero global offset
        let data = [0xFF, 0x01]; // Size = 255
        let mut reader = TrackedBitReader::new(&data, 1000); // Offset at bit 1000
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let size = parse_leb128_size_syntax(&mut reader, &mut builder).unwrap();
        assert_eq!(size, 255);

        let model = builder.build();

        // Check that bit ranges are offset correctly
        let container = model.get_node("obu[0].obu_size").unwrap();
        assert_eq!(container.bit_range, BitRange::new(1000, 1016));

        let byte0 = model.get_node("obu[0].obu_size.size_byte[0]").unwrap();
        assert_eq!(byte0.bit_range, BitRange::new(1000, 1008));

        let byte1 = model.get_node("obu[0].obu_size.size_byte[1]").unwrap();
        assert_eq!(byte1.bit_range, BitRange::new(1008, 1016));
    }

    #[test]
    fn test_parse_leb128_zero() {
        // Size = 0 (0x00) - edge case
        let data = [0x00];
        let mut reader = TrackedBitReader::new(&data, 0);
        let mut builder = SyntaxBuilder::new("obu[0]".to_string(), "obu_0".to_string());

        let size = parse_leb128_size_syntax(&mut reader, &mut builder).unwrap();
        assert_eq!(size, 0);

        let model = builder.build();
        let value_node = model.get_node("obu[0].obu_size_value").unwrap();
        assert_eq!(value_node.value.as_ref().unwrap(), "0 bytes");
    }
}
