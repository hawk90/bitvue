//! Colour description of a sequence: primaries, transfer, matrix, chroma position and
//! `color_config()` (spec 5.5.2).

use serde::{Deserialize, Serialize};

use bitvue_engine::Result;

use super::Av1Profile;
use crate::bitreader::BitReader;

/// Color primaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ColorPrimaries {
    Bt709 = 1,
    Unspecified = 2,
    Bt470M = 4,
    Bt470Bg = 5,
    Bt601 = 6,
    Smpte240 = 7,
    GenericFilm = 8,
    Bt2020 = 9,
    Xyz = 10,
    Smpte431 = 11,
    Smpte432 = 12,
    Ebu3213 = 22,
    Reserved(u8),
}

impl ColorPrimaries {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => ColorPrimaries::Bt709,
            2 => ColorPrimaries::Unspecified,
            4 => ColorPrimaries::Bt470M,
            5 => ColorPrimaries::Bt470Bg,
            6 => ColorPrimaries::Bt601,
            7 => ColorPrimaries::Smpte240,
            8 => ColorPrimaries::GenericFilm,
            9 => ColorPrimaries::Bt2020,
            10 => ColorPrimaries::Xyz,
            11 => ColorPrimaries::Smpte431,
            12 => ColorPrimaries::Smpte432,
            22 => ColorPrimaries::Ebu3213,
            v => ColorPrimaries::Reserved(v),
        }
    }
}

/// Transfer characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TransferCharacteristics {
    Bt709 = 1,
    Unspecified = 2,
    Bt470M = 4,
    Bt470Bg = 5,
    Bt601 = 6,
    Smpte240 = 7,
    Linear = 8,
    Log100 = 9,
    Log100Sqrt10 = 10,
    Iec61966 = 11,
    Bt1361 = 12,
    Srgb = 13,
    Bt202010Bit = 14,
    Bt202012Bit = 15,
    Smpte2084 = 16,
    Smpte428 = 17,
    Hlg = 18,
    Reserved(u8),
}

impl TransferCharacteristics {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => TransferCharacteristics::Bt709,
            2 => TransferCharacteristics::Unspecified,
            4 => TransferCharacteristics::Bt470M,
            5 => TransferCharacteristics::Bt470Bg,
            6 => TransferCharacteristics::Bt601,
            7 => TransferCharacteristics::Smpte240,
            8 => TransferCharacteristics::Linear,
            9 => TransferCharacteristics::Log100,
            10 => TransferCharacteristics::Log100Sqrt10,
            11 => TransferCharacteristics::Iec61966,
            12 => TransferCharacteristics::Bt1361,
            13 => TransferCharacteristics::Srgb,
            14 => TransferCharacteristics::Bt202010Bit,
            15 => TransferCharacteristics::Bt202012Bit,
            16 => TransferCharacteristics::Smpte2084,
            17 => TransferCharacteristics::Smpte428,
            18 => TransferCharacteristics::Hlg,
            v => TransferCharacteristics::Reserved(v),
        }
    }
}

/// Matrix coefficients
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MatrixCoefficients {
    Identity = 0,
    Bt709 = 1,
    Unspecified = 2,
    Fcc = 4,
    Bt470Bg = 5,
    Bt601 = 6,
    Smpte240 = 7,
    YCgCo = 8,
    Bt2020Ncl = 9,
    Bt2020Cl = 10,
    Smpte2085 = 11,
    ChromaDerivedNcl = 12,
    ChromaDerivedCl = 13,
    ICtCp = 14,
    Reserved(u8),
}

impl MatrixCoefficients {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => MatrixCoefficients::Identity,
            1 => MatrixCoefficients::Bt709,
            2 => MatrixCoefficients::Unspecified,
            4 => MatrixCoefficients::Fcc,
            5 => MatrixCoefficients::Bt470Bg,
            6 => MatrixCoefficients::Bt601,
            7 => MatrixCoefficients::Smpte240,
            8 => MatrixCoefficients::YCgCo,
            9 => MatrixCoefficients::Bt2020Ncl,
            10 => MatrixCoefficients::Bt2020Cl,
            11 => MatrixCoefficients::Smpte2085,
            12 => MatrixCoefficients::ChromaDerivedNcl,
            13 => MatrixCoefficients::ChromaDerivedCl,
            14 => MatrixCoefficients::ICtCp,
            v => MatrixCoefficients::Reserved(v),
        }
    }
}

/// Chroma sample position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromaSamplePosition {
    Unknown = 0,
    Vertical = 1,
    Colocated = 2,
    Reserved = 3,
}

impl ChromaSamplePosition {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ChromaSamplePosition::Unknown,
            1 => ChromaSamplePosition::Vertical,
            2 => ChromaSamplePosition::Colocated,
            _ => ChromaSamplePosition::Reserved,
        }
    }
}

