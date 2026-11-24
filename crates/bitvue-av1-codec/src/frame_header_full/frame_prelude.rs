//! The first part of `uncompressed_header()` (spec 5.9.2) after `show_existing_frame`: frame type
//! and visibility, error resilience, screen-content and integer-MV flags, frame id, size override,
//! order hint, primary reference frame, refresh flags and the `ref_order_hint[]` list. Split out of
//! `parse_frame_header_full`.

use super::*;

/// The elements read before the frame size.
pub(super) struct FramePrelude {
    pub(super) frame_type: FrameType,
    pub(super) frame_is_intra: bool,
    pub(super) show_frame: bool,
    pub(super) showable_frame: bool,
    pub(super) error_resilient_mode: bool,
    pub(super) disable_cdf_update: bool,
    pub(super) allow_screen_content_tools: bool,
    pub(super) force_integer_mv: bool,
    pub(super) frame_size_override_flag: bool,
    pub(super) order_hint: u32,
    pub(super) primary_ref_frame: u32,
    pub(super) refresh_frame_flags: u32,
}

/// The temporal and spatial layer of the OBU carrying the header (`temporal_id`/`spatial_id`, spec
/// 5.3.3); only `buffer_removal_time` depends on them.
#[derive(Clone, Copy, Default)]
pub(super) struct Layer {
    pub(super) temporal_id: u8,
    pub(super) spatial_id: u8,
}

pub(super) fn read_frame_prelude(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    layer: Layer,
) -> Result<FramePrelude> {
    let order_hint_bits = seq
        .order_hint_bits_minus_1
        .map(|v| v as u32 + 1)
        .unwrap_or(0);

    let (frame_type, show_frame, showable_frame, error_resilient_mode) =
        read_frame_visibility(reader, seq)?;
    let frame_is_intra = frame_type.is_intra();

    let disable_cdf_update = reader.read_bit()?;
    let (allow_screen_content_tools, force_integer_mv) =
        read_screen_content_flags(reader, seq, frame_is_intra)?;
    skip_current_frame_id(reader, seq)?;

    let frame_size_override_flag = if frame_type == FrameType::Switch {
        true
    } else if seq.reduced_still_picture_header {
        false
    } else {
        reader.read_bit()?
    };

    let order_hint = if order_hint_bits > 0 {
        reader.read_bits(order_hint_bits as u8)?
    } else {
        0
    };

    let primary_ref_frame = if frame_is_intra || error_resilient_mode {
        PRIMARY_REF_NONE
    } else {
        reader.read_bits(3)?
    };

    skip_buffer_removal_times(reader, seq, layer)?;

    let refresh_frame_flags = read_refresh_frame_flags(
        reader,
        seq,
        (frame_type, show_frame, error_resilient_mode),
        order_hint_bits,
    )?;

    Ok(FramePrelude {
        frame_type,
        frame_is_intra,
        show_frame,
        showable_frame,
        error_resilient_mode,
        disable_cdf_update,
        allow_screen_content_tools,
        force_integer_mv,
        frame_size_override_flag,
        order_hint,
        primary_ref_frame,
        refresh_frame_flags,
    })
}

/// Whether the frame header carries `temporal_point_info()` (spec 5.9.2): the sequence has a
/// decoder model and the pictures are not at a constant interval.
pub(super) fn has_temporal_point_info(seq: &SequenceHeader) -> bool {
    seq.decoder_model_info.is_some()
        && !seq
            .timing_info
            .as_ref()
            .is_some_and(|t| t.equal_picture_interval)
}

/// `temporal_point_info()` (spec 5.9.31): `frame_presentation_time`, consumed but not kept.
pub(super) fn skip_temporal_point_info(reader: &mut BitReader, seq: &SequenceHeader) -> Result<()> {
    if let (true, Some(model)) = (
        has_temporal_point_info(seq),
        seq.decoder_model_info.as_ref(),
    ) {
        reader.read_bits_u64(model.frame_presentation_time_length_minus_1 + 1)?;
    }
    Ok(())
}

