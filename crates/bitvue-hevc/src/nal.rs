//! HEVC NAL unit parsing.
//!
//! NAL unit structure (ITU-T H.265 7.3.1.1):
//! - forbidden_zero_bit: 1 bit (must be 0)
//! - nal_unit_type: 6 bits
//! - nuh_layer_id: 6 bits
//! - nuh_temporal_id_plus1: 3 bits

use crate::bitreader::{remove_emulation_prevention_bytes, BitReader};
use crate::error::{HevcError, Result};
use serde::{Deserialize, Serialize};

/// HEVC NAL unit types (ITU-T H.265 Table 7-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NalUnitType {
    // Coded slice segment of trailing picture
    TrailN = 0,
    TrailR = 1,
    // Coded slice segment of TSA picture
    TsaN = 2,
    TsaR = 3,
    // Coded slice segment of STSA picture
    StsaN = 4,
    StsaR = 5,
    // Coded slice segment of RADL picture
    RadlN = 6,
    RadlR = 7,
    // Coded slice segment of RASL picture
    RaslN = 8,
    RaslR = 9,
    // Reserved VCL NAL units (10-15)
    RsvVclN10 = 10,
    RsvVclR11 = 11,
    RsvVclN12 = 12,
    RsvVclR13 = 13,
    RsvVclN14 = 14,
    RsvVclR15 = 15,
    // Coded slice segment of BLA picture
    BlaWLp = 16,
    BlaWRadl = 17,
    BlaNLp = 18,
    // Coded slice segment of IDR picture
    IdrWRadl = 19,
    IdrNLp = 20,
    // Coded slice segment of CRA picture
    CraNut = 21,
    // Reserved IRAP VCL NAL units (22-23)
    RsvIrapVcl22 = 22,
    RsvIrapVcl23 = 23,
    // Reserved VCL NAL units (24-31)
    RsvVcl24 = 24,
    RsvVcl25 = 25,
    RsvVcl26 = 26,
    RsvVcl27 = 27,
    RsvVcl28 = 28,
    RsvVcl29 = 29,
    RsvVcl30 = 30,
    RsvVcl31 = 31,
    // Video Parameter Set
    VpsNut = 32,
    // Sequence Parameter Set
    SpsNut = 33,
    // Picture Parameter Set
    PpsNut = 34,
    // Access Unit Delimiter
    AudNut = 35,
    // End of Sequence
    EosNut = 36,
    // End of Bitstream
    EobNut = 37,
    // Filler Data
    FdNut = 38,
    // Supplemental Enhancement Information (prefix)
    PrefixSeiNut = 39,
    // Supplemental Enhancement Information (suffix)
    SuffixSeiNut = 40,
    // Reserved (41-47)
    RsvNvcl41 = 41,
    RsvNvcl42 = 42,
    RsvNvcl43 = 43,
    RsvNvcl44 = 44,
    RsvNvcl45 = 45,
    RsvNvcl46 = 46,
    RsvNvcl47 = 47,
    // Unspecified (48-63)
    Unspec48 = 48,
    Unspec49 = 49,
    Unspec50 = 50,
    Unspec51 = 51,
    Unspec52 = 52,
    Unspec53 = 53,
    Unspec54 = 54,
    Unspec55 = 55,
    Unspec56 = 56,
    Unspec57 = 57,
    Unspec58 = 58,
    Unspec59 = 59,
    Unspec60 = 60,
    Unspec61 = 61,
    Unspec62 = 62,
    Unspec63 = 63,
}

