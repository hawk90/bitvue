//! VVC NAL unit parsing.
//!
//! VVC NAL unit structure (ITU-T H.266 7.3.1.1):
//! - forbidden_zero_bit: 1 bit (must be 0)
//! - nuh_reserved_zero_bit: 1 bit
//! - nuh_layer_id: 6 bits
//! - nal_unit_type: 5 bits
//! - nuh_temporal_id_plus1: 3 bits

use crate::bitreader::{remove_emulation_prevention_bytes, BitReader};
use crate::error::{VvcError, Result};
use serde::{Deserialize, Serialize};

/// VVC NAL unit types (ITU-T H.266 Table 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NalUnitType {
    /// Coded slice of a trailing picture
    TrailNut = 0,
    /// Coded slice of a STSA picture
    StapNut = 1,
    /// Coded slice of a RADL picture
    RadlNut = 2,
    /// Coded slice of a RASL picture
    RaslNut = 3,
    /// Reserved VCL NAL unit types (4-6)
    RsvVcl4 = 4,
    RsvVcl5 = 5,
    RsvVcl6 = 6,
    /// Coded slice of an IDR picture (with RADL)
    IdrWRadl = 7,
    /// Coded slice of an IDR picture (no leading pictures)
    IdrNLp = 8,
    /// Coded slice of a CRA picture
    CraNut = 9,
    /// Coded slice of a GDR (Gradual Decoding Refresh) picture
    GdrNut = 10,
    /// Reserved VCL NAL unit types (11-12)
    RsvVcl11 = 11,
    RsvVcl12 = 12,
    /// Operating Point Information
    OpiNut = 13,
    /// Decoding Capability Information
    DciNut = 14,
    /// Video Parameter Set
    VpsNut = 15,
    /// Sequence Parameter Set
    SpsNut = 16,
    /// Picture Parameter Set
    PpsNut = 17,
    /// Prefix Adaptation Parameter Set
    PrefixApsNut = 18,
    /// Suffix Adaptation Parameter Set
    SuffixApsNut = 19,
    /// Picture Header
    PhNut = 20,
    /// Access Unit Delimiter
    AudNut = 21,
    /// End of Sequence
    EosNut = 22,
    /// End of Bitstream
    EobNut = 23,
    /// Prefix SEI
    PrefixSeiNut = 24,
    /// Suffix SEI
    SuffixSeiNut = 25,
    /// Filler Data
    FdNut = 26,
    /// Reserved (27-30)
    RsvNvcl27 = 27,
    RsvNvcl28 = 28,
    RsvNvcl29 = 29,
    RsvNvcl30 = 30,
    /// Unspecified (31)
    Unspec31 = 31,
}

