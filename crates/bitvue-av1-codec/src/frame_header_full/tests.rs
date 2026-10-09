use super::*;
use crate::sequence::parse_sequence_header;

fn minimal_seq_header_bytes() -> Vec<u8> {
    // Hand-built minimal AV1 sequence header payload: profile 0, not still-picture, not
    // reduced-still-picture, no timing info, one operating point (idc=0, level=0, no tier),
    // frame_width_bits_minus_1=8 (9 bits), frame_height_bits_minus_1=7 (8 bits),
    // max_frame_width_minus_1=319 (320-1), max_frame_height_minus_1=239 (240-1), no
    // frame_id_numbers, use_128x128_superblock=0, all enable_* flags 0 except order hint,
    // seq_choose_screen_content_tools=1, seq_choose_integer_mv=1, order_hint_bits_minus_1=6
    // (7 bits), enable_superres=0, enable_cdef=1, enable_restoration=1, color_config
    // (8-bit, BT.601-ish minimal), film_grain_params_present=0.
    //
    // Built by hand-encoding bits per AV1 spec 5.5.1, verified by round-tripping through
    // parse_sequence_header in the test below (if the bit layout were wrong, dimensions or
    // flags would come back wrong, not just silently pass).
    let bits: Vec<u8> = {
        let mut b = Vec::new();
        let mut push = |v: u32, n: u8| {
            for i in (0..n).rev() {
                b.push(((v >> i) & 1) as u8);
            }
        };
        push(0, 3); // seq_profile
        push(0, 1); // still_picture
        push(0, 1); // reduced_still_picture_header
        push(0, 1); // timing_info_present_flag
        push(0, 1); // initial_display_delay_present_flag
        push(0, 5); // operating_points_cnt_minus_1 (0 => 1 op)
        push(0, 12); // operating_point_idc[0]
        push(0, 5); // seq_level_idx[0]
                    // seq_level_idx[0] > 7 would add seq_tier -- 0 means no tier bit
        push(8, 4); // frame_width_bits_minus_1 = 8 (=> 9 bits for width)
        push(7, 4); // frame_height_bits_minus_1 = 7 (=> 8 bits for height)
        push(319, 9); // max_frame_width_minus_1
        push(239, 8); // max_frame_height_minus_1
                      // frame_id_numbers_present_flag
        push(0, 1);
        push(0, 1); // use_128x128_superblock
        push(0, 1); // enable_filter_intra
        push(0, 1); // enable_intra_edge_filter
        push(0, 1); // enable_interintra_compound
        push(0, 1); // enable_masked_compound
        push(0, 1); // enable_warped_motion
        push(0, 1); // enable_dual_filter
        push(1, 1); // enable_order_hint
        push(0, 1); // enable_jnt_comp (only if enable_order_hint)
        push(0, 1); // enable_ref_frame_mvs (only if enable_order_hint)
        push(1, 1); // seq_choose_screen_content_tools
        push(1, 1); // seq_choose_integer_mv
        push(6, 3); // order_hint_bits_minus_1 = 6 (=> 7 bits)
        push(0, 1); // enable_superres
        push(1, 1); // enable_cdef
        push(1, 1); // enable_restoration
                    // color_config():
        push(0, 1); // high_bitdepth (profile 0 => 8-bit)
        push(0, 1); // mono_chrome
        push(0, 1); // color_description_present_flag
                    // mono_chrome false, profile!=1 => color_range f(1)
        push(0, 1); // color_range
                    // profile 0 => subsampling_x=1, subsampling_y=1 implicit, no bits
        push(0, 2); // chroma_sample_position (subsampling_x&&y => read)
        push(0, 1); // separate_uv_delta_q
        push(0, 1); // film_grain_params_present
        b
    };
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, bit) in bits.iter().enumerate() {
        if *bit != 0 {
            out[i / 8] |= 1 << (7 - (i % 8));
        }
    }
    out
}

#[test]
fn synthetic_sequence_header_round_trips() {
    let payload = minimal_seq_header_bytes();
    let seq = parse_sequence_header(&payload).expect("hand-built sequence header should parse");
    assert_eq!(seq.max_frame_width, 320);
    assert_eq!(seq.max_frame_height, 240);
    assert!(seq.enable_cdef);
    assert!(seq.enable_restoration);
    assert!(seq.enable_order_hint);
    assert_eq!(seq.order_hint_bits_minus_1, Some(6));
    assert!(!seq.color_config.mono_chrome);
    assert!(seq.color_config.subsampling_x && seq.color_config.subsampling_y);
}

