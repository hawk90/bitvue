//! Symbol-for-symbol comparisons against recorded dav1d traces.

use super::test_support::*;

/// Ground truth from a dav1d 1.5.1 build with `DEBUG_BLOCK_INFO` enabled (single-threaded,
/// decoding `test_data/av1_test.ivf`): the arithmetic decoder's `rng` right after each
/// `partition` symbol of the first 128x128 superblock of frame 0 (`poc=0`), and right after
/// the *first block's* `skip` symbol.
///
/// ```text
/// poc=0,y=0,x=0,bl=0,ctx=0,bp=3: r=63552
/// poc=0,y=0,x=0,bl=1,ctx=0,bp=3: r=50112
/// poc=0,y=0,x=0,bl=2,ctx=0,bp=3: r=54632
/// poc=0,y=0,x=0,bl=3,ctx=0,bp=7: r=51232
/// Post-skip[0]: r=49528
/// ```
///
/// The point is the order: AV1 decodes a block as soon as its partition is known, so the first
/// block must be visited right after the 4th partition symbol (`r=51232`) and its `skip` must
/// come next, before any further partition symbol. The parser used to read the superblock's
/// whole partition tree first and every block afterwards.
#[test]
fn frame_0_blocks_are_decoded_between_partition_symbols_like_dav1d() {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
    let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
    let obu_data: Vec<u8> = [seq_bytes.as_slice(), frames[0].data.as_slice()].concat();
    let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
    let params = parsed.coding_params();

    // dav1d reads no delta_q for this frame (it would show up as `Post-delta_q` right after
    // `Post-cdef_idx` on the first block, which sits at a superblock origin).
    assert!(!params.delta_q_enabled, "frame 0 has delta_q_present = 0");

    let base_qp = parsed.frame_type.base_qp.unwrap() as i16;
    let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
    let mut state = crate::tile::TileState {
        decoder: crate::SymbolDecoder::new_with_qcat(&parsed.tile_data, qcat).unwrap(),
        mv_ctx: crate::tile::MvPredictorContext::new(
            parsed.dimensions.sb_cols,
            parsed.dimensions.sb_rows,
        ),
        tile_ctx: crate::tile::TileContext::new(
            (parsed.dimensions.sb_cols * parsed.dimensions.sb_size).div_ceil(4),
            (parsed.dimensions.sb_rows * parsed.dimensions.sb_size).div_ceil(4),
        ),
    };
    state.tile_ctx.start_superblock_row();

    // Stop at the first leaf and report where the decoder stood when it was reached.
    let mut first_leaf = None;
    let result = crate::tile::partition::parse_partition_recursive(
        &mut state,
        0,
        0,
        crate::tile::BlockSize::Block128x128,
        params.mi_rows,
        params.mi_cols,
        0,
        &mut |state, leaf| {
            first_leaf = Some((leaf.x, leaf.y, leaf.size, state.decoder.decoder.range));
            Err(bitvue_engine::BitvueError::InvalidData(
                "stop at the first leaf".to_string(),
            ))
        },
    );
    assert!(result.is_err());
    let (x, y, size, range) = first_leaf.expect("a leaf was reached");
    assert_eq!((x, y), (0, 0));
    assert_eq!(
        (size.width(), size.height()),
        (8, 16),
        "VERT_B's first block is the full-height left one"
    );
    assert_eq!(
        range, 51232,
        "the first leaf follows the 4th partition symbol"
    );
}