impl NalUnitType {
    /// Create from raw u8 value.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::TrailN,
            1 => Self::TrailR,
            2 => Self::TsaN,
            3 => Self::TsaR,
            4 => Self::StsaN,
            5 => Self::StsaR,
            6 => Self::RadlN,
            7 => Self::RadlR,
            8 => Self::RaslN,
            9 => Self::RaslR,
            10 => Self::RsvVclN10,
            11 => Self::RsvVclR11,
            12 => Self::RsvVclN12,
            13 => Self::RsvVclR13,
            14 => Self::RsvVclN14,
            15 => Self::RsvVclR15,
            16 => Self::BlaWLp,
            17 => Self::BlaWRadl,
            18 => Self::BlaNLp,
            19 => Self::IdrWRadl,
            20 => Self::IdrNLp,
            21 => Self::CraNut,
            22 => Self::RsvIrapVcl22,
            23 => Self::RsvIrapVcl23,
            32 => Self::VpsNut,
            33 => Self::SpsNut,
            34 => Self::PpsNut,
            35 => Self::AudNut,
            36 => Self::EosNut,
            37 => Self::EobNut,
            38 => Self::FdNut,
            39 => Self::PrefixSeiNut,
            40 => Self::SuffixSeiNut,
            _ if value <= 31 => Self::RsvVcl31,
            _ if value <= 47 => Self::RsvNvcl47,
            _ => Self::Unspec63,
        }
    }

    /// Check if this is a VCL (Video Coding Layer) NAL unit.
    pub fn is_vcl(&self) -> bool {
        (*self as u8) <= 31
    }

    /// Check if this is an IRAP (Intra Random Access Point) picture.
    pub fn is_irap(&self) -> bool {
        matches!(
            self,
            Self::BlaWLp
                | Self::BlaWRadl
                | Self::BlaNLp
                | Self::IdrWRadl
                | Self::IdrNLp
                | Self::CraNut
        )
    }

    /// Check if this is an IDR picture.
    pub fn is_idr(&self) -> bool {
        matches!(self, Self::IdrWRadl | Self::IdrNLp)
    }

    /// Check if this is a BLA picture.
    pub fn is_bla(&self) -> bool {
        matches!(self, Self::BlaWLp | Self::BlaWRadl | Self::BlaNLp)
    }

    /// Check if this is a CRA picture.
    pub fn is_cra(&self) -> bool {
        matches!(self, Self::CraNut)
    }

    /// Check if this is a RASL picture.
    pub fn is_rasl(&self) -> bool {
        matches!(self, Self::RaslN | Self::RaslR)
    }

    /// Check if this is a RADL picture.
    pub fn is_radl(&self) -> bool {
        matches!(self, Self::RadlN | Self::RadlR)
    }

    /// Check if this is a leading picture (RASL or RADL).
    pub fn is_leading(&self) -> bool {
        self.is_rasl() || self.is_radl()
    }

    /// Check if this is a trailing picture.
    pub fn is_trailing(&self) -> bool {
        matches!(self, Self::TrailN | Self::TrailR)
    }

    /// Check if this is a reference picture (R suffix).
    pub fn is_reference(&self) -> bool {
        matches!(
            self,
            Self::TrailR
                | Self::TsaR
                | Self::StsaR
                | Self::RadlR
                | Self::RaslR
                | Self::RsvVclR11
                | Self::RsvVclR13
                | Self::RsvVclR15
        ) || self.is_irap()
    }

    /// Get human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::TrailN => "TRAIL_N",
            Self::TrailR => "TRAIL_R",
            Self::TsaN => "TSA_N",
            Self::TsaR => "TSA_R",
            Self::StsaN => "STSA_N",
            Self::StsaR => "STSA_R",
            Self::RadlN => "RADL_N",
            Self::RadlR => "RADL_R",
            Self::RaslN => "RASL_N",
            Self::RaslR => "RASL_R",
            Self::BlaWLp => "BLA_W_LP",
            Self::BlaWRadl => "BLA_W_RADL",
            Self::BlaNLp => "BLA_N_LP",
            Self::IdrWRadl => "IDR_W_RADL",
            Self::IdrNLp => "IDR_N_LP",
            Self::CraNut => "CRA_NUT",
            Self::VpsNut => "VPS_NUT",
            Self::SpsNut => "SPS_NUT",
            Self::PpsNut => "PPS_NUT",
            Self::AudNut => "AUD_NUT",
            Self::EosNut => "EOS_NUT",
            Self::EobNut => "EOB_NUT",
            Self::FdNut => "FD_NUT",
            Self::PrefixSeiNut => "PREFIX_SEI_NUT",
            Self::SuffixSeiNut => "SUFFIX_SEI_NUT",
            _ => "RESERVED/UNSPEC",
        }
    }
}