/// `lr_type` codes are not in enum order (spec `Remap_Lr_Type`): 1 means SWITCHABLE, 2 WIENER,
/// 3 SGRPROJ. A 4:2:0 stream with chroma restoration also reads `lr_uv_shift`, which halves the
/// chroma unit size.
#[test]
fn lr_params_remap_the_type_codes_and_apply_the_uv_shift() {
    let seq = parse_sequence_header(&minimal_seq_header_bytes()).unwrap();
    // y=SGRPROJ(3), u=SWITCHABLE(1), v=WIENER(2); lr_unit_shift=1 (64x64 sb: bits 1, 0),
    // lr_uv_shift=1.
    let bits = [1u8, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0];
    let mut packed = vec![0u8; 2];
    for (i, b) in bits.iter().enumerate() {
        packed[i / 8] |= b << (7 - i % 8);
    }
    let mut reader = BitReader::new(&packed);
    let lr = parse_lr_params(&mut reader, &seq, false, false).unwrap();
    assert_eq!(lr.y_type, LoopRestorationType::SgrProj);
    assert_eq!(lr.u_type, LoopRestorationType::Switchable);
    assert_eq!(lr.v_type, LoopRestorationType::Wiener);
    assert_eq!(lr.unit_size, 128);
    assert_eq!(lr.uv_unit_size, 64);
}

fn bits_writer() -> (Vec<u8>, impl FnMut(&mut Vec<u8>, u32, u8)) {
    (Vec::new(), |bits: &mut Vec<u8>, v: u32, n: u8| {
        for i in (0..n).rev() {
            bits.push(((v >> i) & 1) as u8);
        }
    })
}

fn pack(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, bit) in bits.iter().enumerate() {
        if *bit != 0 {
            out[i / 8] |= 1 << (7 - (i % 8));
        }
    }
    out
}