impl NalUnitType {
    /// Create from raw u8 value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::TrailNut,
            1 => Self::StapNut,
            2 => Self::RadlNut,
            3 => Self::RaslNut,
            4 => Self::RsvVcl4,
            5 => Self::RsvVcl5,
            6 => Self::RsvVcl6,
            7 => Self::IdrWRadl,
            8 => Self::IdrNLp,
            9 => Self::CraNut,
            10 => Self::GdrNut,
            11 => Self::RsvVcl11,
            12 => Self::RsvVcl12,
            13 => Self::OpiNut,
            14 => Self::DciNut,
            15 => Self::VpsNut,
            16 => Self::SpsNut,
            17 => Self::PpsNut,
            18 => Self::PrefixApsNut,
            19 => Self::SuffixApsNut,
            20 => Self::PhNut,
            21 => Self::AudNut,
            22 => Self::EosNut,
            23 => Self::EobNut,
            24 => Self::PrefixSeiNut,
            25 => Self::SuffixSeiNut,
            26 => Self::FdNut,
            27 => Self::RsvNvcl27,
            28 => Self::RsvNvcl28,
            29 => Self::RsvNvcl29,
            30 => Self::RsvNvcl30,
            _ => Self::Unspec31,
        }
    }

    /// Check if this is a VCL NAL unit.
    pub fn is_vcl(&self) -> bool {
        (*self as u8) <= 12
    }

    /// Check if this is an IRAP (Intra Random Access Point).
    pub fn is_irap(&self) -> bool {
        matches!(self, Self::IdrWRadl | Self::IdrNLp | Self::CraNut | Self::GdrNut)
    }

    /// Check if this is an IDR picture.
    pub fn is_idr(&self) -> bool {
        matches!(self, Self::IdrWRadl | Self::IdrNLp)
    }

    /// Check if this is a CRA picture.
    pub fn is_cra(&self) -> bool {
        matches!(self, Self::CraNut)
    }

    /// Check if this is a GDR picture.
    pub fn is_gdr(&self) -> bool {
        matches!(self, Self::GdrNut)
    }

    /// Check if this is a RASL picture.
    pub fn is_rasl(&self) -> bool {
        matches!(self, Self::RaslNut)
    }

    /// Check if this is a RADL picture.
    pub fn is_radl(&self) -> bool {
        matches!(self, Self::RadlNut)
    }

    /// Check if this is a leading picture.
    pub fn is_leading(&self) -> bool {
        self.is_rasl() || self.is_radl()
    }

    /// Check if this is a trailing picture.
    pub fn is_trailing(&self) -> bool {
        matches!(self, Self::TrailNut)
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::TrailNut => "TRAIL_NUT",
            Self::StapNut => "STAP_NUT",
            Self::RadlNut => "RADL_NUT",
            Self::RaslNut => "RASL_NUT",
            Self::IdrWRadl => "IDR_W_RADL",
            Self::IdrNLp => "IDR_N_LP",
            Self::CraNut => "CRA_NUT",
            Self::GdrNut => "GDR_NUT",
            Self::OpiNut => "OPI_NUT",
            Self::DciNut => "DCI_NUT",
            Self::VpsNut => "VPS_NUT",
            Self::SpsNut => "SPS_NUT",
            Self::PpsNut => "PPS_NUT",
            Self::PrefixApsNut => "PREFIX_APS_NUT",
            Self::SuffixApsNut => "SUFFIX_APS_NUT",
            Self::PhNut => "PH_NUT",
            Self::AudNut => "AUD_NUT",
            Self::EosNut => "EOS_NUT",
            Self::EobNut => "EOB_NUT",
            Self::PrefixSeiNut => "PREFIX_SEI_NUT",
            Self::SuffixSeiNut => "SUFFIX_SEI_NUT",
            Self::FdNut => "FD_NUT",
            _ => "RESERVED/UNSPEC",
        }
    }
}

/// VVC NAL unit header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NalUnitHeader {
    /// NAL unit type (5 bits).
    pub nal_unit_type: NalUnitType,
    /// Layer ID (6 bits).
    pub nuh_layer_id: u8,
    /// Temporal ID + 1 (3 bits).
    pub nuh_temporal_id_plus1: u8,
}

impl NalUnitHeader {
    /// Get the actual temporal ID.
    pub fn temporal_id(&self) -> u8 {
        self.nuh_temporal_id_plus1.saturating_sub(1)
    }
}

/// Complete NAL unit with header and payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NalUnit {
    /// NAL unit header.
    pub header: NalUnitHeader,
    /// Byte offset in the original stream.
    pub offset: u64,
    /// Total size including header.
    pub size: u64,
    /// Raw payload bytes (after removing emulation prevention bytes).
    pub payload: Vec<u8>,
    /// Original payload with emulation prevention bytes.
    pub raw_payload: Vec<u8>,
}

impl NalUnit {
    /// Get NAL unit type.
    pub fn nal_type(&self) -> NalUnitType {
        self.header.nal_unit_type
    }

    /// Check if this is a VCL NAL unit.
    pub fn is_vcl(&self) -> bool {
        self.header.nal_unit_type.is_vcl()
    }

    /// Get RBSP data.
    pub fn rbsp(&self) -> &[u8] {
        &self.payload
    }
}

