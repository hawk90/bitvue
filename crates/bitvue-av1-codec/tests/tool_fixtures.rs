//! Real streams that switch on coding tools the older clips never use (#62): segmentation,
//! skip mode, palette and the `allow_intrabc` frame flag. Generated for this repo from synthetic
//! sources (commands in `test_data/README.md`).
//!
//! The strongest check here is `Provenance::Verified`: every superblock of the tile decoded and the
//! tile ends exactly where the decoded symbols put it, which a desynchronised symbol decoder
//! essentially never does. The listed frames are verified, so the tool's syntax elements are read in
//! step with the encoder's. Frames not listed are not checked (inter frames in general are not
//! verified yet); the per-clip digests only pin that nothing changes silently.
//!
//! To re-record the digests after an intended parsing change, run with `GOLDEN_PRINT=1`.

use bitvue_av1_codec::frame_header_full::RefFrameState;
use bitvue_av1_codec::obu::{ObuIterator, ObuType};
use bitvue_av1_codec::overlay_extraction::{
    frame_provenance, parse_all_coding_units, ParsedFrame, Provenance,
};
use bitvue_av1_codec::parse_ivf_frames;
use bitvue_av1_codec::tile::CodingUnit;

const SEGMENTATION_PALETTE: &[u8] =
    include_bytes!("../../../test_data/av1_aomenc_segmentation_palette.ivf");
const SKIPMODE_PALETTE: &[u8] =
    include_bytes!("../../../test_data/av1_aomenc_skipmode_palette.ivf");
const INTRABC_ALLOWED: &[u8] = include_bytes!("../../../test_data/av1_svtav1_intrabc_allowed.ivf");
const INTRABC_GLYPHS: &[u8] = include_bytes!("../../../test_data/av1_aomenc_intrabc_glyphs.ivf");
const LOSSLESS_TILED: &[u8] = include_bytes!("../../../test_data/av1_aomenc_lossless_tiled.ivf");
const LOSSLESS_TESTSRC2: &[u8] =
    include_bytes!("../../../test_data/av1_aomenc_lossless_testsrc2.ivf");
const DECODER_MODEL: &[u8] = include_bytes!("../../../test_data/av1_aomenc_decoder_model.ivf");

struct Frame {
    parsed: ParsedFrame,
    units: Vec<CodingUnit>,
    provenance: Provenance,
}

/// Every frame of an IVF clip, parsed with the reference state threaded from the frames before it.
fn frames(ivf: &[u8]) -> Vec<Frame> {
    let (_header, packets) = parse_ivf_frames(ivf).unwrap();
    let seq = packets
        .iter()
        .find_map(|p| {
            let mut obus = ObuIterator::new(&p.data);
            while let Some(Ok(found)) = obus.next_obu_with_offset() {
                if found.obu.header.obu_type == ObuType::SequenceHeader {
                    return Some(p.data[found.offset..found.offset + found.consumed].to_vec());
                }
            }
            None
        })
        .expect("the clip has a sequence header");
    let mut state = RefFrameState::new();
    packets
        .iter()
        .map(|p| {
            let data = [seq.as_slice(), p.data.as_slice()].concat();
            let parsed = ParsedFrame::parse_with_ref_state(&data, &mut state).unwrap();
            let units = parse_all_coding_units(&parsed)
                .map(|u| u.to_vec())
                .unwrap_or_default();
            let provenance = frame_provenance(&parsed);
            Frame {
                parsed,
                units,
                provenance,
            }
        })
        .collect()
}