/// `buffer_removal_time_present_flag` and, for each operating point that has a decoder model and
/// contains this frame's layers, its `buffer_removal_time` (spec 5.9.2), consumed but not kept.
pub(super) fn skip_buffer_removal_times(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    layer: Layer,
) -> Result<()> {
    let Some(model) = seq.decoder_model_info.as_ref() else {
        return Ok(());
    };
    if !reader.read_bit()? {
        return Ok(());
    }
    for op in &seq.operating_points {
        if !op.decoder_model_present {
            continue;
        }
        let in_temporal_layer = (op.idc >> layer.temporal_id) & 1 == 1;
        let in_spatial_layer = (op.idc >> (layer.spatial_id + 8)) & 1 == 1;
        if op.idc == 0 || (in_temporal_layer && in_spatial_layer) {
            reader.read_bits_u64(model.buffer_removal_time_length_minus_1 + 1)?;
        }
    }
    Ok(())
}

/// `frame_type`, `show_frame`, `showable_frame` and `error_resilient_mode`, in that order.
fn read_frame_visibility(
    reader: &mut BitReader,
    seq: &SequenceHeader,
) -> Result<(FrameType, bool, bool, bool)> {
    let frame_type = if seq.reduced_still_picture_header {
        FrameType::Key
    } else {
        FrameType::from_av1_bits(reader.read_bits(2)?)
    };
    let show_frame = if seq.reduced_still_picture_header {
        true
    } else {
        reader.read_bit()?
    };
    if show_frame && !seq.reduced_still_picture_header {
        skip_temporal_point_info(reader, seq)?;
    }
    let showable_frame = if seq.reduced_still_picture_header {
        false
    } else if show_frame {
        frame_type != FrameType::Key
    } else {
        reader.read_bit()?
    };
    let error_resilient_mode = if seq.reduced_still_picture_header {
        false
    } else if frame_type == FrameType::Switch || (frame_type == FrameType::Key && show_frame) {
        true
    } else {
        reader.read_bit()?
    };
    Ok((frame_type, show_frame, showable_frame, error_resilient_mode))
}

/// `current_frame_id`: consumed but not kept.
fn skip_current_frame_id(reader: &mut BitReader, seq: &SequenceHeader) -> Result<()> {
    if seq.frame_id_numbers_present {
        let id_len = seq.additional_frame_id_length_minus_1.unwrap_or(0) as u32
            + seq.delta_frame_id_length_minus_2.unwrap_or(0) as u32
            + 3;
        reader.read_bits_u64(id_len as u8)?;
    }
    Ok(())
}

/// `refresh_frame_flags` and, for error-resilient frames that do not refresh every slot, the
/// `ref_order_hint[]` list (consumed but not kept). `state` is
/// `(frame_type, show_frame, error_resilient_mode)`.
fn read_refresh_frame_flags(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    state: (FrameType, bool, bool),
    order_hint_bits: u32,
) -> Result<u32> {
    let (frame_type, show_frame, error_resilient_mode) = state;
    let refresh_frame_flags: u32 =
        if frame_type == FrameType::Switch || (frame_type == FrameType::Key && show_frame) {
            0xFF
        } else {
            reader.read_bits(8)?
        };
    if (!frame_type.is_intra() || refresh_frame_flags != 0xFF)
        && error_resilient_mode
        && seq.enable_order_hint
    {
        for _ in 0..NUM_REF_FRAMES {
            reader.read_bits(order_hint_bits as u8)?; // ref_order_hint[i]
        }
    }
    Ok(refresh_frame_flags)
}

/// `allow_screen_content_tools` and `force_integer_mv`.
fn read_screen_content_flags(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    frame_is_intra: bool,
) -> Result<(bool, bool)> {
    let allow_screen_content_tools =
        if seq.seq_force_screen_content_tools == SELECT_SCREEN_CONTENT_TOOLS as u8 {
            reader.read_bit()?
        } else {
            seq.seq_force_screen_content_tools != 0
        };
    let force_integer_mv = if allow_screen_content_tools {
        if seq.seq_force_integer_mv == SELECT_INTEGER_MV as u8 {
            reader.read_bit()?
        } else {
            seq.seq_force_integer_mv != 0
        }
    } else {
        false
    } || frame_is_intra;
    Ok((allow_screen_content_tools, force_integer_mv))
}

#[cfg(test)]
#[path = "frame_prelude_test.rs"]
mod frame_prelude_test;
