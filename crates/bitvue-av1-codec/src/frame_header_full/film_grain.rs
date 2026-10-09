//! `film_grain_params()` (spec 5.9.30).

use super::*;

pub(super) fn parse_film_grain_params(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    show_frame: bool,
    showable_frame: bool,
    frame_type: FrameType,
) -> Result<FilmGrainInfo> {
    if !seq.film_grain_params_present || (!show_frame && !showable_frame) {
        return Ok(FilmGrainInfo::default());
    }
    let apply_grain = reader.read_bit()?;
    if !apply_grain {
        return Ok(FilmGrainInfo::default());
    }
    let seed = reader.read_bits(16)? as u64;
    let update_grain = if frame_type == FrameType::Inter {
        reader.read_bit()?
    } else {
        true
    };
    if !update_grain {
        reader.read_bits(3)?; // film_grain_params_ref_idx -- values would come from a loaded
                              // reference's params, which this parser doesn't track (out of
                              // scope: same "no cross-frame film-grain state" limitation as
                              // PrevGmParams, but here it affects a real value, not just bit
                              // consumption -- flagged, not silently guessed).
        return Ok(FilmGrainInfo {
            enabled: true,
            seed,
            ..Default::default()
        });
    }

    let num_y_points = reader.read_bits(4)?;
    for _ in 0..num_y_points {
        reader.read_bits(8)?; // point_y_value
        reader.read_bits(8)?; // point_y_scaling
    }
    let chroma_scaling_from_luma = if seq.color_config.mono_chrome {
        false
    } else {
        reader.read_bit()?
    };
    let (num_cb_points, num_cr_points) = if seq.color_config.mono_chrome
        || chroma_scaling_from_luma
        || (seq.color_config.subsampling_x && seq.color_config.subsampling_y && num_y_points == 0)
    {
        (0, 0)
    } else {
        let cb = reader.read_bits(4)?;
        for _ in 0..cb {
            reader.read_bits(8)?;
            reader.read_bits(8)?;
        }
        let cr = reader.read_bits(4)?;
        for _ in 0..cr {
            reader.read_bits(8)?;
            reader.read_bits(8)?;
        }
        (cb, cr)
    };
    let grain_scaling_shift = (reader.read_bits(2)? + 8) as u8;
    let ar_coeff_lag = reader.read_bits(2)? as u8;
    let num_pos_luma = 2 * ar_coeff_lag as u32 * (ar_coeff_lag as u32 + 1);
    let num_pos_chroma = if num_y_points > 0 {
        num_pos_luma + 1
    } else {
        num_pos_luma
    };
    let mut ar_coeffs_y = Vec::new();
    if num_y_points > 0 {
        ar_coeffs_y.reserve(num_pos_luma as usize);
        for _ in 0..num_pos_luma {
            let raw = reader.read_bits(8)?;
            ar_coeffs_y.push((raw as i32 - 128) as i8);
        }
    }
    let mut ar_coeffs_uv = Vec::new();
    if chroma_scaling_from_luma || num_cb_points > 0 {
        for _ in 0..num_pos_chroma {
            let raw = reader.read_bits(8)?;
            ar_coeffs_uv.push((raw as i32 - 128) as i8);
        }
    }
    if chroma_scaling_from_luma || num_cr_points > 0 {
        for _ in 0..num_pos_chroma {
            reader.read_bits(8)?; // ar_coeffs_cr -- appended separately from ar_coeffs_uv's Cb
                                  // values in the spec; FilmGrainInfo has one combined
                                  // ar_coeffs_uv field, so Cr values aren't separately kept
                                  // (matches the existing FilmGrainInfo shape, not a new gap).
        }
    }
    let ar_coeff_shift = (reader.read_bits(2)? + 6) as u8;
    let grain_scale_shift = reader.read_bits(2)? as u8;
    if num_cb_points > 0 {
        reader.read_bits(8)?; // cb_mult
        reader.read_bits(8)?; // cb_luma_mult
        reader.read_bits(9)?; // cb_offset
    }
    if num_cr_points > 0 {
        reader.read_bits(8)?; // cr_mult
        reader.read_bits(8)?; // cr_luma_mult
        reader.read_bits(9)?; // cr_offset
    }
    let overlap = reader.read_bit()?;
    let clip_to_restricted_range = reader.read_bit()?;

    Ok(FilmGrainInfo {
        enabled: true,
        update_offset: 0, // AV1 spec has no such field; FilmGrainInfo's field predates this
        // parser and appears unused by advanced_features::extract_film_grain_data
        // (only reads enabled/seed/scaling_shift/ar_coeff_lag/chroma_scaling_from_luma/overlap).
        seed,
        scaling_shift: grain_scaling_shift,
        ar_coeff_lag,
        ar_coeffs_y,
        ar_coeffs_uv,
        ar_coeff_shift,
        grain_scale_shift,
        chroma_scaling_from_luma,
        overlap,
        clip_to_restricted_range,
    })
}