/// Color configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    pub bit_depth: u8,
    pub mono_chrome: bool,
    pub num_planes: u8,
    pub color_primaries: ColorPrimaries,
    pub transfer_characteristics: TransferCharacteristics,
    pub matrix_coefficients: MatrixCoefficients,
    pub color_range: bool,
    pub subsampling_x: bool,
    pub subsampling_y: bool,
    pub chroma_sample_position: ChromaSamplePosition,
    pub separate_uv_delta_q: bool,
}

impl ColorConfig {
    /// Returns the chroma subsampling as a string (e.g., "4:2:0", "4:4:4")
    pub fn chroma_subsampling_str(&self) -> &'static str {
        if self.mono_chrome {
            "4:0:0"
        } else if self.subsampling_x && self.subsampling_y {
            "4:2:0"
        } else if self.subsampling_x {
            "4:2:2"
        } else {
            "4:4:4"
        }
    }
}

pub(super) fn parse_color_config(
    reader: &mut BitReader,
    profile: &Av1Profile,
) -> Result<ColorConfig> {
    let bit_depth = read_bit_depth(reader, profile)?;

    let mono_chrome = if !matches!(profile, Av1Profile::High) {
        reader.read_bit()?
    } else {
        false
    };

    let num_planes = if mono_chrome { 1 } else { 3 };

    let (color_primaries, transfer_characteristics, matrix_coefficients) =
        read_color_description(reader)?;

    let (color_range, subsampling_x, subsampling_y, chroma_sample_position) = if mono_chrome {
        // color_range (1 bit)
        let color_range = reader.read_bit()?;
        (color_range, true, true, ChromaSamplePosition::Unknown)
    } else if matches!(color_primaries, ColorPrimaries::Bt709)
        && matches!(transfer_characteristics, TransferCharacteristics::Srgb)
        && matches!(matrix_coefficients, MatrixCoefficients::Identity)
    {
        (true, false, false, ChromaSamplePosition::Unknown)
    } else {
        // color_range (1 bit)
        let color_range = reader.read_bit()?;
        let (subsampling_x, subsampling_y) = read_subsampling(reader, profile, bit_depth)?;
        let chroma_sample_position = if subsampling_x && subsampling_y {
            ChromaSamplePosition::from_u8(reader.read_bits(2)? as u8)
        } else {
            ChromaSamplePosition::Unknown
        };
        (
            color_range,
            subsampling_x,
            subsampling_y,
            chroma_sample_position,
        )
    };

    let separate_uv_delta_q = if !mono_chrome {
        reader.read_bit()?
    } else {
        false
    };

    Ok(ColorConfig {
        bit_depth,
        mono_chrome,
        num_planes,
        color_primaries,
        transfer_characteristics,
        matrix_coefficients,
        color_range,
        subsampling_x,
        subsampling_y,
        chroma_sample_position,
        separate_uv_delta_q,
    })
}

/// high_bitdepth (+ twelve_bit for the Professional profile) -> bit depth
fn read_bit_depth(reader: &mut BitReader, profile: &Av1Profile) -> Result<u8> {
    // high_bitdepth (1 bit)
    let high_bitdepth = reader.read_bit()?;

    Ok(
        if matches!(profile, Av1Profile::Professional) && high_bitdepth {
            // twelve_bit (1 bit)
            if reader.read_bit()? {
                12
            } else {
                10
            }
        } else if high_bitdepth {
            // (profile 2 with high_bitdepth = 0 is 8-bit 4:2:2, spec 5.5.2; it was read as 10, #227)
            10
        } else {
            8
        },
    )
}

/// color_description_present_flag and, when set, the three 8-bit descriptors
fn read_color_description(
    reader: &mut BitReader,
) -> Result<(ColorPrimaries, TransferCharacteristics, MatrixCoefficients)> {
    // color_description_present_flag (1 bit)
    if reader.read_bit()? {
        Ok((
            ColorPrimaries::from_u8(reader.read_bits(8)? as u8),
            TransferCharacteristics::from_u8(reader.read_bits(8)? as u8),
            MatrixCoefficients::from_u8(reader.read_bits(8)? as u8),
        ))
    } else {
        Ok((
            ColorPrimaries::Unspecified,
            TransferCharacteristics::Unspecified,
            MatrixCoefficients::Unspecified,
        ))
    }
}

/// subsampling_x / subsampling_y: fixed by profile, except 12-bit Professional which signals them
fn read_subsampling(
    reader: &mut BitReader,
    profile: &Av1Profile,
    bit_depth: u8,
) -> Result<(bool, bool)> {
    Ok(if matches!(profile, Av1Profile::Main) {
        (true, true)
    } else if matches!(profile, Av1Profile::High) {
        (false, false)
    } else if bit_depth == 12 {
        let subsampling_x = reader.read_bit()?;
        let subsampling_y = if subsampling_x {
            reader.read_bit()?
        } else {
            false
        };
        (subsampling_x, subsampling_y)
    } else {
        (true, false)
    })
}
