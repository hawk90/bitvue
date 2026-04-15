//! JPEG XS Non-Linear Transform (NLT) marker parser.
//!
//! The NLT marker (0xFF63) describes a per-component tone-mapping applied
//! before encoding and inverted after decoding.
//!
//! NLT types (ISO 21122-1 Table A.8):
//!   0  Quadratic (QUA) — low-complexity
//!   1  Extended Quadratic (XQUAD) — extended precision
//!   2  Extended Exponential (XEXP) — HDR/WCG profiles

use crate::error::Result;

/// NLT type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NltType {
    Quadratic,
    ExtendedQuadratic,
    ExtendedExponential,
    Unknown(u8),
}

impl From<u8> for NltType {
    fn from(v: u8) -> Self {
        match v {
            0 => NltType::Quadratic,
            1 => NltType::ExtendedQuadratic,
            2 => NltType::ExtendedExponential,
            other => NltType::Unknown(other),
        }
    }
}

/// Per-component NLT descriptor.
#[derive(Debug, Clone)]
pub struct NltComponent {
    pub nlt_type: NltType,
    /// Threshold T1 (signed, used for Quadratic and Extended variants).
    pub t1: i32,
    /// Threshold T2.
    pub t2: i32,
    /// Exponent e (used for ExtendedExponential).
    pub e: u8,
}

/// Parsed NLT marker payload.
#[derive(Debug, Clone)]
pub struct NltParams {
    pub components: Vec<NltComponent>,
}

/// Parse an NLT marker payload.
pub fn parse_nlt(data: &[u8], num_comps: usize) -> Result<NltParams> {
    let mut components = Vec::with_capacity(num_comps);
    let mut pos = 0;

    for _ in 0..num_comps {
        if pos >= data.len() {
            break;
        }
        let nlt_type = NltType::from(data[pos] & 0x03);
        pos += 1;

        let (t1, t2, e) = match nlt_type {
            NltType::Quadratic => {
                // QUA: 2× 2-byte signed thresholds
                if pos + 4 > data.len() {
                    break;
                }
                let t1 = i16::from_be_bytes([data[pos], data[pos + 1]]) as i32;
                let t2 = i16::from_be_bytes([data[pos + 2], data[pos + 3]]) as i32;
                pos += 4;
                (t1, t2, 0u8)
            }
            NltType::ExtendedQuadratic => {
                if pos + 8 > data.len() {
                    break;
                }
                let t1 =
                    i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                let t2 = i32::from_be_bytes([
                    data[pos + 4],
                    data[pos + 5],
                    data[pos + 6],
                    data[pos + 7],
                ]);
                pos += 8;
                (t1, t2, 0u8)
            }
            NltType::ExtendedExponential => {
                if pos + 2 > data.len() {
                    break;
                }
                let t1 = data[pos] as i32;
                let e = data[pos + 1];
                pos += 2;
                (t1, 0, e)
            }
            NltType::Unknown(_) => (0, 0, 0),
        };

        components.push(NltComponent {
            nlt_type,
            t1,
            t2,
            e,
        });
    }

    Ok(NltParams { components })
}
