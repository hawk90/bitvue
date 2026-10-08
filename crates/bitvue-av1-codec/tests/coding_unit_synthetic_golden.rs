//! Characterization test for `parse_coding_unit` on syntax the real fixture never exercises
//! (issue #62: segmentation, skip_mode, intrabc, delta_lf, 128x128 superblocks).
//!
//! Any byte string is a decodable arithmetic-coded stream, so deterministic pseudo-random tile
//! data driven through a matrix of frame-level flags reaches every branch of the parser. The
//! `Debug` rendering of each result (CUs or the error) is hashed per configuration. Digests were
//! recorded from the unmodified code before `parse_coding_unit` was split into stages.
//!
//! To re-record after an intentional parsing change, run with `GOLDEN_PRINT=1`.

use bitvue_av1_codec::frame_header::TxfmMode;
use bitvue_av1_codec::frame_header_full::SegmentationInfo;
use bitvue_av1_codec::tile::{
    FrameCodingParams, InterModeFlags, MvPredictorContext, TileContext, TileState, TxTypeFrameFlags,
};
use bitvue_av1_codec::{parse_superblock, SymbolDecoder};

const SEEDS: u64 = 96;
/// Extra seeds (disjoint from `0..SEEDS`) for the deep pass; see `deep_streams_match_...`.
const DEEP_SEEDS: u64 = 400;
const FRAME_PX: u32 = 128;