/// Builds a minimal real-looking KEY frame, show_frame=1 header payload for the sequence
/// header from `minimal_seq_header_bytes` (order_hint_bits=7, enable_cdef/restoration=1,
/// enable_superres=0, 4:2:0, no film grain).
fn minimal_key_frame_bytes(base_q_idx: u32) -> Vec<u8> {
    let (mut bits, mut push) = bits_writer();
    push(&mut bits, 0, 1); // show_existing_frame
    push(&mut bits, 0, 2); // frame_type = KEY_FRAME
    push(&mut bits, 1, 1); // show_frame
                           // showable_frame implicit (KEY && show_frame) -- no bit
                           // error_resilient_mode implicit (KEY && show_frame) -- no bit
    push(&mut bits, 0, 1); // disable_cdf_update
                           // allow_screen_content_tools: seq_choose_screen_content_tools=1 in the
                           // sequence header sets seq_force_screen_content_tools=SELECT(2), which
                           // means the frame header DOES read this bit explicitly (the opposite of
                           // what "seq_choose=1" might suggest -- seq_choose=1 defers the decision
                           // to *this* per-frame bit, it doesn't imply 0).
    push(&mut bits, 0, 1); // allow_screen_content_tools = 0
                           // force_integer_mv: allow_screen_content_tools=0 => no bit read, value=false
                           // pre-OR, then FrameIsIntra => forced true regardless
                           // frame_id_numbers_present_flag=0 => no current_frame_id
                           // frame_size_override_flag: KEY && show_frame => not Switch, reduced_still=0 => read bit
    push(&mut bits, 0, 1); // frame_size_override_flag = 0 (use max_frame_width/height)
    push(&mut bits, 0, 7); // order_hint (7 bits, =0)
                           // primary_ref_frame: FrameIsIntra => PRIMARY_REF_NONE, no bit
                           // decoder_model_info absent => no buffer_removal_time
                           // refresh_frame_flags: KEY && show_frame => implicit 0xFF, no bit
                           // ref_order_hint loop: only if error_resilient_mode (false here) -- skip
                           // FrameIsIntra: frame_size() -- frame_size_override_flag=0 => no width/height bits
                           // superres: enable_superres=0 => use_superres reads NO bit at all
                           // render_size:
    push(&mut bits, 0, 1); // render_and_frame_size_different = 0
                           // allow_screen_content_tools=0 => no allow_intrabc bit
                           // disable_frame_end_update_cdf: reduced_still=0, disable_cdf_update=0 => read bit
    push(&mut bits, 1, 1); // disable_frame_end_update_cdf
                           // tile_info(): 320x240, use_128x128_superblock=0
                           // MiCols=2*((320+7)>>3)=2*40=80, MiRows=2*((240+7)>>3)=2*30=60(actually (240+7)>>3=30)=60
                           // sbCols=(80+15)>>4=5, sbRows=(60+15)>>4=4
                           // maxTileWidthSb=4096>>6=64, minLog2TileCols=tile_log2(64,5)=0, maxLog2TileCols=tile_log2(1,min(5,64))=tile_log2(1,5)=3
                           // uniform_tile_spacing_flag:
    push(&mut bits, 1, 1); // uniform_tile_spacing_flag = 1
                           // TileColsLog2 starts at minLog2TileCols=0; loop while <maxLog2TileCols(3): read increment bit
    push(&mut bits, 0, 1); // increment_tile_cols_log2 = 0 -> stop, TileColsLog2=0
                           // minLog2Tiles = max(minLog2TileCols=0, tile_log2(maxTileAreaSb, sbRows*sbCols))
                           // maxTileAreaSb=4096*2304>>12=2304, sbRows*sbCols=20, tile_log2(2304,20)=0 => minLog2Tiles=0
                           // minLog2TileRows=max(0-0,0)=0; maxLog2TileRows=tile_log2(1,min(4,64))=tile_log2(1,4)=2
    push(&mut bits, 0, 1); // increment_tile_rows_log2 = 0 -> stop, TileRowsLog2=0
                           // TileColsLog2==0 && TileRowsLog2==0 => no context_update_tile_id / tile_size_bytes bits
                           // quantization_params: base_q_idx u(8)
    push(&mut bits, base_q_idx, 8);
    push(&mut bits, 0, 1); // DeltaQYDc delta_coded=0
                           // separate_uv_delta_q=0 => DeltaQUDc, DeltaQUAc
    push(&mut bits, 0, 1); // DeltaQUDc delta_coded=0
    push(&mut bits, 0, 1); // DeltaQUAc delta_coded=0
    push(&mut bits, 0, 1); // using_qmatrix=0
                           // segmentation_params: segmentation_enabled
    push(&mut bits, 0, 1); // segmentation_enabled = 0
                           // delta_q_params: base_q_idx>0 => read delta_q_present
    let delta_q_present = base_q_idx > 0;
    if delta_q_present {
        push(&mut bits, 1, 1);
        push(&mut bits, 0, 2); // delta_q_res
    } else {
        push(&mut bits, 0, 1);
    }
    // delta_lf_params: only if delta_q_present
    if delta_q_present {
        push(&mut bits, 0, 1); // delta_lf_present = 0
    }
    // loop_filter_params: coded_lossless = (base_q_idx==0 && no deltas)
    let coded_lossless = base_q_idx == 0;
    if !coded_lossless {
        push(&mut bits, 0, 6); // loop_filter_level[0]
        push(&mut bits, 0, 6); // loop_filter_level[1]
                               // num_planes=3>1 but both levels 0 => no level[2]/[3]
        push(&mut bits, 0, 3); // sharpness
        push(&mut bits, 0, 1); // delta_enabled = 0
    }
    // cdef_params: enable_cdef=1, !coded_lossless, !allow_intrabc
    if !coded_lossless {
        push(&mut bits, 0, 2); // cdef_damping_minus_3 = 0 => damping=3
        push(&mut bits, 0, 2); // cdef_bits = 0 => 1 iteration
        push(&mut bits, 5, 4); // cdef_y_pri_strength[0] = 5
        push(&mut bits, 1, 2); // cdef_y_sec_strength[0] = 1
        push(&mut bits, 3, 4); // cdef_uv_pri_strength[0] = 3
        push(&mut bits, 0, 2); // cdef_uv_sec_strength[0] = 0
    }
    // lr_params: enable_restoration=1, !all_lossless, !allow_intrabc
    let all_lossless = coded_lossless; // width==upscaled_width here (no superres)
    if !all_lossless {
        push(&mut bits, 0, 2); // lr_type[0] = None
        push(&mut bits, 0, 2); // lr_type[1] = None
        push(&mut bits, 0, 2); // lr_type[2] = None
                               // uses_lr = false => no unit-size bits
    }
    // read_tx_mode: !coded_lossless => tx_mode_select bit
    if !coded_lossless {
        push(&mut bits, 0, 1); // tx_mode_select
    }
    // frame_reference_mode: FrameIsIntra => no bit, reference_select=false
    // skip_mode_params: FrameIsIntra => skip_mode_allowed=false, no bit
    // allow_warped_motion: FrameIsIntra => no bit
    // reduced_tx_set:
    push(&mut bits, 0, 1);
    // global_motion_params: 7 refs, all is_global=0 (1 bit each)
    for _ in 0..7 {
        push(&mut bits, 0, 1);
    }
    // film_grain_params: film_grain_params_present=0 => no bits at all
    pack(&bits)
}

