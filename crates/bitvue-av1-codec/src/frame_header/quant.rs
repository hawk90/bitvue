//! `quantization_params()` (5.9.12), `delta_q_params()` (5.9.17) and `delta_lf_params()` (5.9.18).

use crate::bitreader::BitReader;
use bitvue_engine::BitvueError;

/// Read a signed delta-Q value: 1-bit present flag, then su(7) if present.
/// Per AV1 spec Section 5.9.14 (read_delta_q).
pub(super) fn read_delta_q(reader: &mut BitReader) -> Result<Option<i8>, BitvueError> {
    let delta_coded = reader.read_bit()?;
    if delta_coded {
        // su(7): signed 7-bit value in [-64, 63]
        let val = reader.read_su(7)?;
        Ok(Some(val.clamp(-128, 127) as i8))
    } else {
        Ok(Some(0i8))
    }
}

/// `quantization_params()` (spec 5.9.12): the frame's base index and its five delta-Q terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuantizationParams {
    pub base_q_idx: u8,
    pub y_dc: i8,
    pub u_dc: i8,
    pub u_ac: i8,
    pub v_dc: i8,
    pub v_ac: i8,
}

impl QuantizationParams {
    /// Spec `CodedLossless` needs every delta to be zero (as well as `base_q_idx == 0`).
    pub(crate) fn all_deltas_zero(&self) -> bool {
        self.y_dc == 0 && self.u_dc == 0 && self.u_ac == 0 && self.v_dc == 0 && self.v_ac == 0
    }
}

/// Parse quantization_params() per AV1 spec Section 5.9.12.
///
/// `num_planes` and `separate_uv_delta_q` come from the sequence header color config; a stream
/// with `separate_uv_delta_q` (rav1e writes it) codes a `diff_uv_delta` flag before the U deltas
/// and, when set, separate V deltas, and a `qm_v` of its own. The V terms equal the U ones
/// otherwise. A monochrome stream has no chroma deltas at all.
pub(crate) fn parse_quantization_params(
    reader: &mut BitReader,
    num_planes: u8,
    separate_uv_delta_q: bool,
) -> Result<QuantizationParams, BitvueError> {
    let delta = |reader: &mut BitReader| -> Result<i8, BitvueError> {
        Ok(read_delta_q(reader)?.unwrap_or(0))
    };
    // base_q_idx  u(8)
    let base_q_idx = reader.read_bits(8)? as u8;
    let y_dc = delta(reader)?;
    let (mut u_dc, mut u_ac, mut v_dc, mut v_ac) = (0, 0, 0, 0);
    if num_planes > 1 {
        let diff_uv_delta = separate_uv_delta_q && reader.read_bit()?;
        u_dc = delta(reader)?;
        u_ac = delta(reader)?;
        if diff_uv_delta {
            v_dc = delta(reader)?;
            v_ac = delta(reader)?;
        } else {
            v_dc = u_dc;
            v_ac = u_ac;
        }
    }
    // using_qmatrix (1 bit), then qm_y, qm_u and -- only with separate U/V deltas -- qm_v.
    if reader.read_bit()? {
        reader.read_bits(4)?;
        reader.read_bits(4)?;
        if separate_uv_delta_q {
            reader.read_bits(4)?;
        }
    }
    Ok(QuantizationParams {
        base_q_idx,
        y_dc,
        u_dc,
        u_ac,
        v_dc,
        v_ac,
    })
}

/// Parse delta_q_params() per AV1 spec Section 5.9.17.
///
/// Returns `delta_q_present` flag.
pub(super) fn parse_delta_q_params(
    reader: &mut BitReader,
    base_q_idx: u8,
) -> Result<bool, BitvueError> {
    let delta_q_present = if base_q_idx > 0 {
        reader.read_bit()?
    } else {
        false
    };

    if delta_q_present {
        // delta_q_res  u(2)
        reader.read_bits(2)?;
    }

    Ok(delta_q_present)
}
