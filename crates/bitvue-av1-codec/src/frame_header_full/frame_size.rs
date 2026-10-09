//! Frame size, render size and interpolation filter (spec 5.9.5-5.9.10).

use super::*;

const SUPERRES_DENOM_BITS: u8 = 3;
const SUPERRES_DENOM_MIN: u32 = 9;
const SUPERRES_NUM: u32 = 8;
pub(super) struct FrameSize {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) upscaled_width: u32,
    pub(super) upscaled_height: u32,
    pub(super) use_superres: bool,
    pub(super) superres_denom: u32,
}

/// `frame_size()` + `superres_params()` (AV1 spec 5.9.5/5.9.9) combined -- superres only ever
/// scales width, never height (`UpscaledWidth` = the value read/defaulted here, `FrameWidth` is
/// then derived *from* it via the superres denominator; height passes through unchanged).
pub(super) fn read_frame_size(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    frame_size_override_flag: bool,
) -> Result<FrameSize> {
    let (upscaled_width, height) = if frame_size_override_flag {
        let w = reader.read_bits(seq.frame_width_bits_minus_1 + 1)? + 1;
        let h = reader.read_bits(seq.frame_height_bits_minus_1 + 1)? + 1;
        (w, h)
    } else {
        (seq.max_frame_width, seq.max_frame_height)
    };
    let use_superres = if seq.enable_superres {
        reader.read_bit()?
    } else {
        false
    };
    let denom = if use_superres {
        SUPERRES_DENOM_MIN + reader.read_bits(SUPERRES_DENOM_BITS)?
    } else {
        SUPERRES_NUM
    };
    let width = (upscaled_width * SUPERRES_NUM + denom / 2) / denom;
    Ok(FrameSize {
        width,
        height,
        upscaled_width,
        upscaled_height: height,
        use_superres,
        superres_denom: denom,
    })
}

pub(super) fn read_render_size(reader: &mut BitReader) -> Result<()> {
    let render_and_frame_size_different = reader.read_bit()?;
    if render_and_frame_size_different {
        reader.read_bits(16)?; // render_width_minus_1
        reader.read_bits(16)?; // render_height_minus_1
    }
    Ok(())
}

pub(super) fn read_interpolation_filter(reader: &mut BitReader) -> Result<bool> {
    let is_filter_switchable = reader.read_bit()?;
    if !is_filter_switchable {
        reader.read_bits(2)?; // interpolation_filter
    }
    Ok(is_filter_switchable)
}