fn fnv1a(bytes: &[u8], mut h: u64) -> u64 {
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// xorshift64* -- deterministic tile data per seed.
fn tile_bytes(seed: u64) -> Vec<u8> {
    let mut s = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    (0..4096)
        .map(|_| {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            (s.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 56) as u8
        })
        .collect()
}

fn base_params() -> FrameCodingParams {
    let mi = FRAME_PX / 4;
    FrameCodingParams {
        is_key_frame: false,
        delta_q_enabled: false,
        reference_select: false,
        allow_intrabc: false,
        allow_screen_content_tools: false,
        enable_filter_intra: false,
        delta_lf_present: false,
        delta_lf_multi: false,
        use_ref_frame_mvs: false,
        segmentation: SegmentationInfo::default(),
        tx_type_flags: TxTypeFrameFlags {
            coded_lossless: false,
            qidx_is_zero: false,
            reduced_tx_set: false,
            txfm_mode: Default::default(),
            mono_chrome: false,
            subsampling_x: true,
            subsampling_y: true,
        },
        inter_mode_flags: InterModeFlags {
            switchable_motion_mode: true,
            allow_warped_motion: true,
            enable_interintra_compound: true,
            enable_masked_compound: true,
            enable_jnt_comp: true,
            subpel_filter_switchable: true,
            force_integer_mv: false,
            gm_type: [0; 8],
        },
        mi_rows: mi,
        mi_cols: mi,
        cdef_bits: 2,
        skip_mode_present: false,
        skip_mode_refs: [0, 0],
    }
}

fn seg(
    update_map: bool,
    temporal: bool,
    pre_skip: bool,
    features: &[(usize, usize)],
) -> SegmentationInfo {
    let mut s = SegmentationInfo {
        enabled: true,
        update_map,
        temporal_update: temporal,
        seg_id_pre_skip: pre_skip,
        last_active_seg_id: 7,
        ..Default::default()
    };
    for &(segment, feature) in features {
        s.feature_enabled[segment][feature] = true;
        s.feature_data[segment][feature] = 2;
    }
    s
}

/// (name, params, superblock size) for every configuration.
fn configs() -> Vec<(&'static str, FrameCodingParams, u32)> {
    let b = base_params;
    let s = |segmentation| FrameCodingParams {
        segmentation,
        ..b()
    };
    vec![
        ("inter-plain", b(), 64),
        ("inter-sb128", b(), 128),
        (
            "key",
            FrameCodingParams {
                is_key_frame: true,
                ..b()
            },
            64,
        ),
        (
            "key-intrabc",
            FrameCodingParams {
                is_key_frame: true,
                allow_intrabc: true,
                allow_screen_content_tools: true,
                ..b()
            },
            64,
        ),
        (
            "key-palette-filter",
            FrameCodingParams {
                is_key_frame: true,
                allow_screen_content_tools: true,
                enable_filter_intra: true,
                ..b()
            },
            64,
        ),
        (
            "skip-mode",
            FrameCodingParams {
                skip_mode_present: true,
                skip_mode_refs: [1, 2],
                ..b()
            },
            64,
        ),
        (
            "compound",
            FrameCodingParams {
                reference_select: true,
                use_ref_frame_mvs: true,
                ..b()
            },
            64,
        ),
        (
            "delta-q-lf",
            FrameCodingParams {
                delta_q_enabled: true,
                delta_lf_present: true,
                delta_lf_multi: true,
                ..b()
            },
            64,
        ),
        (
            "delta-q-lf-sb128",
            FrameCodingParams {
                delta_q_enabled: true,
                delta_lf_present: true,
                ..b()
            },
            128,
        ),
        ("seg-no-update", s(seg(false, false, false, &[])), 64),
        ("seg-pre-skip", s(seg(true, false, true, &[(1, 5)])), 64),
        ("seg-post-skip", s(seg(true, false, false, &[(1, 2)])), 64),
        ("seg-temporal", s(seg(true, true, false, &[(2, 1)])), 64),
        ("seg-temporal-pre", s(seg(true, true, true, &[(2, 6)])), 64),
        (
            "seg-skip-feature",
            s(seg(true, false, true, &[(1, 6), (3, 6)])),
            64,
        ),
        (
            "seg-globalmv",
            s(seg(true, false, true, &[(2, 7), (4, 5)])),
            64,
        ),
        (
            "seg-everything",
            FrameCodingParams {
                segmentation: seg(true, true, true, &[(1, 5), (2, 6), (3, 7)]),
                skip_mode_present: true,
                skip_mode_refs: [1, 2],
                reference_select: true,
                delta_q_enabled: true,
                delta_lf_present: true,
                ..b()
            },
            128,
        ),
        (
            "lossless",
            FrameCodingParams {
                tx_type_flags: TxTypeFrameFlags {
                    coded_lossless: true,
                    ..b().tx_type_flags
                },
                ..b()
            },
            64,
        ),
        (
            "txfm-only4x4",
            FrameCodingParams {
                tx_type_flags: TxTypeFrameFlags {
                    txfm_mode: TxfmMode::Only4x4,
                    ..b().tx_type_flags
                },
                ..b()
            },
            64,
        ),
        (
            "txfm-switchable",
            FrameCodingParams {
                tx_type_flags: TxTypeFrameFlags {
                    txfm_mode: TxfmMode::Switchable,
                    ..b().tx_type_flags
                },
                ..b()
            },
            64,
        ),
        (
            "gm-translation-compound",
            FrameCodingParams {
                reference_select: true,
                inter_mode_flags: InterModeFlags {
                    gm_type: [1; 8],
                    ..b().inter_mode_flags
                },
                ..b()
            },
            64,
        ),
        (
            "gm-rotzoom-compound",
            FrameCodingParams {
                reference_select: true,
                inter_mode_flags: InterModeFlags {
                    gm_type: [2; 8],
                    ..b().inter_mode_flags
                },
                ..b()
            },
            64,
        ),
        (
            "gm-mixed-compound",
            FrameCodingParams {
                reference_select: true,
                inter_mode_flags: InterModeFlags {
                    gm_type: [0, 1, 0, 1, 0, 1, 0, 1],
                    ..b().inter_mode_flags
                },
                ..b()
            },
            64,
        ),
    ]
}

/// Parses every superblock of a 128x128 frame from seeded tile data; folds results into a digest.
fn digest(params: &FrameCodingParams, sb_size: u32, seeds: std::ops::Range<u64>) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for seed in seeds {
        let data = tile_bytes(seed);
        let mut state = TileState {
            decoder: SymbolDecoder::new(&data).unwrap(),
            mv_ctx: MvPredictorContext::new(2, 2),
            tile_ctx: TileContext::new(FRAME_PX / 4, FRAME_PX / 4),
        };
        let mut qp = 128i16;
        for sb_y in (0..FRAME_PX).step_by(sb_size as usize) {
            state.tile_ctx.start_superblock_row();
            for sb_x in (0..FRAME_PX).step_by(sb_size as usize) {
                let r = parse_superblock(&mut state, sb_x, sb_y, sb_size, params, qp);
                let text = match &r {
                    Ok((sb, new_qp)) => {
                        qp = *new_qp;
                        format!("{seed}:{sb_x},{sb_y}:{new_qp}:{:?}", sb.coding_units)
                    }
                    Err(e) => format!("{seed}:{sb_x},{sb_y}:err:{e:?}"),
                };
                h = fnv1a(text.as_bytes(), h);
                if r.is_err() {
                    break;
                }
            }
        }
    }
    h
}

