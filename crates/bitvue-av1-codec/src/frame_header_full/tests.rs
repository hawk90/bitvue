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

/// FNV-1a over a string, enough to pin a long `Debug` rendering to one number.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// `(headers parsed, key frames, shown-again frames, digest)`, see [`header_summary`].
type HeaderSummary = (usize, usize, usize, u64);

/// What `parse_frame_header_full` makes of every frame header of a clip, threading
/// `RefFrameState` like a sidecar caller does: `(headers parsed, key frames, shown-again frames,
/// FNV-1a of every header's full `Debug` text in order)`. A header that fails to parse
/// contributes its error text, so a parser that starts failing changes the digest too.
fn header_summary(clip: &[u8]) -> HeaderSummary {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(clip).unwrap();
    let seq = parse_sequence_header(&find_seq_header_in(&frames).unwrap()).unwrap();
    let mut ref_state = RefFrameState::new();
    let (mut count, mut key, mut shown_again) = (0, 0, 0);
    let mut rendered = String::new();
    for frame in &frames {
        for payload in find_frame_header_payloads(&frame.data) {
            count += 1;
            match parse_frame_header_full(&payload, &seq, &mut ref_state) {
                Ok(header) => {
                    key += usize::from(header.frame_type == FrameType::Key);
                    shown_again += usize::from(header.show_existing_frame);
                    rendered.push_str(&format!("{header:?}\n"));
                }
                Err(e) => rendered.push_str(&format!("ERR {e}\n")),
            }
        }
    }
    (count, key, shown_again, fnv1a(&rendered))
}