fn digest(frames: &[Frame]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for (i, f) in frames.iter().enumerate() {
        for b in format!(
            "{i}:{}",
            bitvue_av1_codec::debug_without_cost_fields(&f.units)
        )
        .bytes()
        {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn check_digest(name: &str, frames: &[Frame], golden: u64) {
    let got = digest(frames);
    if std::env::var_os("GOLDEN_PRINT").is_some() {
        println!("GOLDEN {name} {got}");
        return;
    }
    assert_eq!(got, golden, "{name}: coding units changed");
}

fn verified(frames: &[Frame]) -> Vec<usize> {
    frames
        .iter()
        .enumerate()
        .filter(|(_, f)| f.provenance == Provenance::Verified)
        .map(|(i, _)| i)
        .collect()
}

/// aomenc `--aq-mode=1`: segmentation on every frame; the key frame has blocks in segments other
/// than 0 (read through `segment_id` with its predicted-id context) and 63 palette blocks.
#[test]
fn segmentation_and_palette_are_decoded_in_step_on_the_verified_frames() {
    let f = frames(SEGMENTATION_PALETTE);
    assert_eq!(f.len(), 8);
    assert!(f.iter().all(|f| f.parsed.segmentation.enabled));
    assert_eq!(
        verified(&f),
        vec![0, 1],
        "key frame and the first inter frame"
    );

    let key_units = &f[0].units;
    let in_segments = key_units.iter().filter(|u| u.segment_id != 0).count();
    assert_eq!(in_segments, 78, "key frame blocks outside segment 0");
    assert!(key_units.iter().all(|u| u.segment_id < 8));
    assert_eq!(
        key_units.iter().filter(|u| u.palette.y_size > 0).count(),
        63
    );
    assert_eq!(
        f[1].units.iter().filter(|u| u.palette.y_size > 0).count(),
        4
    );
    check_digest("segmentation_palette", &f, 5348272398597928467);
}

/// aomenc `--tune-content=screen` on a tiled pattern: palette blocks on the key frame, and a
/// verified inter frame (frame 4) with `skip_mode_present` whose four blocks use skip mode.
#[test]
fn skip_mode_is_decoded_in_step_on_a_verified_inter_frame() {
    let f = frames(SKIPMODE_PALETTE);
    assert_eq!(f.len(), 8);
    assert_eq!(verified(&f), vec![0, 1, 4]);
    assert_eq!(
        f[0].units.iter().filter(|u| u.palette.y_size > 0).count(),
        8
    );

    let skip_frame = &f[4];
    assert!(skip_frame.parsed.skip_mode_present);
    let skipped: Vec<&CodingUnit> = skip_frame.units.iter().filter(|u| u.skip_mode).collect();
    assert_eq!(skipped.len(), 4);
    assert!(skipped.iter().all(|u| u.mode.is_inter()));
    // no other frame of the clip signals skip mode
    assert_eq!(
        f.iter()
            .enumerate()
            .filter(|(_, f)| f.parsed.skip_mode_present)
            .map(|(i, _)| i)
            .collect::<Vec<_>>(),
        vec![4]
    );
    check_digest("skipmode_palette", &f, 13143573635549582445);
}

/// SVT-AV1 `--scm 1 --enable-intrabc 1`: the key frame sets `allow_intrabc` (so `use_intrabc` is
/// read for every block and the loop filters, cdef and loop restoration are off) and is verified.
/// No block of this clip uses intra block copy; `INTRABC_GLYPHS` below has the ones that do.
#[test]
fn a_frame_that_allows_intrabc_is_decoded_in_step() {
    let f = frames(INTRABC_ALLOWED);
    assert_eq!(f.len(), 3);
    assert!(f[0].parsed.allow_intrabc);
    assert!(f[1..].iter().all(|f| !f.parsed.allow_intrabc));
    assert_eq!(verified(&f), vec![0]);
    assert!(f[0].units.iter().all(|u| !u.use_intrabc));
    assert_eq!(
        f[0].units.iter().filter(|u| u.palette.y_size > 0).count(),
        5
    );
    check_digest("intrabc_allowed", &f, 15559747495860676135);
}

/// aomenc `--tune-content=screen` on a synthetic text page (a handful of 8x8 glyphs repeated on a flat
/// background): both key frames pick intra block copy for most blocks (231 of 243). Each block's
/// displacement vector is read through the intrabc reference stack and clipped to the decoded area,
/// and the frames are verified, so those reads are in step with the encoder's.
#[test]
fn blocks_that_copy_from_the_decoded_area_are_read_in_step() {
    let f = frames(INTRABC_GLYPHS);
    assert_eq!(f.len(), 2);
    assert_eq!(verified(&f), vec![0, 1]);
    let (width, height) = (320, 192);
    for (i, frame) in f.iter().enumerate() {
        assert!(frame.parsed.allow_intrabc, "frame {i}");
        assert_eq!(frame.units.len(), 243, "frame {i}");
        let copies: Vec<_> = frame.units.iter().filter(|u| u.use_intrabc).collect();
        assert_eq!(copies.len(), 231, "frame {i}");
        for u in copies {
            let v = u.mv[0];
            // Whole-sample vectors (1/8 units) whose source lies inside the frame and is already
            // decoded: entirely above the block or entirely to its left.
            assert!(v.x % 8 == 0 && v.y % 8 == 0, "frame {i}: {u:?}");
            let (sx, sy) = (u.x as i32 + v.x / 8, u.y as i32 + v.y / 8);
            assert!(sx >= 0 && sy >= 0, "frame {i}: {u:?}");
            assert!(
                sx + u.width as i32 <= width && sy + u.height as i32 <= height,
                "frame {i}: {u:?}"
            );
            let above = sy + u.height as i32 <= u.y as i32;
            let left = sx + u.width as i32 <= u.x as i32;
            assert!(above || left, "frame {i}: {u:?}");
        }
    }
    check_digest("intrabc_glyphs", &f, 1098898197756232468);
}

/// aomenc `--lossless=1` (CodedLossless: 4x4 Walsh-Hadamard transforms in every plane). Chroma used
/// to be read as one large transform block per 64x64 luma chunk instead of 4x4 blocks, which
/// desynchronised the tile decode of every such frame (#388: key frames stopped after 0 to 3 of 4
/// superblocks). Every frame of both clips, inter frames included, decodes completely and ends
/// where its symbols say it does.
#[test]
fn lossless_frames_are_decoded_in_step() {
    for (name, ivf, golden) in [
        ("lossless_tiled", LOSSLESS_TILED, 17328800481020161262u64),
        (
            "lossless_testsrc2",
            LOSSLESS_TESTSRC2,
            9621018677174943790u64,
        ),
    ] {
        let f = frames(ivf);
        assert_eq!(f.len(), 3, "{name}");
        for (i, frame) in f.iter().enumerate() {
            assert!(frame.parsed.coded_lossless, "{name} frame {i}");
            assert_eq!(frame.provenance, Provenance::Verified, "{name} frame {i}");
            assert!(!frame.units.is_empty(), "{name} frame {i}");
        }
        assert_eq!(verified(&f), vec![0, 1, 2], "{name}");
        check_digest(name, &f, golden);
    }
}

/// aomenc `--timing-info=model` (CBR, no lag): the sequence header signals a decoder model and every
/// frame header carries `temporal_point_info()` and `buffer_removal_time` (spec 5.9.2). Frame headers
/// of such a stream used to be rejected outright, so no tile was found and nothing was analysed
/// (#538). With them read, every frame decodes to the end.
#[test]
fn a_stream_with_a_decoder_model_is_decoded_in_step() {
    let (_header, packets) = parse_ivf_frames(DECODER_MODEL).unwrap();
    let model = packets
        .iter()
        .find_map(|p| {
            let mut obus = ObuIterator::new(&p.data);
            while let Some(Ok(found)) = obus.next_obu_with_offset() {
                if found.obu.header.obu_type == ObuType::SequenceHeader {
                    return bitvue_av1_codec::sequence::parse_sequence_header(&found.obu.payload)
                        .ok();
                }
            }
            None
        })
        .expect("the clip has a sequence header");
    assert!(
        model.decoder_model_info.is_some(),
        "the clip signals a decoder model"
    );
    assert!(model.operating_points[0].decoder_model_present);

    let f = frames(DECODER_MODEL);
    assert_eq!(f.len(), 3);
    for (i, frame) in f.iter().enumerate() {
        assert_eq!(frame.provenance, Provenance::Verified, "frame {i}");
        assert!(!frame.units.is_empty(), "frame {i}");
    }
    check_digest("decoder_model", &f, 11944185530889035051);
}

bitvue_test_alloc::install!();

const DELTA_LF: &[u8] = include_bytes!("../../../test_data/av1_aomenc_delta_lf.ivf");

/// aomenc `--deltaq-mode=1 --delta-lf-mode=1` (#591): the key frame signals `delta_lf_present` with
/// `delta_lf_res = 1` and is `Verified`, so the `delta_lf` symbols are read in step. Blocks carry
/// the accumulated `DeltaLF` (each step is shifted left by `res = 1`, so every value is even),
/// `delta_lf_multi` is off (aomenc never sets it) so only component 0 is used, and frames that do
/// not signal `delta_lf_present` keep zeros.
#[test]
fn delta_lf_values_are_kept_per_block() {
    let f = frames(DELTA_LF);
    assert_eq!(f.len(), 6);
    assert_eq!(verified(&f).first(), Some(&0));

    let key = &f[0];
    assert!(key.parsed.delta_lf_present);
    assert!(!key.parsed.delta_lf_multi);
    assert_eq!(key.parsed.delta_lf_res, 1);
    let nonzero: Vec<[i16; 4]> = key
        .units
        .iter()
        .map(|u| u.delta_lf)
        .filter(|d| *d != [0; 4])
        .collect();
    assert!(!nonzero.is_empty(), "the encoder signalled non-zero deltas");
    assert!(nonzero
        .iter()
        .all(|d| d[0] % 2 == 0 && d[1..] == [0, 0, 0] && d[0].abs() <= 63));

    for fr in f.iter().filter(|fr| !fr.parsed.delta_lf_present) {
        assert!(fr.units.iter().all(|u| u.delta_lf == [0; 4]));
    }
    assert_eq!(
        nonzero.len(),
        67,
        "key frame blocks with a non-zero DeltaLF"
    );
}

/// Same clip (#592): the key frame also signals `delta_q_present`. Each decoded `delta_q` step is
/// shifted left by `delta_q_res` (spec 5.11.39), so a block's `qp` differs from `base_q_idx` by a
/// multiple of `1 << delta_q_res`, and `CurrentQIndex` never drops below 1.
#[test]
fn delta_q_steps_are_shifted_by_delta_q_res() {
    let f = frames(DELTA_LF);
    let key = &f[0];
    assert!(key.parsed.delta_q_enabled);
    let res = key.parsed.delta_q_res;
    assert_eq!(res, 2, "the clip's key frame delta_q_res");
    let base = i16::from(key.parsed.frame_type.base_qp.expect("base_q_idx parsed"));
    let qps: Vec<i16> = key.units.iter().filter_map(|u| u.qp).collect();
    assert!(!qps.is_empty());
    assert!(qps.iter().all(|q| (1..=255).contains(q)));
    assert!(qps.iter().any(|q| *q != base), "the encoder moved the QP");
    assert!(
        qps.iter().all(|q| (q - base) % (1 << res) == 0),
        "qp - base_q_idx must be a multiple of 1 << delta_q_res"
    );
}