const GOLDEN: &[(&str, u64)] = &[
    ("inter-plain", 0xdf5436c2c86f3914),
    ("inter-sb128", 0x9c69f29b6823136a),
    ("key", 0xc73d5aeec680148e),
    ("key-intrabc", 0x3c406e12d54940c1),
    ("key-palette-filter", 0x0ec1ec4cdaa4dd9d),
    ("skip-mode", 0xac6dbd21ff3d62aa),
    ("compound", 0x718746dbb06ed5e9),
    ("delta-q-lf", 0xa7e103f4f1198452),
    ("delta-q-lf-sb128", 0x956a6d78e3812c0d),
    ("seg-no-update", 0xdf5436c2c86f3914),
    ("seg-pre-skip", 0xbb904ceb7d5038cf),
    ("seg-post-skip", 0x893c67818ede7d94),
    ("seg-temporal", 0x7370ae24b9224286),
    ("seg-temporal-pre", 0xe50c67c7afa4b219),
    ("seg-skip-feature", 0x7923dbe60706309c),
    ("seg-globalmv", 0x540d71f49d664ecc),
    ("seg-everything", 0x3ae7a60f54f79935),
    ("lossless", 0x1771152d204f4bd9),
    ("txfm-only4x4", 0x49b414db7a9e774f),
    ("txfm-switchable", 0x3833ad2e8116834a),
    ("gm-translation-compound", 0x7e5e4596cd0d1410),
    ("gm-rotzoom-compound", 0x251a9055fb3c5cc5),
    ("gm-mixed-compound", 0x0ead1407a1563ee5),
];

#[test]
fn synthetic_streams_match_the_recorded_digests() {
    let got: Vec<(&str, u64)> = configs()
        .iter()
        .map(|(name, p, sb)| (*name, digest(p, *sb, 0..SEEDS)))
        .collect();
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        for (n, d) in &got {
            println!("GOLDEN (\"{n}\", 0x{d:016x}),");
        }
        return;
    }
    assert_eq!(got.len(), GOLDEN.len(), "configuration count changed");
    let bad: Vec<String> = got
        .iter()
        .zip(GOLDEN)
        .filter(|(g, w)| g != w)
        .map(|(g, w)| format!("{}: got 0x{:016x}, want 0x{:016x}", g.0, g.1, w.1))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The matrix is only worth having if it reaches the syntax it targets: with segmentation on,
/// at least one configuration must produce a non-zero `segment_id`, and skip_mode must fire.
#[test]
fn synthetic_matrix_reaches_the_untested_syntax() {
    let mut max_segment = 0u8;
    let mut skip_modes = 0usize;
    let mut parsed = 0usize;
    for (_, p, sb) in configs() {
        for seed in 0..SEEDS {
            let data = tile_bytes(seed);
            let mut state = TileState {
                decoder: SymbolDecoder::new(&data).unwrap(),
                mv_ctx: MvPredictorContext::new(2, 2),
                tile_ctx: TileContext::new(FRAME_PX / 4, FRAME_PX / 4),
            };
            let mut qp = 128i16;
            // The whole frame, as `digest` does -- a desynced stream stops parsing early, so the
            // first superblock alone may not reach everything.
            'frame: for sb_y in (0..FRAME_PX).step_by(sb as usize) {
                state.tile_ctx.start_superblock_row();
                for sb_x in (0..FRAME_PX).step_by(sb as usize) {
                    let Ok((s, new_qp)) = parse_superblock(&mut state, sb_x, sb_y, sb, &p, qp)
                    else {
                        break 'frame;
                    };
                    qp = new_qp;
                    parsed += s.coding_units.len();
                    for cu in &s.coding_units {
                        max_segment = max_segment.max(cu.segment_id);
                        skip_modes += usize::from(cu.skip_mode);
                    }
                }
            }
        }
    }
    assert!(parsed > 500, "only {parsed} CUs parsed");
    assert!(max_segment > 0, "no non-zero segment_id reached");
    assert!(skip_modes > 0, "skip_mode never fired");
}

