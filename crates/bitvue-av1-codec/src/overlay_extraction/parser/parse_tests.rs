//! `ParsedFrame::parse_with_ref_state` pinned over whole fixture clips: every packet is parsed
//! with the reference state threaded from the packets before it, and the complete `Debug` of the
//! result (every header value, OBU range, tile range and the tile bytes) goes into one hash.

use super::ParsedFrame;
use crate::frame_header_full::RefFrameState;
use crate::ObuType;

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *hash ^= u64::from(*b);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

fn clip_hash(ivf: &[u8]) -> u64 {
    let (_header, packets) = crate::parse_ivf_frames(ivf).unwrap();
    let seq = packets
        .iter()
        .find_map(|p| {
            crate::parse_all_obus(&p.data)
                .ok()?
                .into_iter()
                .find_map(|o| {
                    (o.header.obu_type == ObuType::SequenceHeader).then(|| {
                        let mut out = vec![((ObuType::SequenceHeader as u8) << 3) | 0x02];
                        out.extend(crate::encode_uleb128(o.payload.len() as u64));
                        out.extend_from_slice(&o.payload);
                        out
                    })
                })
        })
        .expect("clip has a sequence header");
    let mut state = RefFrameState::new();
    let mut hash = 0xcbf29ce484222325u64;
    for packet in &packets {
        let data = [seq.as_slice(), packet.data.as_slice()].concat();
        let parsed = ParsedFrame::parse_with_ref_state(&data, &mut state).unwrap();
        fnv(&mut hash, format!("{parsed:?}").as_bytes());
        fnv(&mut hash, &parsed.tile_data);
    }
    hash
}

macro_rules! clip_case {
    ($name:ident, $file:literal, $hash:expr) => {
        #[test]
        fn $name() {
            let ivf = include_bytes!(concat!("../../../../../test_data/", $file));
            assert_eq!(clip_hash(ivf), $hash);
        }
    };
}

clip_case!(fixture_clip, "av1_test.ivf", 17488777893689411365);
clip_case!(aomenc_clip, "av1_aomenc_testsrc2.ivf", 14619252328506775547);
clip_case!(rav1e_clip, "av1_rav1e_testsrc2.ivf", 17683626133940776541);
clip_case!(svtav1_clip, "av1_svtav1_testsrc2.ivf", 8899569076061236929);
clip_case!(tiles5_clip, "av1_aomenc_tiles5.ivf", 16317030197718576557);
clip_case!(
    tile_groups_clip,
    "av1_aomenc_tilegroups.ivf",
    6928590870512926212
);
