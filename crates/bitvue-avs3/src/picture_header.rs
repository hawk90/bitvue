//! AVS3 Picture Header parsing (GB/T 33475.3 §6.2.3 / §6.2.4).
//!
//! AVS3 has two picture header types:
//!   - I-picture header (SCI = 0xB3)
//!   - P/B-picture header (SCI = 0xB6)
//!
//! Both carry display_order, QP base values, and filter on/off flags.

use crate::bitreader::BitReader;
use crate::error::{Avs3Error, Result};
use serde::{Deserialize, Serialize};

/// AVS3 picture type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PictureType {
    /// Intra picture (I-frame)
    I,
    /// Predictive picture (P-frame)
    P,
    /// Bi-directional picture (B-frame)
    B,
}

/// Per-picture QP and filter parameters extracted from picture header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PictureHeader {
    /// Picture type.
    pub picture_type: PictureType,
    /// Display order (picture_output_delay or BBV delay derived).
    pub display_delay: u32,
    /// Base quantization parameter for luma (6 bits).
    pub picture_qp: u8,
    /// Deblocking filter flag (may override seq-level).
    pub deblocking_filter_disable: bool,
    /// Loop filter level delta for this picture.
    pub loop_filter_alf_enable: bool,
    /// ESAO enabled at picture level.
    pub esao_enable: bool,
    /// CCSAO enabled at picture level.
    pub ccsao_enable: bool,
    /// Random access flag.
    pub random_access_decodable: bool,
}

/// Parse I-picture header from NAL unit data starting at SCI byte (0xB3).
pub fn parse_i_picture_header(data: &[u8]) -> Result<PictureHeader> {
    if data.len() < 4 {
        return Err(Avs3Error::TooShort {
            need: 4,
            have: data.len(),
        });
    }
    // Skip SCI byte
    let mut r = BitReader::new(&data[1..]);
    parse_picture_header_common(&mut r, PictureType::I)
}

/// Parse P/B-picture header from NAL unit data starting at SCI byte (0xB6).
pub fn parse_pb_picture_header(data: &[u8]) -> Result<PictureHeader> {
    if data.len() < 4 {
        return Err(Avs3Error::TooShort {
            need: 4,
            have: data.len(),
        });
    }
    let mut r = BitReader::new(&data[1..]);
    // picture_type: 2 bits (0=P, 1=B)
    let pt_raw = r.read_bits(2)? as u8;
    let picture_type = if pt_raw == 0 {
        PictureType::P
    } else {
        PictureType::B
    };
    parse_picture_header_common(&mut r, picture_type)
}

fn parse_picture_header_common(
    r: &mut BitReader<'_>,
    picture_type: PictureType,
) -> Result<PictureHeader> {
    // BBV delay / display order (16 bits)
    let display_delay = r.read_bits(16)?;

    // Random access decodable flag (only for non-B)
    let random_access_decodable = if picture_type != PictureType::B {
        r.read_flag()?
    } else {
        false
    };

    // Skip temporal_id (3 bits)
    let _ = r.read_bits(3)?;

    // Picture QP (7 bits: 1 fixed_qp flag + 6 qp value)
    let _fixed_qp = r.read_flag()?;
    let picture_qp = r.read_bits(7)? as u8;

    // Loop filter flags
    let loop_filter_alf_enable = r.read_flag()?;
    let deblocking_filter_disable = r.read_flag()?;

    // ESAO / CCSAO picture-level enable flags
    let esao_enable = r.read_flag()?;
    let ccsao_enable = r.read_flag()?;

    Ok(PictureHeader {
        picture_type,
        display_delay,
        picture_qp,
        deblocking_filter_disable,
        loop_filter_alf_enable,
        esao_enable,
        ccsao_enable,
        random_access_decodable,
    })
}