/// Same matrix over many more seeds. The rare paths -- UV-only palette, a `skip_mode` block next
/// to a compound-type read, warped-motion eligibility without a matching neighbour, var-tx
/// recursion past depth 0, residual sums past the `cul_level` clamp -- only show up at this
/// volume, and `deep_streams_reach_the_rare_syntax` asserts that they do.
const DEEP_GOLDEN: &[(&str, u64)] = &[
    ("inter-plain", 0x65305a4a00d9c2cb),
    ("inter-sb128", 0x2c1939593786f133),
    ("key", 0x5c3abca619bd9473),
    ("key-intrabc", 0x2f2220dd8d231d28),
    ("key-palette-filter", 0xacd3260d410a704a),
    ("skip-mode", 0x2a7c571619b41859),
    ("compound", 0xbe03ce426370daa0),
    ("delta-q-lf", 0xeca519b47bd97c08),
    ("delta-q-lf-sb128", 0x44a5e47772003d5f),
    ("seg-no-update", 0x65305a4a00d9c2cb),
    ("seg-pre-skip", 0xca1e13616acc70c5),
    ("seg-post-skip", 0x8d3ed220b123ac9e),
    ("seg-temporal", 0xa73d3ca677933849),
    ("seg-temporal-pre", 0xb4ab0a26b69a87f2),
    ("seg-skip-feature", 0xf55e046bb8500b79),
    ("seg-globalmv", 0x19becdd77b57fbf4),
    ("seg-everything", 0x113314a45938d745),
    ("lossless", 0x67ee23a6d3c52ce0),
    ("txfm-only4x4", 0xbad8088aa66ce168),
    ("txfm-switchable", 0xff093daff4c80692),
    ("gm-translation-compound", 0x7f8cda4f981a338c),
    ("gm-rotzoom-compound", 0xeee577b5f33a13d7),
    ("gm-mixed-compound", 0x40cb076175fb314d),
];

#[test]
fn deep_streams_match_the_recorded_digests() {
    let got: Vec<(&str, u64)> = configs()
        .iter()
        .map(|(name, p, sb)| (*name, digest(p, *sb, SEEDS..SEEDS + DEEP_SEEDS)))
        .collect();
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        for (n, d) in &got {
            println!("DEEP (\"{n}\", 0x{d:016x}),");
        }
        return;
    }
    assert_eq!(got.len(), DEEP_GOLDEN.len(), "configuration count changed");
    let bad: Vec<String> = got
        .iter()
        .zip(DEEP_GOLDEN)
        .filter(|(g, w)| g != w)
        .map(|(g, w)| format!("{}: got 0x{:016x}, want 0x{:016x}", g.0, g.1, w.1))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The deep pass is only worth having if it reaches what it targets. Counts, over the same seeds
/// and configurations as `deep_streams_match_the_recorded_digests`, the rare syntax the shallow
/// pass misses.
#[test]
fn deep_streams_reach_the_rare_syntax() {
    let (mut uv_only_palette, mut skip_modes, mut big_residual, mut mixed_tx) = (0, 0, 0, 0);
    for (_, p, sb) in configs() {
        for seed in SEEDS..SEEDS + DEEP_SEEDS {
            let data = tile_bytes(seed);
            let mut state = TileState {
                decoder: SymbolDecoder::new(&data).unwrap(),
                mv_ctx: MvPredictorContext::new(2, 2),
                tile_ctx: TileContext::new(FRAME_PX / 4, FRAME_PX / 4),
            };
            if let Ok((s, _)) = parse_superblock(&mut state, 0, 0, sb, &p, 128) {
                for cu in &s.coding_units {
                    uv_only_palette +=
                        usize::from(cu.palette.y_size == 0 && cu.palette.uv_size > 0);
                    skip_modes += usize::from(cu.skip_mode);
                    big_residual +=
                        usize::from(cu.residual.as_ref().is_some_and(|r| r.sum_abs_level >= 63));
                    if let Some(blocks) = &cu.tx_blocks {
                        let first = blocks.first().map(|b| (b.width_px, b.height_px));
                        mixed_tx += usize::from(
                            blocks
                                .iter()
                                .any(|b| Some((b.width_px, b.height_px)) != first),
                        );
                    }
                }
            }
        }
    }
    assert!(uv_only_palette > 0, "no UV-only palette block reached");
    assert!(skip_modes > 0, "no skip_mode block reached");
    assert!(big_residual > 0, "no block with residual sum >= 63 reached");
    assert!(
        mixed_tx > 0,
        "no block split into mixed-size transform blocks"
    );
}