/// HEVC NAL unit header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NalUnitHeader {
    /// NAL unit type (6 bits).
    pub nal_unit_type: NalUnitType,
    /// Layer ID (6 bits).
    pub nuh_layer_id: u8,
    /// Temporal ID + 1 (3 bits). Actual temporal ID = nuh_temporal_id_plus1 - 1.
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
    /// Total size including header and any start code.
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

    /// Check if this is an IDR picture.
    pub fn is_idr(&self) -> bool {
        self.header.nal_unit_type.is_idr()
    }

    /// Check if this is a CRA picture.
    pub fn is_cra(&self) -> bool {
        self.header.nal_unit_type.is_cra()
    }

    /// Check if this is a BLA picture.
    pub fn is_bla(&self) -> bool {
        self.header.nal_unit_type.is_bla()
    }

    /// Check if this is an IRAP picture.
    pub fn is_irap(&self) -> bool {
        self.header.nal_unit_type.is_irap()
    }

    /// Get RBSP data (payload without emulation prevention bytes).
    pub fn rbsp(&self) -> &[u8] {
        &self.payload
    }
}

/// Parse NAL unit header from 2 bytes.
pub fn parse_nal_header(data: &[u8]) -> Result<NalUnitHeader> {
    if data.len() < 2 {
        return Err(HevcError::InsufficientData {
            expected: 2,
            actual: data.len(),
        });
    }

    let mut reader = BitReader::new(data);

    // forbidden_zero_bit (1 bit, must be 0)
    let forbidden = reader.read_bit()?;
    if forbidden {
        return Err(HevcError::InvalidData(
            "forbidden_zero_bit is not zero".to_string(),
        ));
    }

    // nal_unit_type (6 bits)
    let nal_type_raw = reader.read_bits(6)? as u8;
    let nal_unit_type = NalUnitType::from_u8(nal_type_raw);

    // nuh_layer_id (6 bits)
    let nuh_layer_id = reader.read_bits(6)? as u8;

    // nuh_temporal_id_plus1 (3 bits)
    let nuh_temporal_id_plus1 = reader.read_bits(3)? as u8;

    Ok(NalUnitHeader {
        nal_unit_type,
        nuh_layer_id,
        nuh_temporal_id_plus1,
    })
}

/// Find NAL unit start codes in data.
/// Returns offsets of each NAL unit (after the start code).
pub fn find_nal_units(data: &[u8]) -> Vec<(usize, usize)> {
    let mut units = Vec::new();
    let mut i = 0;

    while i < data.len() {
        // Look for start code: 0x000001 or 0x00000001
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 {
            let (start_code_len, nal_start) =
                if i + 3 < data.len() && data[i + 2] == 0x00 && data[i + 3] == 0x01 {
                    // 4-byte start code
                    (4, i + 4)
                } else if data[i + 2] == 0x01 {
                    // 3-byte start code
                    (3, i + 3)
                } else {
                    i += 1;
                    continue;
                };

            // Find the end of this NAL unit (next start code or end of data)
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

        // Skip the 2-byte header
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
        assert!(NalUnitType::IdrWRadl.is_reference());

        assert!(NalUnitType::CraNut.is_cra());
        assert!(NalUnitType::CraNut.is_irap());

        assert!(NalUnitType::TrailR.is_vcl());
        assert!(NalUnitType::TrailR.is_reference());
        assert!(!NalUnitType::TrailN.is_reference());

        assert!(!NalUnitType::SpsNut.is_vcl());
        assert!(!NalUnitType::PpsNut.is_vcl());
    }

    #[test]
    fn test_parse_nal_header() {
        // IDR_W_RADL, layer_id=0, temporal_id_plus1=1
        // 0100_1100 1000_0001 = 0x4C 0x81
        // forbidden=0, nal_type=0x13(19), layer_id=0, temporal_id_plus1=1
        let data = [0x26, 0x01]; // 0010_0110 0000_0001
        let header = parse_nal_header(&data).unwrap();
        assert_eq!(header.nal_unit_type, NalUnitType::IdrWRadl);
        assert_eq!(header.nuh_layer_id, 0);
        assert_eq!(header.nuh_temporal_id_plus1, 1);
    }

    #[test]
    fn test_find_nal_units() {
        let data = [
            0x00, 0x00, 0x00, 0x01, // Start code
            0x40, 0x01, // VPS header
            0xAA, 0xBB, // VPS payload
            0x00, 0x00, 0x01, // Start code (3-byte)
            0x42, 0x01, // SPS header
            0xCC, 0xDD, // SPS payload
        ];

        let units = find_nal_units(&data);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0], (4, 8));
        assert_eq!(units[1], (11, 15));
    }
}