/// Ground truth from the same dav1d 1.5.1 trace (see
/// `frame_0_blocks_are_decoded_between_partition_symbols_like_dav1d`): the decoder's `rng`
/// after each syntax element of frame 0's first block, up to its first coefficient token.
///
/// ```text
/// poc=0,y=0,x=0,bl=0..3 ...                   r=63552, 50112, 54632, 51232   (partition x4)
/// Post-skip[0]: r=49528          Post-cdef_idx[0]: r=49864
/// Post-ymode[0]: r=47640         Post-uvmode[0]: r=60524
/// Post-y_pal[1]: r=49216         Post-pal[pl=0,sz=2,...]: r=43528
/// Post-uv_pal[0]: r=57128        Post-y-pal-indices: r=33029   (128 symbols)
/// Post-tx[7]: r=49524            Post-non-zero[2][0][0]: r=48266
/// Post-txtp-intra[7->1][0][6->2]: r=59440
/// Post-eob_bin_128[0][0][4]: r=59456  Post-eob_hi_bit: r=56960  Post-eob[9]: r=56840
/// Post-lo_tok[2][0][1][9=3=1]: r=55611
/// ```
///
/// The block is an 8x16 palette block with `tx=7` (`RTX_8X16`): one rectangular transform
/// (not a 16x16 one, which would read a 256-coefficient `eob_bin` alphabet), a transform type
/// chosen from the 7-symbol intra set because the *smaller* side is 8, then `eob` and the
/// end-of-block token. Each value pins a decision that was once wrong here: `tx_size` for a
/// rectangular block and its per-axis context, and the `get_tx_set` side that picks the CDF.
#[test]
fn frame_0_first_block_matches_the_dav1d_trace_through_its_end_of_block_token() {
    const ORACLE: &[u32] = &[
        63552, 50112, 54632, 51232, // partition symbols
        49528, 49864, 47640, 60524, // skip, cdef_idx, ymode, uvmode
        49216, 43528, 57128, 33029, // y_pal, palette colours, uv_pal, palette indices
        49524, 48266, 59440, // tx_size, all_zero, transform type
        59456, 56960, 56840, 55611, // eob_bin, eob_hi_bit, eob, first token
    ];
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
    let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
    let obu_data: Vec<u8> = [seq_bytes.as_slice(), frames[0].data.as_slice()].concat();
    let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
    let params = parsed.coding_params();

    let base_qp = parsed.frame_type.base_qp.unwrap() as i16;
    let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
    let mut state = crate::tile::TileState {
        decoder: crate::SymbolDecoder::new_with_qcat(&parsed.tile_data, qcat).unwrap(),
        mv_ctx: crate::tile::MvPredictorContext::new(
            parsed.dimensions.sb_cols,
            parsed.dimensions.sb_rows,
        ),
        tile_ctx: crate::tile::TileContext::new(
            (parsed.dimensions.sb_cols * parsed.dimensions.sb_size).div_ceil(4),
            (parsed.dimensions.sb_rows * parsed.dimensions.sb_size).div_ceil(4),
        ),
    };
    state.tile_ctx.start_superblock_row();
    state.decoder.decoder.range_trace = Some(Vec::new());

    let mut sb_ctx = crate::tile::SuperblockCtx::new(0, 0, parsed.dimensions.sb_size);
    let _ = crate::tile::partition::parse_partition_recursive(
        &mut state,
        0,
        0,
        crate::tile::BlockSize::Block128x128,
        params.mi_rows,
        params.mi_cols,
        0,
        &mut |state, leaf| {
            let rect = crate::tile::BlockRect {
                x: leaf.x,
                y: leaf.y,
                width: leaf.size.width(),
                height: leaf.size.height(),
            };
            // Decode the first block only; whatever follows its end-of-block token is
            // covered by later oracle checkpoints.
            let _ = crate::tile::parse_coding_unit(state, &mut sb_ctx, rect, &params, base_qp);
            Err(bitvue_engine::BitvueError::InvalidData(
                "stop after the first block".to_string(),
            ))
        },
    );
    let trace: Vec<u32> = state
        .decoder
        .decoder
        .range_trace
        .take()
        .unwrap()
        .into_iter()
        .map(|(rng, _, _)| rng)
        .collect();

    // The four partition symbols come first, back to back.
    assert_eq!(&trace[..4], &ORACLE[..4]);
    // Everything after is an in-order subsequence; the palette index map is the longest gap
    // (127 context-coded symbols between `uv_pal` and `tx_size`).
    let mut at = 4;
    for (i, want) in ORACLE.iter().enumerate().skip(4) {
        let found = trace[at..]
            .iter()
            .take(140)
            .position(|r| r == want)
            .unwrap_or_else(|| {
                panic!("oracle checkpoint #{i} (r={want}) not reached after symbol {at}")
            });
        at += found + 1;
    }
}

