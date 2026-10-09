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
            enable_dual_filter: true,
            enable_masked_compound: true,
            enable_jnt_comp: true,
            subpel_filter_switchable: true,
            force_integer_mv: false,
            allow_high_precision_mv: false,
            gm_type: [0; 8],
        },
        mi_rows: mi,
        mi_cols: mi,
        cdef_bits: 2,
        restoration: bitvue_av1_codec::tile::RestorationParams::none(),
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
    ("inter-plain", 0x9fd69602fbcb179d),
    ("inter-sb128", 0x9dbc6ea26fa234e6),
    ("key", 0x16da0488f42875e1),
    ("key-intrabc", 0x67ae0c31fb614ba0),
    ("key-palette-filter", 0x9f1ce80241022f41),
    ("skip-mode", 0xe6080236bd3c27d5),
    ("compound", 0xb7a8efef9df3efe7),
    ("delta-q-lf", 0x757f390dbc455ad1),
    ("delta-q-lf-sb128", 0x235fc9d52b987dd0),
    ("seg-no-update", 0x9fd69602fbcb179d),
    ("seg-pre-skip", 0xccc487e0519bb7cf),
    ("seg-post-skip", 0xfe2eff29be9f1611),
    ("seg-temporal", 0x194dfa100226fd37),
    ("seg-temporal-pre", 0x7d7e899d3bd5f751),
    ("seg-skip-feature", 0x813d5986d9f95af5),
    ("seg-globalmv", 0x1f9df5fad2923276),
    ("seg-everything", 0xbf3081b6cb4b2517),
    ("lossless", 0x7cb8264a96ecabfa),
    ("txfm-only4x4", 0xaeac1a834a310285),
    ("txfm-switchable", 0xbc21d40192f1d614),
    ("gm-translation-compound", 0x4e78c514653fa978),
    ("gm-rotzoom-compound", 0x7ca56684a1791903),
    ("gm-mixed-compound", 0xefbd7a679bf45c30),
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
    ("inter-plain", 0x9d1a24bf19d7b816),
    ("inter-sb128", 0xa446bafae0358031),
    ("key", 0x20871000593d2064),
    ("key-intrabc", 0x6e3c96b31fb818f9),
    ("key-palette-filter", 0x85273a535fa5600e),
    ("skip-mode", 0x1bc2a512ab249298),
    ("compound", 0x29667bb2b1428faa),
    ("delta-q-lf", 0x81ea79155d38ca68),
    ("delta-q-lf-sb128", 0x5bc6bd8ed6c4c9d1),
    ("seg-no-update", 0x9d1a24bf19d7b816),
    ("seg-pre-skip", 0x5e7bd768ef4589ed),
    ("seg-post-skip", 0x2d5395d1ff0e6bf8),
    ("seg-temporal", 0x6d9a5d6a4336beda),
    ("seg-temporal-pre", 0xbd716696f15483e3),
    ("seg-skip-feature", 0xe2de2448fa2d762a),
    ("seg-globalmv", 0x40a3763f0ab76467),
    ("seg-everything", 0xe7e3778e205f3a69),
    ("lossless", 0x569aa68c6957f751),
    ("txfm-only4x4", 0x3b9d070b9944e444),
    ("txfm-switchable", 0x5c93aedfe50411d0),
    ("gm-translation-compound", 0x777c6db968b8e7bb),
    ("gm-rotzoom-compound", 0xbcaa5439d7548ed6),
    ("gm-mixed-compound", 0x11622ea5251cccde),
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
