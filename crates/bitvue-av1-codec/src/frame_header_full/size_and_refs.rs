//! Frame size and, for inter frames, the reference list and inter tools: the part of
//! `uncompressed_header()` (spec 5.9.2) between the `ref_order_hint[]` list and
//! `disable_frame_end_update_cdf`, split out of `parse_frame_header_full`. Covers
//! `frame_size()`, `render_size()`, `frame_refs_short_signaling`, `ref_frame_idx[]`,
//! `frame_size_with_refs()`, `allow_high_precision_mv`, `read_interpolation_filter()`,
//! `is_motion_mode_switchable` and `use_ref_frame_mvs` (spec 5.9.5-5.9.10).

use super::frame_size::{read_frame_size, read_interpolation_filter, read_render_size, FrameSize};
use super::*;

/// What the earlier part of the header decided, which this part depends on.
pub(super) struct SizeAndRefsInput {
    pub(super) frame_is_intra: bool,
    pub(super) frame_size_override_flag: bool,
    pub(super) allow_screen_content_tools: bool,
    pub(super) force_integer_mv: bool,
    pub(super) error_resilient_mode: bool,
}

/// Everything this part of the header yields. Intra frames leave the inter fields at their
/// defaults (`ref_frame_idx` all zero, the flags false).
pub(super) struct SizeAndRefs {
    pub(super) size: FrameSize,
    pub(super) allow_intrabc: bool,
    pub(super) ref_frame_idx: [u32; REFS_PER_FRAME],
    pub(super) allow_high_precision_mv: bool,
    pub(super) subpel_filter_switchable: bool,
    pub(super) switchable_motion_mode: bool,
    pub(super) use_ref_frame_mvs: bool,
}

pub(super) fn read_size_and_refs(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    input: &SizeAndRefsInput,
) -> Result<SizeAndRefs> {
    let SizeAndRefsInput {
        frame_is_intra,
        frame_size_override_flag,
        allow_screen_content_tools,
        force_integer_mv,
        error_resilient_mode,
    } = *input;
    if frame_is_intra {
        let mut allow_intrabc = false;
        let size = read_frame_size(reader, seq, frame_size_override_flag)?;
        read_render_size(reader)?;
        if allow_screen_content_tools && size.upscaled_width == size.width {
            allow_intrabc = reader.read_bit()?;
        }
        return Ok(SizeAndRefs {
            size,
            allow_intrabc,
            ref_frame_idx: [0u32; REFS_PER_FRAME],
            allow_high_precision_mv: false,
            subpel_filter_switchable: false,
            switchable_motion_mode: false,
            use_ref_frame_mvs: false,
        });
    }

    let mut ref_frame_idx = [0u32; REFS_PER_FRAME];
    let frame_refs_short_signaling = if seq.enable_order_hint {
        reader.read_bit()?
    } else {
        false
    };
    if frame_refs_short_signaling {
        return Err(err(
            "frame_refs_short_signaling (set_frame_refs) is not supported by parse_frame_header_full",
        ));
    }
    for slot in ref_frame_idx.iter_mut() {
        *slot = reader.read_bits(3)?;
        if seq.frame_id_numbers_present {
            let delta_len = seq.delta_frame_id_length_minus_2.unwrap_or(0) as u32 + 2;
            reader.read_bits(delta_len as u8)?; // delta_frame_id_minus_1
        }
    }
    if frame_size_override_flag && !error_resilient_mode {
        // frame_size_with_refs(): if found_ref via any ref frame's stored size, reuse it;
        // otherwise frame_size()+render_size(). This parser doesn't track ref frame
        // dimensions (only order hints), so it can't take the found_ref=1 shortcut path --
        // always falls through to a real frame_size()+render_size() read below. This is only
        // wrong (reads bits that shouldn't be there) if a *real* encoder actually emits
        // found_ref=1, which requires explicit ref-size reuse signaling support the encoder
        // must opt into; flagged here as a real accuracy gap, not silently assumed away.
        for _ in 0..REFS_PER_FRAME {
            if reader.read_bit()? {
                return Err(err(
                    "frame_size_with_refs' found_ref=1 path is not supported by parse_frame_header_full",
                ));
            }
        }
    }
    let size = read_frame_size(reader, seq, frame_size_override_flag)?;
    read_render_size(reader)?;

    let allow_high_precision_mv = if force_integer_mv {
        false
    } else {
        reader.read_bit()?
    };
    let subpel_filter_switchable = read_interpolation_filter(reader)?;
    let switchable_motion_mode = reader.read_bit()?; // is_motion_mode_switchable
    let use_ref_frame_mvs = if error_resilient_mode || !seq.enable_ref_frame_mvs {
        false
    } else {
        reader.read_bit()?
    };
    Ok(SizeAndRefs {
        size,
        allow_intrabc: false,
        ref_frame_idx,
        allow_high_precision_mv,
        subpel_filter_switchable,
        switchable_motion_mode,
        use_ref_frame_mvs,
    })
}
