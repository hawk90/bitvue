//! H.264/AVC NAL (Network Abstraction Layer) unit parsing.

use crate::bitreader::remove_emulation_prevention_bytes;
use crate::error::{AvcError, Result};
use serde::{Deserialize, Serialize};

/// H.264/AVC NAL unit types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NalUnitType {
    /// Unspecified
    Unspecified = 0,
    /// Coded slice of a non-IDR picture
    NonIdrSlice = 1,
    /// Coded slice data partition A
    SliceDataA = 2,
    /// Coded slice data partition B
    SliceDataB = 3,
    /// Coded slice data partition C
    SliceDataC = 4,
    /// Coded slice of an IDR picture
    IdrSlice = 5,
    /// Supplemental enhancement information (SEI)
    Sei = 6,
    /// Sequence parameter set (SPS)
    Sps = 7,
    /// Picture parameter set (PPS)
    Pps = 8,
    /// Access unit delimiter
    Aud = 9,
    /// End of sequence
    EndOfSequence = 10,
    /// End of stream
    EndOfStream = 11,
    /// Filler data
    FillerData = 12,
    /// SPS extension
    SpsExtension = 13,
    /// Prefix NAL unit
    PrefixNal = 14,
    /// Subset SPS
    SubsetSps = 15,
    /// Depth parameter set
    Dps = 16,
    /// Reserved (17-18)
    Reserved17 = 17,
    Reserved18 = 18,
    /// Coded slice of an auxiliary coded picture
    AuxSlice = 19,
    /// Coded slice extension
    SliceExtension = 20,
    /// Coded slice extension for depth view
    SliceExtensionDepth = 21,
    /// Reserved (22-23)
    Reserved22 = 22,
    Reserved23 = 23,
    /// Unspecified (24-31)
    Unspecified24 = 24,
}

impl NalUnitType {
    /// Create from raw value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => NalUnitType::Unspecified,
            1 => NalUnitType::NonIdrSlice,
            2 => NalUnitType::SliceDataA,
            3 => NalUnitType::SliceDataB,
            4 => NalUnitType::SliceDataC,
            5 => NalUnitType::IdrSlice,
            6 => NalUnitType::Sei,
            7 => NalUnitType::Sps,
            8 => NalUnitType::Pps,
            9 => NalUnitType::Aud,
            10 => NalUnitType::EndOfSequence,
            11 => NalUnitType::EndOfStream,
            12 => NalUnitType::FillerData,
            13 => NalUnitType::SpsExtension,
            14 => NalUnitType::PrefixNal,
            15 => NalUnitType::SubsetSps,
            16 => NalUnitType::Dps,
            17 => NalUnitType::Reserved17,
            18 => NalUnitType::Reserved18,
            19 => NalUnitType::AuxSlice,
            20 => NalUnitType::SliceExtension,
            21 => NalUnitType::SliceExtensionDepth,
            22 => NalUnitType::Reserved22,
            23 => NalUnitType::Reserved23,
            _ => NalUnitType::Unspecified24,
        }
    }

    /// Check if this is a VCL (Video Coding Layer) NAL unit.
    pub fn is_vcl(&self) -> bool {
        matches!(
            self,
            NalUnitType::NonIdrSlice
                | NalUnitType::SliceDataA
                | NalUnitType::SliceDataB
                | NalUnitType::SliceDataC
                | NalUnitType::IdrSlice
                | NalUnitType::AuxSlice
                | NalUnitType::SliceExtension
                | NalUnitType::SliceExtensionDepth
        )
    }

    /// Check if this is a parameter set.
    pub fn is_parameter_set(&self) -> bool {
        matches!(
            self,
            NalUnitType::Sps
                | NalUnitType::Pps
                | NalUnitType::SpsExtension
                | NalUnitType::SubsetSps
        )
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            NalUnitType::Unspecified => "Unspecified",
            NalUnitType::NonIdrSlice => "Non-IDR Slice",
            NalUnitType::SliceDataA => "Slice Data A",
            NalUnitType::SliceDataB => "Slice Data B",
            NalUnitType::SliceDataC => "Slice Data C",
            NalUnitType::IdrSlice => "IDR Slice",
            NalUnitType::Sei => "SEI",
            NalUnitType::Sps => "SPS",
            NalUnitType::Pps => "PPS",
            NalUnitType::Aud => "AUD",
            NalUnitType::EndOfSequence => "End of Sequence",
            NalUnitType::EndOfStream => "End of Stream",
            NalUnitType::FillerData => "Filler Data",
            NalUnitType::SpsExtension => "SPS Extension",
            NalUnitType::PrefixNal => "Prefix NAL",
            NalUnitType::SubsetSps => "Subset SPS",
            NalUnitType::Dps => "DPS",
            NalUnitType::Reserved17 | NalUnitType::Reserved18 => "Reserved",
            NalUnitType::AuxSlice => "Auxiliary Slice",
            NalUnitType::SliceExtension => "Slice Extension",
            NalUnitType::SliceExtensionDepth => "Slice Extension (Depth)",
            NalUnitType::Reserved22 | NalUnitType::Reserved23 => "Reserved",
            NalUnitType::Unspecified24 => "Unspecified",
        }
    }
}