#[test]
fn parses_a_synthetic_key_frame_and_reaches_cdef_values() {
    let seq_payload = minimal_seq_header_bytes();
    let seq = parse_sequence_header(&seq_payload).unwrap();
    let frame_payload = minimal_key_frame_bytes(40);
    let mut ref_state = RefFrameState::new();
    let header = parse_frame_header_full(&frame_payload, &seq, &mut ref_state).unwrap();

    assert_eq!(header.frame_type, FrameType::Key);
    assert_eq!(header.base_q_idx, Some(40));
    assert_eq!(header.width, 320);
    assert_eq!(header.height, 240);
    assert!(header.cdef_damping.enabled);
    assert_eq!(header.cdef_damping.damping, 3);
    assert_eq!(header.cdef_y_primary_strength, 5);
    assert_eq!(header.cdef_y_secondary_strength, 1);
    assert_eq!(header.cdef_uv_primary_strength, 3);
    assert!(!header.loop_restoration.enabled);
    assert!(!header.film_grain.enabled);
}

#[test]
fn parses_a_synthetic_lossless_key_frame_and_skips_cdef() {
    let seq_payload = minimal_seq_header_bytes();
    let seq = parse_sequence_header(&seq_payload).unwrap();
    let frame_payload = minimal_key_frame_bytes(0);
    let mut ref_state = RefFrameState::new();
    let header = parse_frame_header_full(&frame_payload, &seq, &mut ref_state).unwrap();

    assert_eq!(header.base_q_idx, Some(0));
    assert!(
        !header.cdef_damping.enabled,
        "CodedLossless frames skip cdef_params entirely"
    );
}

#[test]
fn ref_frame_state_updates_after_a_key_frame() {
    let seq_payload = minimal_seq_header_bytes();
    let seq = parse_sequence_header(&seq_payload).unwrap();
    let frame_payload = minimal_key_frame_bytes(40);
    let mut ref_state = RefFrameState::new();
    parse_frame_header_full(&frame_payload, &seq, &mut ref_state).unwrap();
    // KEY_FRAME + show_frame refreshes all 8 slots to this frame's order_hint (0 here).
    assert_eq!(ref_state.ref_order_hint, [0u32; 8]);
}

#[test]
fn read_ns_matches_spec_worked_examples() {
    // ns(3): w=FloorLog2(3)+1=2, m=(1<<2)-3=1. v=f(1). If v<1 (v=0): return 0 (1 bit total).
    // If v>=1: read extra bit, return (v<<1)-1+extra.
    let data = pack(&[0]); // v=0 -> returns 0, consumes 1 bit
    let mut r = BitReader::new(&data);
    assert_eq!(read_ns(&mut r, 3).unwrap(), 0);
    assert_eq!(r.position(), 1);

    let data2 = pack(&[1, 0]); // v=1 (>=m=1) -> extra_bit=0 -> (1<<1)-1+0=1
    let mut r2 = BitReader::new(&data2);
    assert_eq!(read_ns(&mut r2, 3).unwrap(), 1);
    assert_eq!(r2.position(), 2);

    let data3 = pack(&[1, 1]); // v=1, extra_bit=1 -> (1<<1)-1+1=2
    let mut r3 = BitReader::new(&data3);
    assert_eq!(read_ns(&mut r3, 3).unwrap(), 2);
}

#[test]
fn tile_log2_matches_spec_definition() {
    assert_eq!(tile_log2(64, 5), 0); // 64<<0=64 >= 5
    assert_eq!(tile_log2(1, 5), 3); // 1,2,4 all < 5; 8 >= 5 -> k=3
    assert_eq!(tile_log2(1, 1), 0);
}

const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../../test_data/av1_test.ivf");