/// Every field of every frame header of every committed clip, pinned. The checks above only ask
/// "did it parse" or look at a handful of fields; this one fails when ANY field of ANY header
/// changes, which is what a refactor of `parse_frame_header_full` must not do. A failure means a
/// header parsed differently: find out which field before changing the numbers. The values were
/// recorded from the parser as it was before any split of `parse_frame_header_full`, not from
/// an independent oracle, so they prove "unchanged", not "correct" (the clips' symbol-level
/// agreement with dav1d, in `cu_parser/oracle_test.rs`, is what vouches for correctness). Re-recorded
/// when `FilmGrainInfo` gained the scaling points, the Cr coefficients and the chroma multipliers:
/// with those fields' text (all empty or zero in these clips) cut out of the `Debug` output the old
/// numbers came out again.
#[test]
fn every_frame_header_of_every_clip_is_unchanged() {
    const CLIPS: [(&str, &[u8], HeaderSummary); 6] = [
        (
            "av1_test",
            include_bytes!("../../../../test_data/av1_test.ivf"),
            (274, 1, 0, 588199865380866743),
        ),
        (
            "aomenc_testsrc2",
            include_bytes!("../../../../test_data/av1_aomenc_testsrc2.ivf"),
            (16, 4, 3, 4729721367337169264),
        ),
        (
            "aomenc_tilegroups",
            include_bytes!("../../../../test_data/av1_aomenc_tilegroups.ivf"),
            (12, 1, 0, 11656466471076927050),
        ),
        (
            "aomenc_tiles5",
            include_bytes!("../../../../test_data/av1_aomenc_tiles5.ivf"),
            (12, 1, 0, 7465563672542036348),
        ),
        (
            "rav1e_testsrc2",
            include_bytes!("../../../../test_data/av1_rav1e_testsrc2.ivf"),
            (17, 6, 5, 13681353827795637648),
        ),
        (
            "svtav1_testsrc2",
            include_bytes!("../../../../test_data/av1_svtav1_testsrc2.ivf"),
            (17, 6, 5, 13221103126010935209),
        ),
    ];
    let mismatches: Vec<String> = CLIPS
        .iter()
        .filter_map(|&(name, clip, expected)| {
            let got = header_summary(clip);
            (got != expected).then(|| format!("{name}: got {got:?}, expected {expected:?}"))
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

// ---- Hand-built headers for the branches no committed clip reaches ----------------------------
//
// Each test writes the header one syntax element at a time (AV1 spec 5.9.2 order) and asserts
// that the parser both read the values back and stopped exactly at the last bit written
// (`header_size_bits`). A parser that reads one bit too many or too few desyncs every later
// element, so the end position is the strongest single check on a header parser.

/// A header under construction, MSB first.
#[derive(Default)]
struct HeaderBits(Vec<u8>);

impl HeaderBits {
    fn put(&mut self, value: u32, width: u8) -> &mut Self {
        for i in (0..width).rev() {
            self.0.push(((value >> i) & 1) as u8);
        }
        self
    }

    fn bit_len(&self) -> u64 {
        self.0.len() as u64
    }

    fn bytes(&self) -> Vec<u8> {
        pack(&self.0)
    }
}

/// `tile_info()` through `read_tx_mode()` for a 320x240 4:2:0 frame with `base_q_idx = 40`, one
/// tile, no segmentation, no delta q/lf, loop filter level 0, CDEF with one strength set, no loop
/// restoration -- the same layout `minimal_key_frame_bytes` writes for the same sequence header.
fn put_tile_info_through_tx_mode(b: &mut HeaderBits) {
    b.put(1, 1).put(0, 1).put(0, 1); // uniform_tile_spacing_flag, no extra tile column/row bits
    b.put(40, 8).put(0, 4); // base_q_idx; DeltaQYDc, DeltaQUDc, DeltaQUAc, using_qmatrix = 0
    b.put(0, 1); // segmentation_enabled
    b.put(1, 1).put(0, 2); // delta_q_present, delta_q_res
    b.put(0, 1); // delta_lf_present
    b.put(0, 6).put(0, 6).put(0, 3).put(0, 1); // loop filter levels, sharpness, delta_enabled
    b.put(0, 2)
        .put(0, 2)
        .put(5, 4)
        .put(1, 2)
        .put(3, 4)
        .put(0, 2); // cdef
    b.put(0, 2).put(0, 2).put(0, 2); // lr_type for the three planes
    b.put(0, 1); // tx_mode_select
}

/// Which optional parts of an inter frame header to write.
struct InterHeader {
    error_resilient: bool,
    frame_size_override: bool,
    short_signaling: bool,
}

const INTER_ORDER_HINT: u32 = 5;
const INTER_PRIMARY_REF_FRAME: u32 = 2;
const INTER_REFRESH_FLAGS: u32 = 0b0000_0110;

/// An inter frame header from `show_existing_frame` through the `ref_frame_idx[]` loop (and the
/// `found_ref` flags, when `frame_size_override` is set without error resilience). Returns
/// whether `force_integer_mv` ended up set, which decides if `allow_high_precision_mv` is
/// present. Only `seq_force_screen_content_tools` of 1 or 2 and any `seq_force_integer_mv` are
/// supported.
fn put_inter_header_through_ref_idx(
    b: &mut HeaderBits,
    seq: &SequenceHeader,
    o: &InterHeader,
) -> bool {
    b.put(0, 1); // show_existing_frame
    b.put(1, 2); // frame_type = INTER_FRAME
    b.put(1, 1); // show_frame (showable_frame is implied)
    b.put(u32::from(o.error_resilient), 1);
    b.put(0, 1); // disable_cdf_update
    let allow_screen_content_tools = match seq.seq_force_screen_content_tools {
        2 => {
            b.put(0, 1);
            false
        }
        v => v != 0,
    };
    let mut force_integer_mv = false;
    if allow_screen_content_tools {
        match seq.seq_force_integer_mv {
            2 => {
                b.put(0, 1);
            }
            v => force_integer_mv = v != 0,
        }
    }
    if seq.frame_id_numbers_present {
        let id_len = seq.additional_frame_id_length_minus_1.unwrap()
            + seq.delta_frame_id_length_minus_2.unwrap()
            + 3;
        b.put(0xA5, id_len); // current_frame_id
    }
    b.put(u32::from(o.frame_size_override), 1);
    b.put(INTER_ORDER_HINT, 7);
    if !o.error_resilient {
        b.put(INTER_PRIMARY_REF_FRAME, 3);
    }
    b.put(INTER_REFRESH_FLAGS, 8);
    if o.error_resilient {
        for hint in 1..=8 {
            b.put(hint, 7); // ref_order_hint[i]
        }
    }
    b.put(u32::from(o.short_signaling), 1);
    for idx in 1..=7 {
        b.put(idx, 3); // ref_frame_idx[i]
        if seq.frame_id_numbers_present {
            b.put(0x5, seq.delta_frame_id_length_minus_2.unwrap() + 2); // delta_frame_id_minus_1
        }
    }
    if o.frame_size_override && !o.error_resilient {
        for _ in 0..7 {
            b.put(0, 1); // found_ref = 0
        }
    }
    force_integer_mv
}

pub(super) fn base_seq() -> SequenceHeader {
    parse_sequence_header(&minimal_seq_header_bytes()).unwrap()
}

fn parse_with(
    seq: &SequenceHeader,
    bits: &HeaderBits,
) -> (std::result::Result<FrameHeader, BitvueError>, RefFrameState) {
    let mut ref_state = RefFrameState::new();
    let result = parse_frame_header_full(&bits.bytes(), seq, &mut ref_state);
    (result, ref_state)
}

/// Every inter-frame element in order, with values that differ from each other, and a global
/// motion table with one ROTZOOM, one TRANSLATION and one AFFINE reference (the parameter bits
/// are all zero-valued `decode_subexp` codes: `0` then 3 bits).
#[test]
fn inter_frame_header_reads_every_element_and_stops_at_the_last_bit() {
    let mut seq = base_seq();
    seq.enable_ref_frame_mvs = true;
    seq.enable_warped_motion = true;
    let mut b = HeaderBits::default();
    let force_integer_mv = put_inter_header_through_ref_idx(
        &mut b,
        &seq,
        &InterHeader {
            error_resilient: false,
            frame_size_override: false,
            short_signaling: false,
        },
    );
    assert!(!force_integer_mv);
    b.put(0, 1); // render_and_frame_size_different
    b.put(1, 1); // allow_high_precision_mv
    b.put(1, 1); // is_filter_switchable
    b.put(1, 1); // is_motion_mode_switchable
    b.put(1, 1); // use_ref_frame_mvs
    b.put(0, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reference_select (so no skip_mode_present bit)
    b.put(1, 1); // allow_warped_motion
    b.put(1, 1); // reduced_tx_set
    let param = |b: &mut HeaderBits| {
        b.put(0, 1).put(0, 3);
    };
    // LAST: ROTZOOM = is_global, is_rot_zoom, then 4 parameters
    b.put(1, 1).put(1, 1);
    (0..4).for_each(|_| param(&mut b));
    // BWDREF-side neighbour: TRANSLATION = is_global, !is_rot_zoom, is_translation, 2 parameters
    b.put(1, 1).put(0, 1).put(1, 1);
    (0..2).for_each(|_| param(&mut b));
    // AFFINE = is_global, !is_rot_zoom, !is_translation, 6 parameters
    b.put(1, 1).put(0, 1).put(0, 1);
    (0..6).for_each(|_| param(&mut b));
    for _ in 0..4 {
        b.put(0, 1); // IDENTITY
    }

    let (result, ref_state) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert_eq!(h.frame_type, FrameType::Inter);
    assert!(h.show_frame && !h.show_existing_frame && !h.error_resilient_mode);
    assert_eq!((h.width, h.height), (320, 240));
    assert_eq!(h.order_hint, INTER_ORDER_HINT);
    assert_eq!(h.primary_ref_frame, INTER_PRIMARY_REF_FRAME);
    assert_eq!(h.refresh_frame_flags, Some(INTER_REFRESH_FLAGS as u8));
    assert_eq!(h.ref_frame_idx, Some([1, 2, 3, 4, 5, 6, 7]));
    assert!(h.allow_high_precision_mv && !h.force_integer_mv);
    assert!(h.subpel_filter_switchable && h.switchable_motion_mode && h.use_ref_frame_mvs);
    assert!(!h.disable_frame_end_update_cdf);
    assert!(!h.reference_select && !h.skip_mode_present);
    assert!(h.allow_warped_motion && h.reduced_tx_set);
    assert_eq!(h.base_q_idx, Some(40));
    assert_eq!(
        h.gm_type,
        [
            GM_TYPE_IDENTITY,
            GM_TYPE_ROTZOOM,
            GM_TYPE_TRANSLATION,
            GM_TYPE_AFFINE,
            GM_TYPE_IDENTITY,
            GM_TYPE_IDENTITY,
            GM_TYPE_IDENTITY,
            GM_TYPE_IDENTITY
        ]
    );
    assert_eq!(
        h.header_size_bits,
        b.bit_len(),
        "stopped at the last bit written"
    );
    // Slots 1 and 2 were refreshed with this frame's order hint, the rest were not.
    assert_eq!(
        ref_state.ref_order_hint,
        [0, 5, 5, 0, 0, 0, 0, 0],
        "refresh_frame_flags {INTER_REFRESH_FLAGS:#b}"
    );
}

/// With `error_resilient_mode`: no `primary_ref_frame`, `use_ref_frame_mvs` or
/// `allow_warped_motion` bits (even though the sequence enables both tools), and a
/// `ref_order_hint[8]` loop that no committed clip has.
#[test]
fn error_resilient_inter_frame_skips_three_elements_and_reads_the_ref_order_hint_loop() {
    let mut seq = base_seq();
    seq.enable_ref_frame_mvs = true;
    seq.enable_warped_motion = true;
    let mut b = HeaderBits::default();
    put_inter_header_through_ref_idx(
        &mut b,
        &seq,
        &InterHeader {
            error_resilient: true,
            frame_size_override: false,
            short_signaling: false,
        },
    );
    b.put(0, 1); // render_and_frame_size_different
    b.put(0, 1); // allow_high_precision_mv
    b.put(0, 1).put(1, 2); // is_filter_switchable = 0, interpolation_filter = 1
    b.put(0, 1); // is_motion_mode_switchable
                 // use_ref_frame_mvs: not present when error resilient
    b.put(1, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reference_select
                 // allow_warped_motion: not present when error resilient
    b.put(0, 1); // reduced_tx_set
    for _ in 0..7 {
        b.put(0, 1); // global motion: all IDENTITY
    }

    let (result, _) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert!(h.error_resilient_mode);
    assert_eq!(h.primary_ref_frame, PRIMARY_REF_NONE);
    assert!(!h.use_ref_frame_mvs && !h.allow_warped_motion);
    assert!(!h.subpel_filter_switchable && !h.switchable_motion_mode);
    assert!(h.disable_frame_end_update_cdf);
    assert_eq!(h.order_hint, INTER_ORDER_HINT);
    assert_eq!(
        h.header_size_bits,
        b.bit_len(),
        "stopped at the last bit written"
    );
}

/// `frame_id_numbers_present_flag`: a `current_frame_id` after the screen-content elements and a
/// `delta_frame_id_minus_1` after each `ref_frame_idx`.
#[test]
fn frame_id_numbers_add_one_id_and_one_delta_per_reference() {
    let mut seq = base_seq();
    seq.frame_id_numbers_present = true;
    seq.additional_frame_id_length_minus_1 = Some(3);
    seq.delta_frame_id_length_minus_2 = Some(2);
    let mut b = HeaderBits::default();
    put_inter_header_through_ref_idx(
        &mut b,
        &seq,
        &InterHeader {
            error_resilient: false,
            frame_size_override: false,
            short_signaling: false,
        },
    );
    b.put(0, 1); // render_and_frame_size_different
    b.put(0, 1); // allow_high_precision_mv
    b.put(1, 1); // is_filter_switchable
    b.put(0, 1); // is_motion_mode_switchable
                 // use_ref_frame_mvs: enable_ref_frame_mvs is 0 in this sequence
    b.put(0, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reference_select
                 // allow_warped_motion: enable_warped_motion is 0 in this sequence
    b.put(0, 1); // reduced_tx_set
    for _ in 0..7 {
        b.put(0, 1);
    }

    let (result, _) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert_eq!(h.ref_frame_idx, Some([1, 2, 3, 4, 5, 6, 7]));
    assert_eq!(h.order_hint, INTER_ORDER_HINT);
    assert_eq!(
        h.header_size_bits,
        b.bit_len(),
        "8-bit current_frame_id and 7 four-bit deltas"
    );
}

/// A sequence that forces screen content tools on and integer MVs on: neither element has a bit,
/// `force_integer_mv` is set, and so `allow_high_precision_mv` has none either.
#[test]
fn forced_screen_content_tools_and_integer_mv_remove_their_bits() {
    let mut seq = base_seq();
    seq.seq_force_screen_content_tools = 1;
    seq.seq_force_integer_mv = 1;
    let mut b = HeaderBits::default();
    let force_integer_mv = put_inter_header_through_ref_idx(
        &mut b,
        &seq,
        &InterHeader {
            error_resilient: false,
            frame_size_override: false,
            short_signaling: false,
        },
    );
    assert!(force_integer_mv);
    b.put(0, 1); // render_and_frame_size_different
                 // allow_high_precision_mv: not present with force_integer_mv
    b.put(1, 1); // is_filter_switchable
    b.put(0, 1); // is_motion_mode_switchable
    b.put(0, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reference_select
    b.put(0, 1); // reduced_tx_set
    for _ in 0..7 {
        b.put(0, 1);
    }

    let (result, _) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert!(h.allow_screen_content_tools && h.force_integer_mv);
    assert!(!h.allow_high_precision_mv);
    assert_eq!(h.header_size_bits, b.bit_len());
}

/// `reduced_still_picture_header`: no `show_existing_frame`, `frame_type`, `show_frame` or
/// `disable_frame_end_update_cdf` bits, and a KEY frame that is shown.
#[test]
fn reduced_still_picture_header_key_frame_has_far_fewer_elements() {
    let mut seq = base_seq();
    seq.reduced_still_picture_header = true;
    seq.still_picture = true;
    seq.enable_order_hint = false;
    seq.order_hint_bits_minus_1 = None;
    let mut b = HeaderBits::default();
    b.put(0, 1); // disable_cdf_update
    b.put(0, 1); // allow_screen_content_tools (sequence says SELECT)
                 // frame_size_override_flag, order_hint, primary_ref_frame, refresh_frame_flags: implied
    b.put(0, 1); // render_and_frame_size_different
                 // allow_intrabc: allow_screen_content_tools is 0
                 // disable_frame_end_update_cdf: implied 1
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reduced_tx_set
    let intended_end = b.bit_len();
    // `global_motion_params()` is not called for an intra frame, so these 7 trailing bits must
    // NOT be consumed: the header has to end right after `reduced_tx_set`.
    for _ in 0..7 {
        b.put(0, 1);
    }

    let (result, _) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert_eq!(h.frame_type, FrameType::Key);
    assert!(h.show_frame && !h.show_existing_frame);
    assert!(h.disable_frame_end_update_cdf);
    assert_eq!(h.refresh_frame_flags, Some(0xFF));
    assert_eq!(h.order_hint, 0);
    assert_eq!(h.header_size_bits, intended_end);
}

/// `parse_frame_header_full` refuses what it cannot parse instead of guessing: each of these
/// would otherwise desync every later element.
#[test]
fn streams_it_cannot_parse_are_refused_with_a_reason() {
    let mut seq = base_seq();

    // frame_refs_short_signaling
    let mut b = HeaderBits::default();
    put_inter_header_through_ref_idx(
        &mut b,
        &seq,
        &InterHeader {
            error_resilient: false,
            frame_size_override: false,
            short_signaling: true,
        },
    );
    let message = parse_with(&seq, &b).0.unwrap_err().to_string();
    assert!(message.contains("frame_refs_short_signaling"), "{message}");

    // frame_size_with_refs: found_ref = 1
    let mut b = HeaderBits::default();
    put_inter_header_through_ref_idx(
        &mut b,
        &seq,
        &InterHeader {
            error_resilient: false,
            frame_size_override: true,
            short_signaling: false,
        },
    );
    let mut bits = b.0.clone();
    let found_ref_start = bits.len() - 7;
    bits[found_ref_start + 3] = 1; // the fourth reference says "same size as me"
    let mut ref_state = RefFrameState::new();
    let message = parse_frame_header_full(&pack(&bits), &seq, &mut ref_state)
        .unwrap_err()
        .to_string();
    assert!(message.contains("found_ref"), "{message}");

    // decoder_model_info_present
    seq.decoder_model_info = Some(crate::sequence::DecoderModelInfo {
        buffer_delay_length_minus_1: 0,
        num_units_in_decoding_tick: 1,
        buffer_removal_time_length_minus_1: 0,
        frame_presentation_time_length_minus_1: 0,
    });
    let mut ref_state = RefFrameState::new();
    let message = parse_frame_header_full(&minimal_key_frame_bytes(40), &seq, &mut ref_state)
        .unwrap_err()
        .to_string();
    assert!(message.contains("decoder_model_info"), "{message}");
}

/// A header cut short anywhere is an error, never a panic and never a "successful" parse of
/// fewer bits than the header needs.
#[test]
fn every_truncation_of_a_valid_header_is_an_error() {
    let seq = base_seq();
    let mut inter = HeaderBits::default();
    put_inter_header_through_ref_idx(
        &mut inter,
        &seq,
        &InterHeader {
            error_resilient: false,
            frame_size_override: false,
            short_signaling: false,
        },
    );
    inter.put(0, 1).put(0, 1).put(1, 1).put(0, 1).put(0, 1);
    put_tile_info_through_tx_mode(&mut inter);
    inter.put(0, 1).put(0, 1);
    for _ in 0..7 {
        inter.put(0, 1);
    }
    let inter = inter.bytes();
    let key = minimal_key_frame_bytes(40);

    for (name, bytes) in [("inter", inter), ("key", key)] {
        assert!(
            parse_frame_header_full(&bytes, &seq, &mut RefFrameState::new()).is_ok(),
            "{name}: the untruncated header parses"
        );
        // The last byte holds padding after the final element, so only cuts that remove whole
        // bytes are guaranteed to lose a real bit.
        for cut in 0..bytes.len() - 1 {
            let result = parse_frame_header_full(&bytes[..cut], &seq, &mut RefFrameState::new());
            assert!(result.is_err(), "{name} cut to {cut} bytes parsed");
        }
    }
}

/// `frame_size_with_refs()` (the `found_ref` flags) only exists when `frame_size_override_flag`
/// is set AND the frame is not error resilient; an error-resilient frame with the override flag
/// reads `frame_size()` directly.
#[test]
fn error_resilient_frame_with_size_override_reads_the_size_without_found_ref_flags() {
    let seq = base_seq();
    let mut b = HeaderBits::default();
    put_inter_header_through_ref_idx(
        &mut b,
        &seq,
        &InterHeader {
            error_resilient: true,
            frame_size_override: true,
            short_signaling: false,
        },
    );
    b.put(159, 9).put(119, 8); // frame_width_minus_1, frame_height_minus_1: 160x120
    b.put(0, 1); // render_and_frame_size_different
    b.put(0, 1); // allow_high_precision_mv
    b.put(1, 1); // is_filter_switchable
    b.put(0, 1); // is_motion_mode_switchable
    b.put(1, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reference_select
    b.put(0, 1); // reduced_tx_set
    for _ in 0..7 {
        b.put(0, 1);
    }

    let (result, _) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert_eq!((h.width, h.height), (160, 120));
    assert!(h.error_resilient_mode);
    assert_eq!(h.header_size_bits, b.bit_len());
}

/// A SWITCH_FRAME is an inter frame that is error resilient, overrides the frame size and
/// refreshes every slot, all without a bit: so it reads the `ref_order_hint[]` list, no
/// `primary_ref_frame`, no `refresh_frame_flags`, and the size directly.
#[test]
fn switch_frame_implies_error_resilience_size_override_and_refresh_of_every_slot() {
    let seq = base_seq();
    let mut b = HeaderBits::default();
    b.put(0, 1); // show_existing_frame
    b.put(3, 2); // frame_type = SWITCH_FRAME
    b.put(1, 1); // show_frame
                 // error_resilient_mode, frame_size_override_flag: implied true
    b.put(0, 1); // disable_cdf_update
    b.put(0, 1); // allow_screen_content_tools
    b.put(INTER_ORDER_HINT, 7);
    // primary_ref_frame: error resilient; refresh_frame_flags: implied 0xFF
    for hint in 1..=8 {
        b.put(hint, 7); // ref_order_hint[i]
    }
    b.put(0, 1); // frame_refs_short_signaling
    for idx in 1..=7 {
        b.put(idx, 3); // ref_frame_idx[i]
    }
    // frame_size_with_refs(): not for an error resilient frame, so frame_size() directly
    b.put(159, 9).put(119, 8);
    b.put(0, 1); // render_and_frame_size_different
    b.put(0, 1); // allow_high_precision_mv
    b.put(1, 1); // is_filter_switchable
    b.put(0, 1); // is_motion_mode_switchable
    b.put(1, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reference_select
    b.put(0, 1); // reduced_tx_set
    for _ in 0..7 {
        b.put(0, 1);
    }

    let (result, ref_state) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert_eq!(h.frame_type, FrameType::Switch);
    assert!(h.error_resilient_mode);
    assert_eq!(h.primary_ref_frame, PRIMARY_REF_NONE);
    assert_eq!(h.refresh_frame_flags, Some(0xFF));
    assert_eq!((h.width, h.height), (160, 120));
    assert_eq!(h.header_size_bits, b.bit_len());
    assert_eq!(ref_state.ref_order_hint, [INTER_ORDER_HINT; 8]);
}

/// An error-resilient INTRA_ONLY frame that does not refresh every slot reads the
/// `ref_order_hint[]` list even though it is intra, and a 1-bit order hint is one bit, not none.
#[test]
fn error_resilient_intra_only_frame_reads_ref_order_hints_when_it_does_not_refresh_every_slot() {
    let mut seq = base_seq();
    seq.order_hint_bits_minus_1 = Some(0);
    let mut b = HeaderBits::default();
    b.put(0, 1); // show_existing_frame
    b.put(2, 2); // frame_type = INTRA_ONLY_FRAME
    b.put(1, 1); // show_frame
    b.put(1, 1); // error_resilient_mode
    b.put(0, 1); // disable_cdf_update
    b.put(0, 1); // allow_screen_content_tools
    b.put(0, 1); // frame_size_override_flag
    b.put(1, 1); // order_hint (1 bit)
                 // primary_ref_frame: intra
    b.put(0x0F, 8); // refresh_frame_flags: not every slot
    for hint in 0..8 {
        b.put(hint & 1, 1); // ref_order_hint[i] (1 bit each)
    }
    b.put(0, 1); // render_and_frame_size_different
    b.put(1, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(0, 1); // reduced_tx_set
                 // global_motion_params(): intra frame, none

    let (result, ref_state) = parse_with(&seq, &b);
    let h = result.unwrap();
    assert_eq!(h.frame_type, FrameType::IntraOnly);
    assert!(h.error_resilient_mode);
    assert_eq!(h.order_hint, 1);
    assert_eq!(h.refresh_frame_flags, Some(0x0F));
    assert_eq!(h.header_size_bits, b.bit_len());
    assert_eq!(ref_state.ref_order_hint, [1, 1, 1, 1, 0, 0, 0, 0]);
}

/// `allow_warped_motion` is only present in inter frames: an intra frame of a sequence that
/// enables warped motion reads no extra bit. (A shown key frame is error resilient, which already
/// removes the bit; an INTRA_ONLY frame that is not error resilient is what needs the intra rule.)
#[test]
fn intra_only_frame_reads_no_allow_warped_motion_bit() {
    let mut seq = base_seq();
    seq.enable_warped_motion = true;
    let mut b = HeaderBits::default();
    b.put(0, 1); // show_existing_frame
    b.put(2, 2); // frame_type = INTRA_ONLY_FRAME
    b.put(1, 1); // show_frame
    b.put(0, 1); // error_resilient_mode
    b.put(0, 1); // disable_cdf_update
    b.put(0, 1); // allow_screen_content_tools
    b.put(0, 1); // frame_size_override_flag
    b.put(3, 7); // order_hint
    b.put(0x01, 8); // refresh_frame_flags (primary_ref_frame: intra)
    b.put(0, 1); // render_and_frame_size_different
    b.put(1, 1); // disable_frame_end_update_cdf
    put_tile_info_through_tx_mode(&mut b);
    b.put(1, 1); // reduced_tx_set (allow_warped_motion: intra, so no bit)

    let h = parse_with(&seq, &b).0.unwrap();
    assert_eq!(h.frame_type, FrameType::IntraOnly);
    assert!(!h.error_resilient_mode && !h.allow_warped_motion);
    assert!(h.reduced_tx_set);
    assert_eq!(h.header_size_bits, b.bit_len());
}

/// A TRANSLATION parameter is `decode_subexp(2 * 2^bits + 1)` with `bits` = 9 when
/// `allow_high_precision_mv` and 8 when not. Six "more" flags then a "stop" flag lands the two
/// symbol counts on different paths: 1025 symbols need one more flag and 8 bits (15 bits in
/// all), 513 symbols go straight to `ns(257)`, 8 bits (14 bits in all).
#[test]
fn translation_parameter_width_follows_allow_high_precision_mv() {
    for allow_high_precision_mv in [true, false] {
        let seq = base_seq();
        let mut b = HeaderBits::default();
        put_inter_header_through_ref_idx(
            &mut b,
            &seq,
            &InterHeader {
                error_resilient: false,
                frame_size_override: false,
                short_signaling: false,
            },
        );
        b.put(0, 1); // render_and_frame_size_different
        b.put(u32::from(allow_high_precision_mv), 1);
        b.put(1, 1); // is_filter_switchable
        b.put(0, 1); // is_motion_mode_switchable
        b.put(0, 1); // disable_frame_end_update_cdf
        put_tile_info_through_tx_mode(&mut b);
        b.put(0, 1); // reference_select
        b.put(0, 1); // reduced_tx_set
        b.put(1, 1).put(0, 1).put(1, 1); // LAST: is_global, !is_rot_zoom, is_translation
        for _ in 0..6 {
            b.put(1, 1); // subexp_more_bits
        }
        if allow_high_precision_mv {
            b.put(0, 1); // subexp_more_bits = 0
        }
        b.put(0, 8);
        // A second parameter of a different shape, so that a misread first parameter cannot be
        // cancelled out by a misread second one.
        b.put(0, 1).put(0, 3);
        for _ in 0..6 {
            b.put(0, 1); // the other six references: IDENTITY
        }

        let h = parse_with(&seq, &b).0.unwrap();
        assert_eq!(h.gm_type[1], GM_TYPE_TRANSLATION);
        assert_eq!(h.header_size_bits, b.bit_len(), "{allow_high_precision_mv}");
    }
}

/// A coded-lossless frame normally skips `lr_params()`, but with superres the frame is only
/// `all_lossless` when the upscaled width equals the frame width, so the loop-restoration types
/// are still read.
#[test]
fn lossless_frame_with_superres_still_reads_loop_restoration() {
    let mut seq = base_seq();
    seq.enable_superres = true;
    let mut b = HeaderBits::default();
    b.put(0, 1); // show_existing_frame
    b.put(0, 2); // frame_type = KEY_FRAME
    b.put(1, 1); // show_frame
    b.put(0, 1); // disable_cdf_update
    b.put(0, 1); // allow_screen_content_tools
    b.put(0, 1); // frame_size_override_flag
    b.put(0, 7); // order_hint
    b.put(1, 1).put(0, 3); // use_superres, coded_denom = 0 => denominator 9
    b.put(0, 1); // render_and_frame_size_different
    b.put(1, 1); // disable_frame_end_update_cdf
    b.put(1, 1).put(0, 1).put(0, 1); // tile_info: uniform, no extra column/row bits
    b.put(0, 8).put(0, 4); // base_q_idx = 0; no delta q, no qmatrix
    b.put(0, 1); // segmentation_enabled
                 // delta_q_present, loop filter, CDEF, tx mode: absent for a coded-lossless frame
    b.put(0, 2).put(0, 2).put(0, 2); // lr_type for the three planes: read, since not all lossless
    b.put(0, 1); // reduced_tx_set

    let h = parse_with(&seq, &b).0.unwrap();
    assert_eq!(h.upscaled_width, 320);
    assert_eq!(h.width, 284, "(320 * 8 + 4) / 9");
    assert_eq!(h.base_q_idx, Some(0));
    assert_eq!(h.header_size_bits, b.bit_len());
}

// ---- relative_dist / skip_mode_params -----------------------------------------------------------

/// Spec `get_relative_dist`, restated: the signed distance of `a - b` wrapped into
/// `[-2^(bits-1), 2^(bits-1))`.
fn reference_relative_dist(a: u32, b: u32, bits: u32) -> i64 {
    let n = 1i64 << bits;
    let d = (i64::from(a) - i64::from(b)).rem_euclid(n);
    if d >= n / 2 {
        d - n
    } else {
        d
    }
}

#[test]
fn relative_dist_wraps_like_the_spec_for_every_pair_of_small_order_hints() {
    for bits in 1..=7u32 {
        for a in 0..(1u32 << bits) {
            for b in 0..(1u32 << bits) {
                assert_eq!(
                    relative_dist(a, b, true, bits),
                    reference_relative_dist(a, b, bits),
                    "bits {bits}: {a} vs {b}"
                );
            }
        }
    }
    // A worked case: 1 is two ticks after 127 once seven bits wrap.
    assert_eq!(relative_dist(1, 127, true, 7), 2);
    assert_eq!(relative_dist(127, 1, true, 7), -2);
    assert_eq!(relative_dist(0, 64, true, 7), -64);
}

#[test]
fn relative_dist_is_zero_without_order_hints() {
    assert_eq!(relative_dist(5, 1, false, 7), 0);
    assert_eq!(relative_dist(5, 1, true, 0), 0);
    assert_eq!(relative_dist(5, 1, false, 0), 0);
    assert_eq!(relative_dist(u32::MAX, 0, false, 7), 0);
}

fn ref_state_with(hints: [u32; 8]) -> RefFrameState {
    let mut state = RefFrameState::new();
    state.ref_order_hint = hints;
    state
}

type SkipModeResult = std::result::Result<(bool, Option<[u8; 2]>), BitvueError>;

/// Runs `read_skip_mode_params` over `bits` (the one `skip_mode_present` bit, when read) for a
/// frame with `order_hint` whose seven references point at slots `0..7`.
fn skip_mode(
    seq: &SequenceHeader,
    intra: bool,
    reference_select: bool,
    hints: [u32; 8],
    order_hint: u32,
    present_bit: u32,
) -> (SkipModeResult, u64) {
    let bytes = pack(&[present_bit as u8]);
    let mut reader = BitReader::new(&bytes);
    let refs = [0u32, 1, 2, 3, 4, 5, 6];
    let result = read_skip_mode_params(
        &mut reader,
        intra,
        reference_select,
        seq,
        &ref_state_with(hints),
        &refs,
        order_hint,
    );
    (result, reader.position())
}

#[test]
fn skip_mode_needs_an_inter_frame_with_reference_select_and_order_hints() {
    let seq = base_seq();
    let hints = [8, 9, 12, 20, 3, 11, 15, 0];
    // The baseline is allowed and reads one bit.
    let (result, pos) = skip_mode(&seq, false, true, hints, 10, 1);
    assert_eq!((result.unwrap().0, pos), (true, 1));
    // Intra frame, no reference_select, or order hints disabled: nothing is read.
    for (intra, select) in [(true, true), (false, false), (true, false)] {
        let (result, pos) = skip_mode(&seq, intra, select, hints, 10, 1);
        assert_eq!(
            (result.unwrap(), pos),
            ((false, None), 0),
            "intra {intra} select {select}"
        );
    }
    let mut no_hints = seq.clone();
    no_hints.enable_order_hint = false;
    let (result, pos) = skip_mode(&no_hints, false, true, hints, 10, 1);
    assert_eq!((result.unwrap(), pos), ((false, None), 0));
}

#[test]
fn skip_mode_picks_the_closest_forward_and_backward_references() {
    let seq = base_seq();
    // Current frame 10. Forward (earlier): slots 0 (8), 1 (9), 4 (3) -> 9 at index 1 is the
    // closest. Backward (later): 2 (12), 3 (20), 5 (11), 6 (15) -> 11 at index 5.
    let hints = [8, 9, 12, 20, 3, 11, 15, 0];
    let (result, pos) = skip_mode(&seq, false, true, hints, 10, 1);
    assert_eq!(
        result.unwrap(),
        (true, Some([2, 6])),
        "LAST-relative numbering is index + 1"
    );
    assert_eq!(pos, 1);
    // The flag itself is read, not assumed.
    let (result, _) = skip_mode(&seq, false, true, hints, 10, 0);
    assert_eq!(result.unwrap(), (false, Some([2, 6])));
    // The order of the two references in the result is by index, whichever side is first.
    let hints = [11, 12, 9, 8, 20, 3, 15, 0];
    let (result, _) = skip_mode(&seq, false, true, hints, 10, 1);
    assert_eq!(result.unwrap().1, Some([1, 3]));
}

#[test]
fn skip_mode_with_only_forward_references_uses_the_two_closest_ones() {
    let seq = base_seq();
    // Everything is earlier than frame 10: the closest is 9 (index 1), then 8 (index 0).
    let hints = [8, 9, 7, 3, 5, 6, 2, 0];
    let (result, _) = skip_mode(&seq, false, true, hints, 10, 1);
    assert_eq!(result.unwrap().1, Some([1, 2]));
    // The second one is searched among strictly earlier hints than the first.
    let hints = [3, 5, 9, 8, 9, 2, 1, 0];
    let (result, _) = skip_mode(&seq, false, true, hints, 10, 1);
    // First forward: index 2 (9; the later 9 at index 4 does not replace it). Second: 8 at 3.
    assert_eq!(result.unwrap().1, Some([3, 4]));
}

#[test]
fn skip_mode_has_no_references_without_two_distinct_earlier_frames() {
    let seq = base_seq();
    // No earlier frame at all (equal and later hints do not count).
    let (result, pos) = skip_mode(&seq, false, true, [10, 11, 12, 10, 20, 30, 10, 0], 10, 1);
    assert_eq!((result.unwrap(), pos), ((false, None), 0));
    // One earlier frame, the rest equal or later: no second forward reference and no backward
    // one.
    let (result, pos) = skip_mode(&seq, false, true, [9, 10, 10, 10, 10, 10, 10, 0], 10, 1);
    assert_eq!((result.unwrap(), pos), ((false, None), 0));
    // All references share the same earlier hint: still no second distinct one.
    let (result, _) = skip_mode(&seq, false, true, [9; 8], 10, 1);
    assert_eq!(result.unwrap(), (false, None));
}

#[test]
fn skip_mode_compares_order_hints_with_wrap_around() {
    let seq = base_seq();
    // Seven-bit hints, current frame 2: 126 and 127 are earlier (-4 and -3), 5 and 6 later.
    let hints = [126, 127, 5, 6, 0, 0, 0, 0];
    let (result, _) = skip_mode(&seq, false, true, hints, 2, 1);
    // Earlier than frame 2 (distance -4, -3, -2): 126 (index 0), 127 (index 1) and the zeros at
    // indices 4..6; the closest is the first 0 (index 4). Later: 5 (index 2) is closer than 6.
    assert_eq!(result.unwrap().1, Some([3, 5]));
}

#[test]
fn skip_mode_flag_missing_from_the_data_is_an_error() {
    let seq = base_seq();
    let mut reader = BitReader::new(&[]);
    let refs = [0u32, 1, 2, 3, 4, 5, 6];
    let err = read_skip_mode_params(
        &mut reader,
        false,
        true,
        &seq,
        &ref_state_with([8, 9, 12, 20, 3, 11, 15, 0]),
        &refs,
        10,
    );
    assert!(err.is_err());
}

// ---- segmentation_params without update_map ---------------------------------------------------

/// `update_map = 0`: no `segmentation_temporal_update` bit, and `update_data` follows directly.
#[test]
fn segmentation_without_a_map_update_reads_no_temporal_update_bit() {
    let mut previous = SegmentationFeatures::default();
    previous.feature_enabled[1][0] = true;
    previous.feature_data[1][0] = 7;
    // enabled = 1, update_map = 0, update_data = 0, then an 0xFF tail that must not be consumed.
    let data = [0b1000_0111u8, 0xFF];
    let mut reader = BitReader::new(&data);
    let info = parse_segmentation_params(&mut reader, 2, &previous).unwrap();
    assert_eq!(reader.position(), 3);
    assert!(info.enabled && !info.update_map && !info.temporal_update);
    assert_eq!(info.feature_data[1][0], 7);
    assert_eq!(info.last_active_seg_id, 1);

    // With update_map = 1, the temporal_update bit is read before update_data.
    let data = [0b1110_0111u8, 0xFF];
    let mut reader = BitReader::new(&data);
    let info = parse_segmentation_params(&mut reader, 2, &previous).unwrap();
    assert_eq!(reader.position(), 4);
    assert!(info.update_map && info.temporal_update);

    // With no primary reference, nothing is read after `enabled` besides the data itself.
    let mut bits = vec![0b1000_0000u8];
    bits.extend([0u8; 9]);
    let mut reader = BitReader::new(&bits);
    let info = parse_segmentation_params(&mut reader, PRIMARY_REF_NONE, &previous).unwrap();
    assert!(info.update_map && !info.temporal_update);
    assert!(info.feature_enabled.iter().flatten().all(|&on| !on));
}

// ---- sequences without order hints ------------------------------------------------------------

/// An inter frame header from `show_existing_frame` through the `ref_frame_idx[]` loop for a
/// sequence with `enable_order_hint = 0`: no `order_hint`, no `ref_order_hint[]` loop even with
/// error resilience, and no `frame_refs_short_signaling` bit.
fn put_inter_header_without_order_hints(b: &mut HeaderBits, error_resilient: bool) {
    b.put(0, 1); // show_existing_frame
    b.put(1, 2); // INTER_FRAME
    b.put(1, 1); // show_frame
    b.put(u32::from(error_resilient), 1);
    b.put(0, 1); // disable_cdf_update
    b.put(0, 1); // allow_screen_content_tools (seq_force_screen_content_tools is SELECT)
    b.put(0, 1); // frame_size_override_flag
    if !error_resilient {
        b.put(INTER_PRIMARY_REF_FRAME, 3);
    }
    b.put(INTER_REFRESH_FLAGS, 8);
    for idx in 1..=7 {
        b.put(idx, 3); // ref_frame_idx[i]
    }
}

fn seq_without_order_hints() -> SequenceHeader {
    let mut seq = base_seq();
    seq.enable_order_hint = false;
    seq.order_hint_bits_minus_1 = None;
    seq
}

#[test]
fn without_order_hints_an_error_resilient_inter_frame_reads_no_order_hint_elements() {
    for error_resilient in [true, false] {
        let seq = seq_without_order_hints();
        let mut b = HeaderBits::default();
        put_inter_header_without_order_hints(&mut b, error_resilient);
        b.put(0, 1); // render_and_frame_size_different
        b.put(0, 1); // allow_high_precision_mv
        b.put(1, 1); // is_filter_switchable
        b.put(0, 1); // is_motion_mode_switchable
        b.put(1, 1); // disable_frame_end_update_cdf
        put_tile_info_through_tx_mode(&mut b);
        b.put(1, 1); // reference_select (no skip_mode_present bit follows: no order hints)
        b.put(0, 1); // reduced_tx_set
        for _ in 0..7 {
            b.put(0, 1); // global motion: IDENTITY
        }
        let (result, ref_state) = parse_with(&seq, &b);
        let h = result.unwrap_or_else(|e| panic!("error_resilient {error_resilient}: {e}"));
        assert_eq!(h.error_resilient_mode, error_resilient);
        assert_eq!(h.order_hint, 0, "no order_hint field");
        assert_eq!(h.ref_frame_idx, Some([1, 2, 3, 4, 5, 6, 7]));
        assert!(h.reference_select && !h.skip_mode_present);
        assert_eq!(h.skip_mode_refs, [0, 0]);
        assert_eq!(
            h.header_size_bits,
            b.bit_len(),
            "error_resilient {error_resilient}: stopped at the last bit written"
        );
        assert_eq!(
            ref_state.ref_order_hint, [0; 8],
            "order hints stay untouched"
        );
    }
}

// ---- tile_info with a minimum tile count -------------------------------------------------------

/// 8192x4608 with 64x64 superblocks is 128x72 superblocks, more than one tile's worth of area:
/// `minLog2Tiles = 2`.
fn big_frame_tile_info(bits: &HeaderBits) -> (crate::frame_header::TileLayout, u64) {
    let seq = base_seq();
    let bytes = bits.bytes();
    let mut reader = BitReader::new(&bytes);
    let layout = read_tile_info(&mut reader, &seq, 8192, 4608).unwrap();
    (layout, reader.position())
}

#[test]
fn uniform_tile_spacing_starts_from_the_minimum_log2_counts() {
    // Columns: min 1 (a tile is at most 64 superblocks wide), one more increment bit, which we
    // answer 0. Rows: minLog2Tiles (2) minus the column count (1) = 1, one more bit, 0.
    let mut b = HeaderBits::default();
    b.put(1, 1).put(0, 1).put(0, 1);
    b.put(2, 2).put(1, 2); // context_update_tile_id (2 bits), tile_size_bytes_minus_1
    let (layout, used) = big_frame_tile_info(&b);
    assert_eq!(used, b.bit_len());
    assert_eq!((layout.tile_cols_log2, layout.tile_rows_log2), (1, 1));
    assert_eq!(layout.col_starts_sb, vec![0, 64, 128]);
    assert_eq!(layout.row_starts_sb, vec![0, 36, 72]);
    assert_eq!(
        (layout.context_update_tile_id, layout.tile_size_bytes),
        (2, 2)
    );
    assert_eq!(layout.tile_count(), 4);
}

#[test]
fn explicit_tile_sizes_bound_the_row_height_by_the_minimum_tile_area() {
    // Non-uniform: two 64-wide columns (ns(64) = 6 bits of 63), then rows of at most
    // (128 * 72) >> (2 + 1) / 64 = 18 superblocks: ns(18) of 17 = 1111 + extra bit 1.
    let mut b = HeaderBits::default();
    b.put(0, 1);
    b.put(63, 6).put(63, 6);
    for _ in 0..4 {
        b.put(0b1111, 4).put(1, 1);
    }
    b.put(5, 3).put(2, 2); // context_update_tile_id (1 + 2 bits), tile_size_bytes_minus_1
    let (layout, used) = big_frame_tile_info(&b);
    assert_eq!(used, b.bit_len());
    assert_eq!(layout.col_starts_sb, vec![0, 64, 128]);
    assert_eq!(layout.row_starts_sb, vec![0, 18, 36, 54, 72]);
    assert_eq!((layout.tile_cols_log2, layout.tile_rows_log2), (1, 2));
    assert_eq!(
        (layout.context_update_tile_id, layout.tile_size_bytes),
        (5, 3)
    );
}

#[test]
fn explicit_tiles_of_a_small_frame_use_the_whole_frame_as_the_area_bound() {
    // 320x240: 5x4 superblocks, minLog2Tiles = 0. One column (ns(5) of 4), rows
    // bounded by the frame (4): ns(4) for 3 is 2 bits `11`.
    let seq = base_seq();
    let mut b = HeaderBits::default();
    b.put(0, 1);
    b.put(0b111, 3); // ns(5) of 4: `11` then the extra bit
    b.put(0b11, 2);
    let bytes = b.bytes();
    let mut reader = BitReader::new(&bytes);
    let layout = read_tile_info(&mut reader, &seq, 320, 240).unwrap();
    assert_eq!(reader.position(), b.bit_len());
    assert_eq!(layout.col_starts_sb, vec![0, 5]);
    assert_eq!(layout.row_starts_sb, vec![0, 4]);
    assert_eq!(layout.tile_count(), 1);
}

/// 4224x2240 is 66x35 superblocks: `minLog2Tiles = 1`, so explicit rows are bounded by
/// `(66 * 35) >> 2 = 577` area, i.e. 577 / 64 = 9 rows for the 64-wide first column.
#[test]
fn a_minimum_of_one_halves_the_area_and_the_widest_tile_is_the_first_not_the_last() {
    let seq = base_seq();
    let mut b = HeaderBits::default();
    b.put(0, 1);
    b.put(63, 6); // first column: 64 superblocks
    b.put(1, 1); // second column: 2 superblocks (ns(2): 1 bit, minus_1 = 1)
    for _ in 0..3 {
        b.put(0b111, 3).put(1, 1); // ns(9) of 8
    }
    b.put(0b111, 3); // last row: 8 left, ns(8) of 7
    b.put(1, 3).put(0, 2); // context_update_tile_id (1 + 2 bits), tile_size_bytes_minus_1
    let bytes = b.bytes();
    let mut reader = BitReader::new(&bytes);
    let layout = read_tile_info(&mut reader, &seq, 4224, 2240).unwrap();
    assert_eq!(reader.position(), b.bit_len());
    assert_eq!(layout.col_starts_sb, vec![0, 64, 66]);
    assert_eq!(layout.row_starts_sb, vec![0, 9, 18, 27, 35]);
    assert_eq!((layout.tile_cols_log2, layout.tile_rows_log2), (1, 2));
}

// ---- is_scaled(refFrame) (spec 5.11.27) ------------------------------------------------------

fn size_and_refs(
    width: u32,
    height: u32,
    ref_frame_idx: [u32; 7],
) -> super::size_and_refs::SizeAndRefs {
    super::size_and_refs::SizeAndRefs {
        size: super::frame_size::FrameSize {
            width,
            height,
            upscaled_width: width,
            upscaled_height: height,
            use_superres: false,
            superres_denom: 8,
        },
        allow_intrabc: false,
        ref_frame_idx,
        allow_high_precision_mv: false,
        subpel_filter_switchable: false,
        switchable_motion_mode: false,
        use_ref_frame_mvs: false,
    }
}

#[test]
fn a_reference_is_scaled_when_its_upscaled_size_differs_from_the_coded_size() {
    let mut state = RefFrameState::new();
    // Slot 0: an unscaled 352x288 picture. Slot 1: a 352-wide picture next to a 313-wide frame.
    // Slot 2: same width, other height. Slot 3 was never refreshed.
    state.size[0] = (352, 288);
    state.size[1] = (352, 288);
    state.size[2] = (352, 144);
    let unscaled = size_and_refs(352, 288, [0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(ref_scaled_flags(&state, &unscaled), [false; 8]);

    let superres = size_and_refs(313, 288, [0, 1, 2, 3, 0, 1, 2]);
    // Index 0 is INTRA_FRAME, never scaled; 1..=7 follow `ref_frame_idx`.
    assert_eq!(
        ref_scaled_flags(&state, &superres),
        [false, true, true, true, false, true, true, true],
        "an unknown slot (3) counts as unscaled"
    );

    let taller = size_and_refs(352, 288, [2, 0, 0, 0, 0, 0, 0]);
    assert_eq!(ref_scaled_flags(&state, &taller)[1..3], [true, false]);
}

#[test]
fn a_refreshed_slot_remembers_its_upscaled_size_and_show_existing_copies_it() {
    let mut state = RefFrameState::new();
    let gm = [GM_IDENTITY_PARAMS; 8];
    refresh_ref_slots(
        &mut state,
        0b0000_0110,
        5,
        &Default::default(),
        &gm,
        (352, 288),
        &Default::default(),
    );
    assert_eq!(state.size[..4], [(0, 0), (352, 288), (352, 288), (0, 0)]);
    state.copy_slot_to_all(2);
    assert_eq!(state.size, [(352, 288); 8]);
}

#[test]
fn a_refreshed_slot_remembers_its_film_grain_and_show_existing_loads_and_copies_it() {
    let grain = FilmGrainInfo {
        enabled: true,
        seed: 321,
        y_points: vec![(0, 50)],
        ..Default::default()
    };
    let mut state = RefFrameState::new();
    refresh_ref_slots(
        &mut state,
        0b0000_0100,
        5,
        &Default::default(),
        &[GM_IDENTITY_PARAMS; 8],
        (352, 288),
        &grain,
    );
    assert_eq!(state.film_grain[2], grain);
    assert_eq!(state.film_grain[1], FilmGrainInfo::default());

    // show_existing_frame = 1, frame_to_show_map_idx = 2
    let shown = parse_frame_header_full(&[0b1010_0000], &base_seq(), &mut state).unwrap();
    assert!(shown.show_existing_frame);
    assert_eq!(shown.film_grain, grain, "load_grain_params(2)");

    state.copy_slot_to_all(2);
    assert!(state.film_grain.iter().all(|g| *g == grain));
}