/// NAL unit header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NalUnitHeader {
    /// forbidden_zero_bit (should be 0)
    pub forbidden_zero_bit: bool,
    /// nal_ref_idc (0-3)
    pub nal_ref_idc: u8,
    /// nal_unit_type
    pub nal_unit_type: NalUnitType,
}

/// Parsed NAL unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NalUnit {
    /// NAL unit header.
    pub header: NalUnitHeader,
    /// Byte offset in the original stream (start of NAL unit header).
    pub offset: usize,
    /// Size of the NAL unit in bytes (including header).
    pub size: usize,
    /// Raw payload (after header, with emulation prevention bytes removed).
    pub payload: Vec<u8>,
    /// Original payload (with emulation prevention bytes).
    pub raw_payload: Vec<u8>,
}

impl NalUnit {
    /// Get NAL unit type.
    pub fn nal_type(&self) -> NalUnitType {
        self.header.nal_unit_type
    }

    /// Check if this is a reference picture.
    pub fn is_reference(&self) -> bool {
        self.header.nal_ref_idc > 0
    }
}

/// Parse NAL unit header from a single byte.
pub fn parse_nal_header(byte: u8) -> Result<NalUnitHeader> {
    let forbidden_zero_bit = (byte >> 7) & 1 != 0;
    let nal_ref_idc = (byte >> 5) & 0x03;
    let nal_unit_type = NalUnitType::from_u8(byte & 0x1F);

    if forbidden_zero_bit {
        return Err(AvcError::InvalidNalUnit(
            "forbidden_zero_bit is set".to_string(),
        ));
    }

    Ok(NalUnitHeader {
        forbidden_zero_bit,
        nal_ref_idc,
        nal_unit_type,
    })
}

/// Find NAL unit start codes in Annex B byte stream.
/// Returns offsets pointing to the first byte after the start code.
pub fn find_nal_units(data: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut i = 0;

    while i < data.len() {
        // Look for start code: 0x000001 or 0x00000001
        if i + 2 < data.len() && data[i] == 0 && data[i + 1] == 0 {
            if data[i + 2] == 1 {
                // 3-byte start code
                positions.push(i + 3);
                i += 3;
                continue;
            } else if i + 3 < data.len() && data[i + 2] == 0 && data[i + 3] == 1 {
                // 4-byte start code
                positions.push(i + 4);
                i += 4;
                continue;
            }
        }
        i += 1;
    }

    positions
}

/// Parse all NAL units from Annex B byte stream.
pub fn parse_nal_units(data: &[u8]) -> Result<Vec<NalUnit>> {
    let positions = find_nal_units(data);
    let mut nal_units = Vec::new();

    for (idx, &start) in positions.iter().enumerate() {
        if start >= data.len() {
            continue;
        }

        // Determine end of NAL unit
        let end = if idx + 1 < positions.len() {
            // Find the start code before next NAL
            let next_start = positions[idx + 1];
            // Go back to find the 0x000001 or 0x00000001
            if next_start >= 4 && data[next_start - 4] == 0 {
                next_start - 4
            } else {
                next_start - 3
            }
        } else {
            data.len()
        };

        if start >= end {
            continue;
        }

        // Parse header
        let header = parse_nal_header(data[start])?;

        // Extract payload (skip header byte)
        let raw_payload = data[start + 1..end].to_vec();
        let payload = remove_emulation_prevention_bytes(&raw_payload);

        // Calculate offset (pointing to header byte)
        let offset = if start >= 4 && data[start - 4] == 0 {
            start - 4
        } else {
            start - 3
        };

        nal_units.push(NalUnit {
            header,
            offset,
            size: end - offset,
            payload,
            raw_payload,
        });
    }

    Ok(nal_units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_start_codes() {
        let data = [0x00, 0x00, 0x01, 0x67, 0x00, 0x00, 0x00, 0x01, 0x68];
        let positions = find_nal_units(&data);
        assert_eq!(positions, vec![3, 8]);
    }

    #[test]
    fn test_parse_nal_header() {
        // SPS: nal_ref_idc=3, nal_unit_type=7
        let header = parse_nal_header(0x67).unwrap();
        assert_eq!(header.nal_ref_idc, 3);
        assert_eq!(header.nal_unit_type, NalUnitType::Sps);

        // PPS: nal_ref_idc=3, nal_unit_type=8
        let header = parse_nal_header(0x68).unwrap();
        assert_eq!(header.nal_ref_idc, 3);
        assert_eq!(header.nal_unit_type, NalUnitType::Pps);

        // IDR: nal_ref_idc=3, nal_unit_type=5
        let header = parse_nal_header(0x65).unwrap();
        assert_eq!(header.nal_ref_idc, 3);
        assert_eq!(header.nal_unit_type, NalUnitType::IdrSlice);
    }

    #[test]
    fn test_nal_type_is_vcl() {
        assert!(NalUnitType::NonIdrSlice.is_vcl());
        assert!(NalUnitType::IdrSlice.is_vcl());
        assert!(!NalUnitType::Sps.is_vcl());
        assert!(!NalUnitType::Pps.is_vcl());
        assert!(!NalUnitType::Sei.is_vcl());
    }
}
