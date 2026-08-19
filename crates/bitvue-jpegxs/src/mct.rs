//! JPEG XS Multiple Component Transform (MCT) marker parser.
//!
//! The MCT marker (0xFF72) describes the inter-component decorrelation
//! transform applied before coding. Common configurations:
//!   - No MCT (identity)
//!   - RCT (Reversible Colour Transform): YCgCo lossless variant
//!   - ICT (Irreversible Colour Transform): YCbCr float coefficients

use crate::error::Result;

/// MCT type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MctType {
    /// No transform (components coded independently).
    None,
    /// Reversible Colour Transform (lossless — integer arithmetic).
    Rct,
    /// Irreversible Colour Transform (lossy — float arithmetic).
    Ict,
    /// Custom matrix (coefficients stored in payload).
    Custom,
    Unknown(u8),
}

impl From<u8> for MctType {
    fn from(v: u8) -> Self {
        match v & 0x0F {
            0 => MctType::None,
            1 => MctType::Rct,
            2 => MctType::Ict,
            3 => MctType::Custom,
            other => MctType::Unknown(other),
        }
    }
}

/// Parsed MCT marker.
#[derive(Debug, Clone)]
pub struct MctParams {
    pub mct_type: MctType,
    /// Number of input components.
    pub num_comps_in: u8,
    /// Number of output components.
    pub num_comps_out: u8,
    /// Custom matrix coefficients (row-major, only for MctType::Custom).
    pub matrix: Vec<f32>,
}

/// Parse an MCT marker payload.
pub fn parse_mct(data: &[u8]) -> Result<MctParams> {
    if data.is_empty() {
        return Ok(MctParams {
            mct_type: MctType::None,
            num_comps_in: 0,
            num_comps_out: 0,
            matrix: Vec::new(),
        });
    }

    let mct_type = MctType::from(data[0]);
    let num_comps_in = if data.len() > 1 {
        (data[1] >> 4) + 1
    } else {
        0
    };
    let num_comps_out = if data.len() > 1 {
        (data[1] & 0x0F) + 1
    } else {
        0
    };

    let mut matrix = Vec::new();
    if mct_type == MctType::Custom && data.len() > 2 {
        let coeff_bytes = &data[2..];
        let n = coeff_bytes.len() / 4;
        for i in 0..n {
            let f = f32::from_be_bytes([
                coeff_bytes[i * 4],
                coeff_bytes[i * 4 + 1],
                coeff_bytes[i * 4 + 2],
                coeff_bytes[i * 4 + 3],
            ]);
            matrix.push(f);
        }
    }

    Ok(MctParams {
        mct_type,
        num_comps_in,
        num_comps_out,
        matrix,
    })
}
