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
        for b in format!("{i}:{:?}", f.units).bytes() {
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
/// No block of the clip actually uses intra block copy: no available encoder produced one for
/// the synthetic sources tried, so the copy itself is still untested on a real stream.
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

bitvue_test_alloc::install!();