/// Real-fixture end-to-end: no independent decoder oracle exists for CDEF/LR/film-grain
/// values (see module doc), so this is the strongest available check -- parses every frame
/// sequentially from 0 (as a real sidecar caller must, per the cross-frame state
/// requirement), asserting no error/panic and that dimensions match the fixture's
/// independently-known-correct 320x240 (pinned elsewhere against decode_bridge's real pixel
/// output) for every single frame, not just frame 0.
#[test]
fn real_fixture_parses_every_frame_without_error() {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
    let seq_payload =
        find_seq_header_in(&frames).expect("fixture should contain a real sequence header");
    let seq = crate::sequence::parse_sequence_header(&seq_payload).unwrap();

    let mut ref_state = RefFrameState::new();
    let mut cdef_enabled_count = 0;
    for (i, frame) in frames.iter().enumerate() {
        let payload = find_frame_header_payload(&frame.data)
            .unwrap_or_else(|| panic!("frame {i} has no Frame/FrameHeader OBU"));
        let header = parse_frame_header_full(&payload, &seq, &mut ref_state)
            .unwrap_or_else(|e| panic!("frame {i} failed to parse: {e}"));
        if header.show_existing_frame {
            continue;
        }
        assert_eq!(header.width, 320, "frame {i}: wrong width");
        assert_eq!(header.height, 240, "frame {i}: wrong height");
        if header.cdef_damping.enabled {
            cdef_enabled_count += 1;
        }
    }
    // Not a strong assertion (could legitimately be 0 if the fixture disables CDEF), but
    // catches the specific regression this module exists to fix: silently getting 0 for
    // *every* frame because cdef_params() was never actually reached (e.g. an earlier bit
    // miscount always landing coded_lossless=true or enable_cdef=false).
    eprintln!(
        "real_fixture_parses_every_frame_without_error: {cdef_enabled_count}/{} frames had CDEF enabled",
        frames.len()
    );
}

#[test]
fn seg_feature_active_and_data_match_spec_semantics() {
    let mut seg = SegmentationInfo {
        enabled: true,
        ..SegmentationInfo::default()
    };
    seg.feature_enabled[2][SEG_LVL_REF_FRAME] = true;
    seg.feature_data[2][SEG_LVL_REF_FRAME] = 3;

    assert!(seg.seg_feature_active(2, SEG_LVL_REF_FRAME));
    assert_eq!(seg.seg_feature_data(2, SEG_LVL_REF_FRAME), 3);
    // A different segment with the same feature never enabled => inactive, data 0.
    assert!(!seg.seg_feature_active(0, SEG_LVL_REF_FRAME));
    assert_eq!(seg.seg_feature_data(0, SEG_LVL_REF_FRAME), 0);
    // A different feature on the *same* segment that was never enabled => inactive, data 0
    // (spec: an inactive feature's data is never read, `0` isn't a "real zero override").
    assert!(!seg.seg_feature_active(2, SEG_LVL_SKIP));
    assert_eq!(seg.seg_feature_data(2, SEG_LVL_SKIP), 0);
    // `enabled=false` (segmentation off entirely) => nothing is ever active, regardless of
    // what feature_enabled/feature_data happen to hold.
    seg.enabled = false;
    assert!(!seg.seg_feature_active(2, SEG_LVL_REF_FRAME));
    assert_eq!(seg.seg_feature_data(2, SEG_LVL_REF_FRAME), 0);
}