/// The whole key frame (frame 0, 320x240, 82,966 coded symbols) decodes bit-exactly like
/// dav1d 1.5.1: after every symbol of a multi-symbol alphabet the arithmetic decoder's
/// `(rng, cnt, dif)` equals the reference decoder's.
///
/// The reference values come from a dav1d build with a one-line trace in `ctx_norm`
/// (`MS rng=%u cnt=%d dif=%llx` after each decode, `--threads 1 --limit 1`). They are folded into
/// an FNV-1a-64 of the lines `"{rng} {cnt} {dif}\n"` (`dif` in decimal) so the test needs no
/// 82,966-line fixture. Any wrong CDF, context, symbol order or missing/extra read changes the
/// digest; unlike the first-block test above, this covers every block of the frame --
/// palette maps, rectangular and 64-wide transforms, 1D transform classes, chroma, filter
/// intra, golomb tails.
///
/// Symbols of one-symbol alphabets (the placeholder `partition` read of a 4x4 child) consume no
/// bits and are not part of the trace.
#[test]
fn frame_0_decodes_bit_exactly_like_dav1d() {
    const ORACLE_SYMBOLS: usize = 82_966;
    const ORACLE_DIGEST: u64 = 0xe7c9_0862_8b33_a6a4;

    let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
    let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
    let obu_data: Vec<u8> = [seq_bytes.as_slice(), frames[0].data.as_slice()].concat();
    let parsed = super::super::parser::ParsedFrame::parse(&obu_data).unwrap();
    let params = parsed.coding_params();

    let base_qp = parsed.frame_type.base_qp.unwrap() as i16;
    let qcat = (base_qp > 20) as u8 + (base_qp > 60) as u8 + (base_qp > 120) as u8;
    let dims = &parsed.dimensions;
    let mut state = crate::tile::TileState {
        decoder: crate::SymbolDecoder::new_with_qcat(&parsed.tile_data, qcat).unwrap(),
        mv_ctx: crate::tile::MvPredictorContext::new(dims.sb_cols, dims.sb_rows),
        tile_ctx: crate::tile::TileContext::new(
            (dims.sb_cols * dims.sb_size).div_ceil(4),
            (dims.sb_rows * dims.sb_size).div_ceil(4),
        ),
    };
    state.decoder.decoder.range_trace = Some(Vec::new());

    let mut qp = base_qp;
    for sb_y in 0..dims.sb_rows {
        state.tile_ctx.start_superblock_row();
        for sb_x in 0..dims.sb_cols {
            let (_sb, new_qp) = crate::parse_superblock(
                &mut state,
                sb_x * dims.sb_size,
                sb_y * dims.sb_size,
                dims.sb_size,
                &params,
                qp,
            )
            .expect("frame 0 parses without error");
            qp = new_qp;
        }
    }

    let trace = state.decoder.decoder.range_trace.take().unwrap();
    let mut digest = 0xcbf2_9ce4_8422_2325u64;
    for (rng, cnt, dif) in &trace {
        for byte in format!("{rng} {cnt} {dif}\n").bytes() {
            digest ^= u64::from(byte);
            digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    assert_eq!(trace.len(), ORACLE_SYMBOLS, "number of coded symbols");
    assert_eq!(digest, ORACLE_DIGEST, "decoder state diverges from dav1d");
}

/// The fixture's chunk 1 holds a hidden frame (refreshes slot 6) and the shown frame
/// (refreshes slot 2), both with order hint 1: state threaded across it must contain both
/// refreshes, not only the first frame's.
#[test]
fn state_threading_applies_every_frame_of_a_temporal_unit() {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(AV1_IVF_FIXTURE).unwrap();
    let seq_bytes = find_seq_header_bytes(&frames).expect("fixture has a sequence header");
    let seq = crate::parse_sequence_header(
        &crate::obu::ObuIterator::new(&seq_bytes)
            .next_obu_with_offset()
            .unwrap()
            .unwrap()
            .obu
            .payload,
    )
    .unwrap();
    let state = crate::frame_header_full::thread_ref_state_before(&frames, &seq, 2).unwrap();
    let hints = state.ref_order_hint();
    assert_eq!(hints[6], 1, "hidden frame's refresh of slot 6");
    assert_eq!(hints[2], 1, "shown frame's refresh of slot 2");
    assert_eq!(hints[0], 0, "slot 0 still holds the key frame");
}

/// Decodes the first `count` frames of the fixture in decode order (one entry per Frame OBU,
/// so a hidden frame is its own entry), threading reference state and the motion field, and
/// returns each frame's `(rng, cnt, dif)` trace.
fn decode_fixture_traces(count: usize) -> Vec<Vec<(u32, i32, usize)>> {
    decode_traces(AV1_IVF_FIXTURE, count)
}

/// [`decode_fixture_traces`] for any IVF.
fn decode_traces(ivf: &[u8], count: usize) -> Vec<Vec<(u32, i32, usize)>> {
    let (_hdr, frames) = crate::ivf::parse_ivf_frames(ivf).unwrap();
    let seq_bytes = find_seq_header_bytes(&frames).expect("stream has a sequence header");
    let seq = crate::parse_sequence_header(
        &crate::obu::ObuIterator::new(&seq_bytes)
            .next_obu_with_offset()
            .unwrap()
            .unwrap()
            .obu
            .payload,
    )
    .unwrap();
    let mut state = crate::overlay_extraction::StreamDecodeState::new();
    let mut traces = Vec::new();
    for chunk in &frames {
        let units = crate::overlay_extraction::stream_state::frame_units(
            &chunk.data,
            seq.reduced_still_picture_header,
        );
        for unit in &units {
            if traces.len() == count {
                return traces;
            }
            let obu_data = [seq_bytes.as_slice(), unit.as_slice()].concat();
            state.decode_next(&obu_data, &seq).unwrap();
            traces.push(state.last_trace.clone());
        }
        // A unit without a frame (a `show_existing_frame` header) still updates the state.
        if units.is_empty() {
            let obu_data = [seq_bytes.as_slice(), chunk.data.as_slice()].concat();
            state.skip_unit(&obu_data).unwrap();
        }
    }
    traces
}

/// Every symbol the decoder reads for the first five frames of the fixture (key frame, hidden
/// ARF, shown inter frame, two more inter frames) is identical to dav1d's: the arithmetic
/// decoder's `(rng, cnt, dif)` after each symbol, from a dav1d 1.5.1 debug build with its
/// `msac` instrumented. Comparing the full state (not only the decoded value) catches reads
/// with a wrong probability even when they decode to the same bit. Each tuple is the symbol
/// count and the FNV-1a digest of the `"{rng} {cnt} {dif}\n"` lines of the whole frame.
///
/// Frames 1..=4 exercise: a temporal unit holding two frames (reference state threaded
/// through both), CDFs loaded from a reference (`primary_ref_frame`), MV candidate stacks and
/// contexts, temporal projection, the compound syntax, and chroma transform types inherited
/// from luma.
#[test]
fn first_five_frames_decode_symbol_for_symbol_like_dav1d() {
    const ORACLE: [(usize, u64); 5] = [
        (82_966, 0xe7c9_0862_8b33_a6a4),
        (41_785, 0xf637_9543_fc90_f2b5),
        (4_844, 0x7f2d_df0b_b44d_b942),
        (4_149, 0x083f_d2d3_9c64_6428),
        (2_194, 0x73ce_16d6_2ef2_1c03),
    ];
    let traces = decode_fixture_traces(ORACLE.len());
    assert_eq!(traces.len(), ORACLE.len());
    for (frame, (trace, &(symbols, digest))) in traces.iter().zip(&ORACLE).enumerate() {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for (rng, cnt, dif) in trace {
            for byte in format!("{rng} {cnt} {dif}\n").bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        assert_eq!(trace.len(), symbols, "frame {frame}: number of symbols");
        assert_eq!(
            hash, digest,
            "frame {frame}: decoder state diverges from dav1d"
        );
    }
}

/// Decodes every frame of `clip` and requires each frame's symbol count and FNV-1a digest of
/// the `"{rng} {cnt} {dif}\n"` lines to equal `oracle`, which comes from dav1d 1.5.1
/// (instrumented `msac`).
fn assert_clip_decodes_like_dav1d(clip: &[u8], oracle: &[(usize, u64)]) {
    let traces = decode_traces(clip, oracle.len());
    assert_eq!(traces.len(), oracle.len());
    let matches = |trace: &[(u32, i32, usize)], &(symbols, digest): &(usize, u64)| {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for (rng, cnt, dif) in trace {
            for byte in format!("{rng} {cnt} {dif}\n").bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        trace.len() == symbols && hash == digest
    };
    for (frame, (trace, expected)) in traces.iter().zip(oracle).enumerate() {
        assert!(
            matches(trace, expected),
            "frame {frame} diverges from dav1d ({} symbols, expected {})",
            trace.len(),
            expected.0
        );
    }
}

/// All 274 frames of the fixture (every Frame OBU in decode order, hidden frames included)
/// decode symbol for symbol like dav1d 1.5.1. This covers what the first-five-frames test
/// cannot: long reference chains, temporal MV projection when a reference slot holds an intra
/// frame (dav1d keeps no motion vectors for those), the tiny 3-byte tiles, and frames that
/// load CDFs from far back. Each entry is the frame's symbol count and the FNV-1a digest of
/// its `"{rng} {cnt} {dif}\n"` lines.
#[test]
fn whole_fixture_decodes_symbol_for_symbol_like_dav1d() {
    const ORACLE: [(usize, u64); 274] = [
        (82_966, 0xe7c9_0862_8b33_a6a4),
        (41_785, 0xf637_9543_fc90_f2b5),
        (4_844, 0x7f2d_df0b_b44d_b942),
        (4_149, 0x083f_d2d3_9c64_6428),
        (2_194, 0x73ce_16d6_2ef2_1c03),
        (2_466, 0x3e1d_d9cf_06bc_5b6b),
        (2_389, 0x5800_aa55_771b_28a0),
        (1_638, 0x6ef4_dbe4_6c4a_7e36),
        (2_378, 0x514c_3f3f_4119_abae),
        (1_470, 0x1290_f7e6_b86b_2904),
        (1_531, 0x3dc8_7a18_4db8_341a),
        (53, 0xd917_b23f_be83_ec37),
        (7_573, 0xb9b7_f172_4565_b9da),
        (2_856, 0x06cb_0a00_bf47_3238),
        (2_277, 0xbd56_bd67_cd29_db6d),
        (2_076, 0x3d3c_9d8b_f155_c613),
        (2_608, 0x6fe8_a078_4128_2e31),
        (3_228, 0x9911_9c8c_4337_5df3),
        (2_506, 0x243d_e85e_30ee_38e1),
        (2_328, 0xa0b4_cd7c_cd16_820f),
        (2_494, 0x8642_bf98_d2a8_739e),
        (2_097, 0x9a3a_3345_3edb_98ae),
        (54, 0x2bb4_1364_35d1_1e72),
        (13_885, 0x753a_7ad2_87ef_3969),
        (3_128, 0x508e_3425_b38d_6082),
        (3_038, 0xa176_0921_02d1_53b1),
        (4_133, 0xa826_7102_dd0a_8e03),
        (3_264, 0xd3b7_d493_0bf2_a1b8),
        (30_637, 0xbd30_3955_2fba_baec),
        (2_495, 0xacbf_101b_20d1_2218),
        (2_039, 0x3898_bf9b_7442_6bac),
        (2_302, 0xc911_ee02_a185_a1b0),
        (2_684, 0x7da3_4b5f_69b9_d35a),
        (54, 0x2bb4_1364_35d1_1e72),
        (11_846, 0x9641_53b8_0e18_67d4),
        (3_338, 0x7792_eb46_e352_52cd),
        (2_320, 0xd998_ca4c_b9dc_cff1),
        (3_758, 0x1454_de33_f2da_bdc4),
        (2_505, 0xfa83_2b5e_6113_a3d0),
        (3_017, 0xff78_58c8_13a1_3d08),
        (3_116, 0xa68e_ac26_4b87_48e6),
        (2_606, 0x1af6_aa4b_0c2c_1ca2),
        (2_318, 0x15bb_a4e3_8ae9_0bf3),
        (2_767, 0x3015_bde3_0370_039b),
        (54, 0x2bb4_1364_35d1_1e72),
        (38_504, 0xe074_e0a0_347f_0836),
        (2_781, 0x09a6_2650_9896_b7d8),
        (3_251, 0x5318_cd06_70e2_89c8),
        (3_114, 0x7e70_a3ad_c5ed_bbd5),
        (2_267, 0x379d_493c_4dfb_1d48),
        (4_031, 0xaaba_84b8_dd3c_47f0),
        (3_363, 0x527f_8010_3e1d_d041),
        (3_042, 0x3dec_2c00_38b4_f520),
        (2_651, 0xb291_4c29_a113_c92c),
        (2_926, 0xd894_1e20_143a_e743),
        (62, 0x0554_0322_5ed1_7605),
        (12_920, 0x8347_5b42_c770_67dc),
        (4_358, 0xccba_2c27_b88a_0422),
        (3_824, 0xad74_2adb_ed6a_f135),
        (5_316, 0x3643_a955_48a3_38fd),
        (3_776, 0x5d58_4bc9_c69c_1a76),
        (3_981, 0xa4aa_31f9_7f10_d098),
        (2_935, 0x2488_6101_7f8f_42cf),
        (2_865, 0x2956_bafd_6aab_4a29),
        (2_925, 0xc4fc_4a7a_1e71_bc64),
        (3_102, 0x41f4_00be_cb93_b274),
        (55, 0x3add_fc6c_ce8a_08dd),
        (33_754, 0xfb27_67b8_9a62_cf23),
        (4_151, 0x6394_02e8_bcf2_66ce),
        (3_750, 0x5437_c747_cd4b_2a54),
        (2_621, 0xe45c_2d47_ce11_7c9a),
        (6_311, 0x1d38_a2e4_fbd1_2110),
        (3_915, 0x0534_a164_19bb_2141),
        (3_284, 0x401a_f6c3_0a40_1090),
        (3_698, 0xe843_36cf_0945_e209),
        (3_196, 0xff31_fcb4_4d4d_a60a),
        (3_297, 0x701a_4212_5937_112d),
        (56, 0x653f_eac3_a2d4_de26),
        (11_376, 0xb71b_dc4d_b9a0_4cc6),
        (4_291, 0xe730_7a77_1ca5_07c3),
        (2_957, 0xf026_6cff_1c34_e26a),
        (4_570, 0xc8fe_2a4d_2ac3_c12b),
        (3_323, 0x0c2c_b134_247a_81b6),
        (38_935, 0x428a_0b8b_1fa8_9074),
        (3_390, 0xe099_931f_84a6_ea31),
        (2_426, 0x57b3_4b19_1e43_29bd),
        (2_539, 0x3eb0_3fd3_37d2_072d),
        (3_228, 0xb0ad_acac_b628_dc4f),
        (53, 0xd917_b23f_be83_ec37),
        (18_813, 0xff35_6513_b7fd_c65c),
        (3_321, 0xf2c6_e9df_a903_14ef),
        (2_687, 0xadff_a1c1_3388_efb5),
        (3_996, 0x9e04_8174_1642_ca1c),
        (3_223, 0xedec_db08_5e4b_d387),
        (3_320, 0x30a2_ee3c_b5ba_a9ad),
        (3_398, 0x26a9_1259_adb1_af0b),
        (2_915, 0x784a_b2d4_db79_6c3b),
        (2_337, 0xdd31_c7ce_0a9b_7395),
        (3_253, 0x0858_094c_8758_d861),
        (54, 0x2bb4_1364_35d1_1e72),
        (39_311, 0x9d03_42c2_738d_f522),
        (3_688, 0x66ef_4700_5dc2_370c),
        (2_992, 0x348e_6e3e_3098_8643),
        (4_064, 0xc2a8_9f49_8b83_578f),
        (2_924, 0x2cbe_1b67_0fa8_47bd),
        (4_336, 0xbd9d_0436_5ada_d2b0),
        (3_451, 0xe7e8_3fc5_cac5_4bc8),
        (2_946, 0x8ea3_11f5_ffea_f3e1),
        (2_804, 0x7b25_b0eb_c2a8_73d2),
        (3_785, 0x4fb1_209a_8c0a_a959),
        (62, 0x0554_0322_5ed1_7605),
        (14_088, 0x4c4c_d3f4_37ff_311e),
        (4_958, 0xba71_5f2d_125d_6269),
        (4_534, 0x9913_c4d4_d51b_c435),
        (6_903, 0xa97d_98ce_97e2_dbea),
        (4_138, 0xed69_be73_b694_7f5c),
        (4_910, 0xdb63_9d3f_8a89_b466),
        (3_507, 0x3529_dadf_c344_4c4c),
        (2_997, 0x20de_50f2_f710_1b1f),
        (3_305, 0x147e_cab8_6c79_b0d9),
        (3_856, 0x6a3f_27a4_74ce_eadc),
        (55, 0x3add_fc6c_ce8a_08dd),
        (8_248, 0x8770_781b_2990_5cd8),
        (4_645, 0x23f5_8406_8a08_8840),
        (2_934, 0x2e59_a7fb_10ba_32bb),
        (3_216, 0x6f9e_3caa_7ac1_7550),
        (2_994, 0x6cdb_f7ed_5e8a_2a68),
        (4_385, 0x9c2d_e857_3f76_2454),
        (3_511, 0x2267_02fb_9e17_c7ea),
        (4_120, 0x480d_bfb0_6a43_0b34),
        (3_754, 0x1dd9_3a64_4e9a_7a63),
        (4_300, 0x7a55_79b3_40cc_b818),
        (56, 0x0726_72dc_cd18_5d2e),
        (35_063, 0x24eb_990f_19dc_cb7a),
        (4_838, 0xcce7_d81e_ba93_3840),
        (4_243, 0x1560_6a7d_9888_f9ef),
        (7_623, 0xbbf9_c851_2077_e12d),
        (4_018, 0x70b1_9222_553e_13ed),
        (43_211, 0xa5c7_4bf5_04cc_12ac),
        (4_274, 0x5570_c1e4_aa26_8db9),
        (2_897, 0xbaf9_778b_0074_899e),
        (4_769, 0xba22_86fb_ad93_e911),
        (3_476, 0xc7d5_7e4f_1a05_70f6),
        (53, 0xd917_b23f_be83_ec37),
        (22_551, 0xbc9a_5faa_7246_a410),
        (2_612, 0x820b_3c62_e9f9_5757),
        (2_595, 0x4c8a_a839_a629_f99c),
        (3_424, 0x305a_5ee2_7f2f_d7f3),
        (3_211, 0x42e9_a3f8_1556_f74d),
        (3_926, 0xd800_55c7_b2b1_9681),
        (3_518, 0x14c0_a02c_2b6b_92fc),
        (3_205, 0x32a7_629f_4113_42c9),
        (2_751, 0x045c_7950_d33b_24b3),
        (3_655, 0x7705_96a8_4421_45f1),
        (55, 0x1ac1_8227_1339_2db6),
        (43_772, 0x4c85_dc60_fbd2_4811),
        (3_679, 0x8fcc_8e65_2470_f1a4),
        (3_453, 0x0b98_9347_4b11_448e),
        (4_257, 0x800e_46ee_76d3_e3d4),
        (2_893, 0x32e3_6df7_113b_5bae),
        (4_490, 0xb097_cac7_8fbf_9ab4),
        (4_115, 0x137a_3e26_f967_de83),
        (3_613, 0x5681_004b_48b3_1086),
        (3_239, 0x509b_e8dd_f86a_3566),
        (3_894, 0x8bc3_8343_bb01_c8c3),
        (62, 0x0554_0322_5ed1_7605),
        (13_818, 0xf21c_0be0_daab_fd99),
        (6_079, 0x0ac7_bdc4_f883_3b07),
        (3_897, 0x3e42_1102_7220_b2b5),
        (7_638, 0x0305_59a9_d453_31aa),
        (4_189, 0x2667_8304_0227_0d05),
        (5_493, 0xeac2_a811_d54d_7921),
        (3_115, 0x94ab_17e9_0993_9725),
        (3_147, 0xd894_ab5b_6746_93b3),
        (3_153, 0xf26e_a34e_6429_3e61),
        (3_723, 0x296d_21b1_c5cb_d34a),
        (55, 0x4c0a_0324_edfe_5611),
        (8_510, 0x4d72_b043_25ec_3e59),
        (4_682, 0xc3b8_d2be_2f80_7064),
        (3_605, 0x1600_22c7_5df4_4ebe),
        (3_363, 0x3ad1_2c58_ea7c_ca8f),
        (3_446, 0x73dd_6f47_d8d2_cc0a),
        (4_701, 0x71ff_3fdd_21b3_635e),
        (3_404, 0x2fe3_6b7c_f41d_0d15),
        (4_439, 0xed48_5196_65e9_cb26),
        (4_458, 0xe1ac_16e7_88ba_05ca),
        (4_283, 0x32f6_e205_f9f2_a8e8),
        (129, 0x0840_6362_f2b5_3f3f),
        (11_320, 0x201b_c1aa_0776_e508),
        (5_860, 0x60df_9fd4_3fcd_70e2),
        (5_082, 0x67a3_9d2f_c63b_95ce),
        (5_673, 0x7bbc_a327_edbc_95fe),
        (3_808, 0x2e4f_d66f_ccef_cdab),
        (45_673, 0x8fe8_489c_005f_7188),
        (3_465, 0xc39e_6546_21c0_d3ab),
        (2_898, 0x0c91_104e_7ea3_bfc3),
        (2_884, 0x0d40_86c9_9b5a_cb4b),
        (3_875, 0xa6c7_ad23_ee29_e725),
        (55, 0x3add_fc6c_ce8a_08dd),
        (27_756, 0x467a_e092_db48_c245),
        (3_473, 0xa030_a0c4_e9c0_735d),
        (3_120, 0x468e_256d_0a30_e30f),
        (4_809, 0x96e5_98f2_8fd6_7fc9),
        (3_432, 0xd6ad_57bb_9914_fcc3),
        (3_897, 0xc7e7_9eef_1448_08b4),
        (3_689, 0x70ac_b598_c40e_aa05),
        (3_636, 0xca36_dba0_a6d1_878a),
        (2_554, 0x8dae_7d76_63bf_abb4),
        (3_486, 0xcb42_0dcc_c780_fb7f),
        (218, 0x41b1_ba89_7365_ab6d),
        (63_113, 0xcf09_64ec_3e8c_c1fb),
        (3_390, 0xe7a2_c59f_86d3_fd92),
        (10_438, 0x79ca_eb2e_12c3_fef4),
        (4_290, 0xe025_0852_8717_50df),
        (3_272, 0x8d62_7330_c156_d1aa),
        (5_278, 0x9653_65a3_17a3_49b9),
        (4_025, 0x4431_94e7_7d16_01bc),
        (3_380, 0xca50_8770_1dcb_ad67),
        (3_261, 0x0941_204b_f6e6_2ae3),
        (4_633, 0xa5a0_de7b_1d2d_72a8),
        (82, 0x0eaa_9f80_83ff_affb),
        (13_750, 0x45f0_c270_b6cd_10c4),
        (8_172, 0xf0e9_5b4e_c6bd_8c3f),
        (4_319, 0x8ad8_cc84_ff02_8972),
        (8_307, 0x1e0c_ef1c_60c1_950c),
        (4_391, 0x4f98_9253_53d9_a70f),
        (4_788, 0x4e61_49ed_5700_edad),
        (3_054, 0x09f0_e0e3_c128_d7b8),
        (3_591, 0x25ea_d4af_88e0_44dd),
        (3_350, 0x4688_40e5_f28b_c3bf),
        (3_852, 0xb363_3058_c79b_175a),
        (101, 0x0e1e_c9fb_635e_54f8),
        (7_804, 0xdece_4bca_251b_3ac8),
        (3_789, 0x667b_c03d_7b05_aef2),
        (3_059, 0x208a_3167_ebb6_e43e),
        (3_755, 0xeb2d_c249_1ea5_7bf4),
        (3_484, 0x48c8_f981_8244_aaef),
        (4_732, 0x964f_f44e_5e65_e36a),
        (3_927, 0x428e_5c1c_22a2_239e),
        (4_615, 0xa1b4_290e_42fa_3b1d),
        (4_110, 0x45d0_57f7_4df1_2119),
        (5_140, 0x84a6_cbb0_d945_515b),
        (127, 0xba7f_34b4_a3b1_668a),
        (11_506, 0x8832_17a8_f426_8371),
        (6_274, 0x970d_b8ec_d917_f0c0),
        (4_599, 0x1419_5cef_453f_bc15),
        (5_834, 0x1231_f7c3_0844_a880),
        (4_293, 0xf87d_e8f1_57c6_d703),
        (45_839, 0x242d_091e_f91f_7577),
        (4_071, 0x6455_fe04_5c5f_ee9e),
        (2_846, 0x6d70_2fb4_5420_7336),
        (2_623, 0x7f9f_f1b3_38ef_59d5),
        (3_580, 0x0a41_9dca_6d8c_33aa),
        (54, 0x2bb4_1364_35d1_1e72),
        (33_996, 0xdc6e_cc66_f70e_5976),
        (4_158, 0xd781_47af_d129_9bc8),
        (2_888, 0xe56b_155b_07b7_cea4),
        (4_890, 0x0d74_a565_cd35_2492),
        (3_332, 0x7ab0_dcc7_0df6_df1c),
        (3_401, 0x9086_ef8f_4247_0f98),
        (3_398, 0x5d30_21b8_2de5_2016),
        (3_064, 0xc653_54f2_19c0_5b34),
        (2_682, 0xab32_e416_0596_47bb),
        (3_815, 0x9123_462f_a70d_da49),
        (54, 0x2bb4_1364_35d1_1e72),
        (3_740, 0x1822_f919_8fce_5973),
        (3_594, 0xf529_6976_3389_7125),
        (3_821, 0x8441_858a_bb44_ebc5),
        (2_899, 0x30e4_672d_084a_48c3),
        (5_714, 0x7429_dfbe_f1e8_5b37),
        (4_882, 0xf1fc_22f0_3ff6_4642),
        (3_695, 0x9618_bb3c_02ac_e5ee),
        (4_090, 0xa5b4_8d51_fa97_e63c),
        (3_836, 0xb92f_d650_f448_a2c3),
    ];
    assert_clip_decodes_like_dav1d(AV1_IVF_FIXTURE, &ORACLE);
}

/// 12 frames of a synthetic test pattern encoded with rav1e
/// (`test_data/av1_rav1e_testsrc2.ivf`, see `docs/PARITY_CHECKLIST.md`). Unlike the fixture it
/// is a typical encoder output: every frame updates its CDFs at the end of the frame
/// (`disable_frame_end_update_cdf = 0`), frames load their CDFs from a reference, segmentation
/// features carry over between frames, and its key frame uses loop restoration.
#[test]
fn rav1e_clip_decodes_symbol_for_symbol_like_dav1d() {
    const ORACLE: [(usize, u64); 12] = [
        (35_205, 0xf4d9_4bf6_a511_c1c9),
        (22_555, 0x42d5_5223_c818_2a18),
        (11_473, 0x2aba_ef6c_d37e_9aed),
        (8_381, 0x7973_73b8_e2e5_2884),
        (7_941, 0x1819_8407_bce6_6e1c),
        (20_374, 0xcdb0_c9eb_d52b_09d8),
        (10_645, 0x6431_871b_8538_b750),
        (8_469, 0x7b6e_fc92_c08b_1b1a),
        (7_945, 0x13ad_24e2_75f6_7348),
        (12_429, 0x65b6_a13a_35da_e0ef),
        (8_843, 0xab45_9ff6_76ca_d372),
        (7_863, 0x777e_d8ef_8959_bc8f),
    ];
    assert_clip_decodes_like_dav1d(
        include_bytes!("../../../../../test_data/av1_rav1e_testsrc2.ivf"),
        &ORACLE,
    );
}

/// 13 frames (one hidden) encoded with aomenc: IntraBC and delta-q on the key frame,
/// compound prediction with jnt_comp and masked compound, one interpolation filter shared by
/// both axes (`enable_dual_filter = 0`).
#[test]
fn aomenc_clip_decodes_symbol_for_symbol_like_dav1d() {
    const ORACLE: [(usize, u64); 13] = [
        (36_976, 0x8ad9_2b7c_9793_aec0),
        (19_122, 0xcafe_83d0_6a62_253f),
        (12_248, 0xe7cd_6f61_5a51_e547),
        (11_875, 0x5039_7c82_99d0_01eb),
        (8_005, 0x04b3_a8ea_64c4_e504),
        (7_801, 0x6534_6ff8_eeb5_2875),
        (7_760, 0xd58f_2223_14d3_c2af),
        (10_303, 0xe424_7dbb_a1de_5b36),
        (9_710, 0x47ef_35be_d6d3_eba9),
        (8_740, 0x5769_e594_84ed_da30),
        (7_697, 0x3780_efae_761a_0647),
        (7_396, 0xd3e4_edc0_1224_567d),
        (53, 0x0573_dc75_c66c_f904),
    ];
    assert_clip_decodes_like_dav1d(
        include_bytes!("../../../../../test_data/av1_aomenc_testsrc2.ivf"),
        &ORACLE,
    );
}

/// 12 frames encoded with SVT-AV1 4.2.0: IntraBC on the key frame, hierarchical prediction
/// with compound references, interintra, a single shared interpolation filter.
#[test]
fn svt_av1_clip_decodes_symbol_for_symbol_like_dav1d() {
    const ORACLE: [(usize, u64); 12] = [
        (38_011, 0xc2d7_a4e8_4882_c9c3),
        (16_872, 0x6fa0_6593_810f_b287),
        (14_111, 0x4138_60de_4b02_849b),
        (10_287, 0x40e2_4554_944b_70c5),
        (6_905, 0x75e2_1291_f50b_be32),
        (7_013, 0x278c_b5b9_05f5_2a31),
        (10_393, 0x2cea_3cf9_261d_ddf5),
        (9_541, 0x22c6_a392_f6e6_0bb2),
        (7_660, 0x18e9_3670_c2e3_3d8f),
        (12_720, 0x80b8_3fba_1760_50a4),
        (6_213, 0x2aac_3c40_a08b_177a),
        (9_211, 0x17fe_18f1_94a0_2a25),
    ];
    assert_clip_decodes_like_dav1d(
        include_bytes!("../../../../../test_data/av1_svtav1_testsrc2.ivf"),
        &ORACLE,
    );
}

/// 12 frames from aomenc 3.15.1 with `--tile-columns=3 --tile-rows=2`: a 5x2 superblock frame
/// splits into 5 tile columns (`TileColsLog2` is 3, more than the five tiles need) and 2 tile
/// rows, so every tile is one superblock wide. Covers the tile layout, the size field in front
/// of each tile, per-tile reset of the contexts, tiles of a single byte (an all-skip tile), and
/// the CDFs the frame saves coming from `context_update_tile_id`. dav1d decodes one superblock
/// row of every tile column in turn; the comparison trace is merged in that order.
#[test]
fn aomenc_five_tile_columns_clip_decodes_symbol_for_symbol_like_dav1d() {
    const ORACLE: [(usize, u64); 12] = [
        (42_172, 0x011b_98a6_771a_6170),
        (5_625, 0x83e6_efbd_f738_1b22),
        (8_953, 0x23f8_22ea_d51c_f7ce),
        (5_234, 0xeb69_99fe_67bc_f3c2),
        (6_307, 0x2f6c_3410_04a7_8f09),
        (7_516, 0x9e34_69e4_febf_46c4),
        (8_114, 0x0d4c_0ad9_e392_36b1),
        (5_621, 0x0208_146b_c150_27b8),
        (6_203, 0x1a15_a567_b87d_6f54),
        (4_660, 0xd9c3_dee5_d5e2_a269),
        (6_546, 0x723a_3c80_f8de_84f1),
        (4_549, 0x028b_01af_0dc6_c837),
    ];
    const CLIP: &[u8] = include_bytes!("../../../../../test_data/av1_aomenc_tiles5.ivf");
    assert_clip_decodes_like_dav1d(CLIP, &ORACLE);
}

/// 12 frames from aomenc 3.15.1 with `--tile-columns=1 --tile-rows=1 --num-tile-groups=2`:
/// every frame is a `FrameHeader` OBU and two `TileGroup` OBUs, the second starting at tile 2
/// (`tile_start_and_end_present_flag` set).
#[test]
fn aomenc_two_tile_groups_clip_decodes_symbol_for_symbol_like_dav1d() {
    const ORACLE: [(usize, u64); 12] = [
        (39_070, 0xad66_3c4d_7aa4_5276),
        (5_585, 0x1bf0_7dde_229b_26d3),
        (9_058, 0x9e89_17ce_412b_f4a9),
        (5_044, 0x7023_f4dc_08e7_ed95),
        (6_473, 0x4c39_a0c3_55db_eeca),
        (7_841, 0x13f1_12f8_54c7_200f),
        (8_487, 0x4624_7d14_146c_78e9),
        (5_216, 0xbb39_2607_6a0c_8db2),
        (5_532, 0x3856_f83f_52e9_cfe4),
        (4_540, 0xed17_8cc1_ab1b_cacc),
        (6_472, 0xfc89_d7bb_3292_222b),
        (4_361, 0xd787_4c30_4380_a6d4),
    ];
    const CLIP: &[u8] = include_bytes!("../../../../../test_data/av1_aomenc_tilegroups.ivf");
    assert_clip_decodes_like_dav1d(CLIP, &ORACLE);
}