/// Parse NAL unit header from 2 bytes.
pub fn parse_nal_header(data: &[u8]) -> Result<NalUnitHeader> {
    if data.len() < 2 {
        return Err(VvcError::InsufficientData {
            expected: 2,
            actual: data.len(),
        });
    }

    let mut reader = BitReader::new(data);

    // forbidden_zero_bit (1 bit)
    let forbidden = reader.read_bit()?;
    if forbidden {
        return Err(VvcError::InvalidData(
            "forbidden_zero_bit is not zero".to_string(),
        ));
    }

    // nuh_reserved_zero_bit (1 bit)
    let _reserved = reader.read_bit()?;

    // nuh_layer_id (6 bits)
    let nuh_layer_id = reader.read_bits(6)? as u8;

    // nal_unit_type (5 bits)
    let nal_type_raw = reader.read_bits(5)? as u8;
    let nal_unit_type = NalUnitType::from_u8(nal_type_raw);

    // nuh_temporal_id_plus1 (3 bits)
    let nuh_temporal_id_plus1 = reader.read_bits(3)? as u8;

    Ok(NalUnitHeader {
        nal_unit_type,
        nuh_layer_id,
        nuh_temporal_id_plus1,
    })
}

/// Find NAL unit start codes in data.
pub fn find_nal_units(data: &[u8]) -> Vec<(usize, usize)> {
    let mut units = Vec::new();
    let mut i = 0;

    while i < data.len() {
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 {
            let (start_code_len, nal_start) = if i + 3 < data.len()
                && data[i + 2] == 0x00
                && data[i + 3] == 0x01
            {
                (4, i + 4)
            } else if data[i + 2] == 0x01 {
                (3, i + 3)
            } else {
                i += 1;
                continue;
            };

            let mut nal_end = data.len();
            let mut j = nal_start;
            while j + 2 < data.len() {
                if data[j] == 0x00 && data[j + 1] == 0x00 {
                    if (j + 2 < data.len() && data[j + 2] == 0x01)
                        || (j + 3 < data.len() && data[j + 2] == 0x00 && data[j + 3] == 0x01)
                    {
                        nal_end = j;
                        break;
                    }
                }
                j += 1;
            }

            units.push((nal_start, nal_end));
            i = nal_start + start_code_len;
        } else {
            i += 1;
        }
    }

    units
}

/// Parse all NAL units from Annex B byte stream.
pub fn parse_nal_units(data: &[u8]) -> Result<Vec<NalUnit>> {
    let positions = find_nal_units(data);
    let mut units = Vec::with_capacity(positions.len());

    for (start, end) in positions {
        if end <= start || start + 2 > data.len() {
            continue;
        }

        let nal_data = &data[start..end];
        let header = parse_nal_header(nal_data)?;

        let raw_payload = nal_data[2..].to_vec();
        let payload = remove_emulation_prevention_bytes(&raw_payload);

        units.push(NalUnit {
            header,
            offset: start as u64,
            size: (end - start) as u64,
            payload,
            raw_payload,
        });
    }

    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nal_type_properties() {
        assert!(NalUnitType::IdrWRadl.is_idr());
        assert!(NalUnitType::IdrWRadl.is_irap());

        assert!(NalUnitType::CraNut.is_cra());
        assert!(NalUnitType::CraNut.is_irap());

        assert!(NalUnitType::GdrNut.is_gdr());
        assert!(NalUnitType::GdrNut.is_irap());

        assert!(NalUnitType::TrailNut.is_vcl());
        assert!(!NalUnitType::SpsNut.is_vcl());
    }

    #[test]
    fn test_find_nal_units() {
        let data = [
            0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, // VPS header
            0xAA, 0xBB,
            0x00, 0x00, 0x01,
            0x00, 0x81, // SPS header
            0xCC, 0xDD,
        ];

        let units = find_nal_units(&data);
        assert_eq!(units.len(), 2);
    }
}