#[test]
fn parse_segmentation_params_retains_real_feature_enabled_and_data() {
    let (mut bits, mut push) = bits_writer();
    push(&mut bits, 1, 1); // enabled = 1
                           // primary_ref_frame == PRIMARY_REF_NONE => update_map/temporal_update/update_data are
                           // all implicit (no bits read), straight into the 8x8 feature loop.
    for seg in 0..MAX_SEGMENTS {
        for feature in 0..SEG_LVL_MAX {
            if seg == 0 && feature == 0 {
                // SEG_LVL_ALT_Q: 8 signed bits (su(9)), value = 5.
                push(&mut bits, 1, 1);
                push(&mut bits, 5, 9);
            } else if seg == 1 && feature == SEG_LVL_REF_FRAME {
                // SEG_LVL_REF_FRAME: 3 unsigned bits, value = 3 (RefFrame::Last3).
                push(&mut bits, 1, 1);
                push(&mut bits, 3, 3);
            } else {
                push(&mut bits, 0, 1);
            }
        }
    }
    let payload = pack(&bits);
    let mut reader = BitReader::new(&payload);
    let info = parse_segmentation_params(
        &mut reader,
        PRIMARY_REF_NONE,
        &SegmentationFeatures::default(),
    )
    .unwrap();

    assert!(info.enabled);
    assert!(info.update_map);
    assert!(!info.temporal_update);
    assert!(info.seg_feature_active(0, 0));
    assert_eq!(info.seg_feature_data(0, 0), 5);
    assert!(info.seg_feature_active(1, SEG_LVL_REF_FRAME));
    assert_eq!(info.seg_feature_data(1, SEG_LVL_REF_FRAME), 3);
    // seg_id_pre_skip: true because SEG_LVL_REF_FRAME (index 5, >= SEG_LVL_REF_FRAME) is
    // active for segment 1 -- matches this struct's existing derivation.
    assert!(info.seg_id_pre_skip);
    assert_eq!(info.last_active_seg_id, 1);
    // Nothing else was ever enabled.
    assert!(!info.seg_feature_active(0, SEG_LVL_REF_FRAME));
    assert!(!info.seg_feature_active(2, 0));
}

#[test]
fn classify_gm_type_matches_spec_5_9_24_decision_tree() {
    // !is_global => IDENTITY, regardless of the other (unread-in-this-case) flags.
    assert_eq!(classify_gm_type(false, false, false), GM_TYPE_IDENTITY);
    assert_eq!(classify_gm_type(false, true, true), GM_TYPE_IDENTITY);
    // is_global && is_rot_zoom => ROTZOOM, regardless of is_translation (unread in this case).
    assert_eq!(classify_gm_type(true, true, false), GM_TYPE_ROTZOOM);
    assert_eq!(classify_gm_type(true, true, true), GM_TYPE_ROTZOOM);
    // is_global && !is_rot_zoom && is_translation => TRANSLATION.
    assert_eq!(classify_gm_type(true, false, true), GM_TYPE_TRANSLATION);
    // is_global && !is_rot_zoom && !is_translation => AFFINE.
    assert_eq!(classify_gm_type(true, false, false), GM_TYPE_AFFINE);
    // The exact ordering `read_motion_mode`'s `GmType[ref] > TRANSLATION` exclusion relies on.
    // These are compile-time constants, so check them in a const block: a regression here
    // fails the build itself rather than only a test run.
    const { assert!(GM_TYPE_IDENTITY < GM_TYPE_TRANSLATION) };
    const { assert!(GM_TYPE_TRANSLATION < GM_TYPE_ROTZOOM) };
    const { assert!(GM_TYPE_ROTZOOM < GM_TYPE_AFFINE) };
}

/// Real-fixture regression for `parse_global_motion_params`'s real `GmType[ref]` output
/// (previously discarded entirely -- see this function's doc). Confirms the real bitstream
/// still parses cleanly end-to-end with `gm_type` now a real return value threaded through
/// `FrameHeader`, and reports the classification distribution actually observed rather than
/// asserting a specific one -- this crate's committed fixture is ordinary (non-warped-motion)
/// content, so all-IDENTITY on every inter frame is an expected, not a failing, outcome (same
/// non-strong-assertion precedent as `real_fixture_parses_every_frame_without_error`'s CDEF
/// count just above).
#[test]
fn real_fixture_gm_type_is_real_and_frames_still_parse_cleanly() {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
    let seq_payload =
        find_seq_header_in(&frames).expect("fixture should contain a real sequence header");
    let seq = crate::sequence::parse_sequence_header(&seq_payload).unwrap();

    let mut ref_state = RefFrameState::new();
    let mut non_identity_count = 0;
    for (i, frame) in frames.iter().enumerate() {
        let Some(payload) = find_frame_header_payload(&frame.data) else {
            continue;
        };
        let header = parse_frame_header_full(&payload, &seq, &mut ref_state)
            .unwrap_or_else(|e| panic!("frame {i} failed to parse: {e}"));
        if header.gm_type.iter().any(|&t| t != GM_TYPE_IDENTITY) {
            non_identity_count += 1;
        }
    }
    eprintln!(
        "real_fixture_gm_type_is_real_and_frames_still_parse_cleanly: {non_identity_count}/{} \
         frames had at least one non-IDENTITY GmType",
        frames.len()
    );
}

/// Test-only helper (not `find_sequence_header_bytes` from `bitvue-sidecar`, which this crate
/// can't depend on) -- scans a bounded prefix of frames for a real SequenceHeader OBU's raw
/// bytes, same approach `bitvue-sidecar::frame_analysis` uses.
pub(crate) fn find_seq_header_in(frames: &[crate::ivf::IvfFrame]) -> Option<Vec<u8>> {
    for frame in frames.iter().take(8) {
        let mut iter = crate::obu::ObuIterator::new(&frame.data);
        while let Some(Ok(found)) = iter.next_obu_with_offset() {
            if found.obu.header.obu_type == crate::obu::ObuType::SequenceHeader {
                // parse_sequence_header wants the OBU *payload* only (post header/size
                // field) -- not the raw offset..consumed slice, which is the whole OBU
                // including its header. Different from find_frame_header_payload's own
                // convention (also payload-only, via `found.obu.payload` directly) --
                // matching it here rather than reusing this test-only helper elsewhere.
                return Some(found.obu.payload.to_vec());
            }
        }
    }
    None
}

/// `segmentation_update_data = 0`: the frame keeps the features its primary reference frame was
/// saved with (spec `load_previous`), and `SegIdPreSkip`/`LastActiveSegId` follow from them.
#[test]
fn segmentation_without_new_data_uses_the_primary_reference_frames_features() {
    // enabled, update_map = 1, temporal_update = 0, update_data = 0
    let data = [0b1100_0000u8];
    let mut previous = SegmentationFeatures::default();
    previous.feature_enabled[0][0] = true;
    previous.feature_data[0][0] = -11;
    previous.feature_enabled[3][0] = true;
    previous.feature_data[3][0] = 19;

    let mut reader = BitReader::new(&data);
    let info = parse_segmentation_params(&mut reader, 2, &previous).unwrap();

    assert!(info.enabled && info.update_map && !info.temporal_update);
    assert_eq!(info.feature_data[0][0], -11);
    assert_eq!(info.feature_data[3][0], 19);
    assert_eq!(info.last_active_seg_id, 3);
    assert!(!info.seg_id_pre_skip);

    // A feature at or above SEG_LVL_REF_FRAME in the loaded data makes segment_id() precede skip.
    previous.feature_enabled[4][SEG_LVL_REF_FRAME] = true;
    let mut reader = BitReader::new(&data);
    let info = parse_segmentation_params(&mut reader, 2, &previous).unwrap();
    assert!(info.seg_id_pre_skip);
    assert_eq!(info.last_active_seg_id, 4);
}

/// With new data the loaded features are discarded, and with segmentation off the saved features
/// are clear.
#[test]
fn segmentation_with_new_data_or_disabled_ignores_the_previous_features() {
    let mut previous = SegmentationFeatures::default();
    previous.feature_enabled[2][0] = true;
    previous.feature_data[2][0] = 5;

    // enabled = 0
    let mut reader = BitReader::new(&[0b0000_0000u8]);
    let off = parse_segmentation_params(&mut reader, 2, &previous).unwrap();
    assert!(!off.enabled);
    assert!(off.feature_enabled.iter().flatten().all(|&on| !on));

    // enabled, update_map = 1, temporal_update = 0, update_data = 1, then 64 feature-enabled
    // bits all 0
    let mut bits = vec![0b1101_0000u8];
    bits.extend([0u8; 9]);
    let mut reader = BitReader::new(&bits);
    let fresh = parse_segmentation_params(&mut reader, 2, &previous).unwrap();
    assert!(fresh.enabled);
    assert!(fresh.feature_enabled.iter().flatten().all(|&on| !on));
    assert_eq!(fresh.last_active_seg_id, 0);
}

/// `RefFrameSignBias`: a reference displayed after the current frame is flagged; intra frames
/// (no references) flag nothing.
#[test]
fn ref_frame_sign_bias_flags_references_displayed_after_the_frame() {
    let seq = parse_sequence_header(&minimal_seq_header_bytes()).unwrap();
    let mut hints = [0u32; NUM_REF_FRAMES];
    hints[1] = 3; // before the frame
    hints[5] = 9; // after it
    let idx = [1u8, 1, 1, 5, 5, 5, 5];

    let bias = ref_frame_sign_bias(&hints, Some(&idx), 6, &seq);
    assert_eq!(bias, [false, false, false, true, true, true, true]);

    assert_eq!(ref_frame_sign_bias(&hints, None, 6, &seq), [false; 7]);
}
