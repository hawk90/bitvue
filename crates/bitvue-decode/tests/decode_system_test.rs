//! System-level checks of the decode crate through its public API: real clips from `test_data/`
//! and the real size / format helpers. (This file used to hold twelve tests that defined their own
//! structs and asserted on literals they had just written, so they could not fail whatever the
//! crate did, #63.)

use bitvue_decode::decoder::{detect_format, validate_frame, ChromaFormat, VideoFormat};
use bitvue_decode::{Av1Decoder, BitDepth, ChromaSubsampling, YuvFileParams};

fn test_data(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../test_data/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("test_data/{name}: {e}"))
}

/// The `pts` of every packet of an IVF file (12-byte frame headers after the 32-byte file header).
fn ivf_pts(data: &[u8]) -> Vec<i64> {
    let (mut pos, mut out) = (32usize, Vec::new());
    while pos + 12 <= data.len() {
        let size = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        out.push(u64::from_le_bytes(data[pos + 4..pos + 12].try_into().unwrap()) as i64);
        pos += 12 + size;
    }
    out
}

fn params(width: u32, height: u32, chroma: ChromaSubsampling, depth: BitDepth) -> YuvFileParams {
    YuvFileParams {
        width,
        height,
        chroma_subsampling: chroma,
        bit_depth: depth,
        frame_rate: (30, 1),
    }
}

#[test]
fn raw_frame_size_follows_chroma_format_and_bit_depth() {
    use BitDepth::{Bit10, Bit12, Bit8};
    use ChromaSubsampling::{Mono, Yuv420, Yuv422, Yuv444};
    // 1920 x 1080: luma 2,073,600 samples; chroma 1/4, 1/2, 1 of that per plane, twice
    for (chroma, depth, bytes) in [
        (Yuv420, Bit8, 3_110_400),
        (Yuv422, Bit8, 4_147_200),
        (Yuv444, Bit8, 6_220_800),
        (Mono, Bit8, 2_073_600),
        (Yuv420, Bit10, 6_220_800),
        (Yuv444, Bit12, 12_441_600),
        (Mono, Bit10, 4_147_200),
    ] {
        let p = params(1920, 1080, chroma, depth);
        assert_eq!(p.frame_size_bytes(), bytes, "{chroma:?} {depth:?}");
        assert_eq!(
            p.y_plane_size(),
            2_073_600 * depth.bytes_per_sample(),
            "{chroma:?} {depth:?}"
        );
    }
}

#[test]
fn bit_depth_sets_the_bytes_per_sample() {
    for (depth, bits, bytes) in [
        (BitDepth::Bit8, 8, 1),
        (BitDepth::Bit10, 10, 2),
        (BitDepth::Bit12, 12, 2),
    ] {
        assert_eq!(depth.bits(), bits);
        assert_eq!(depth.bytes_per_sample(), bytes);
    }
}

#[test]
fn a_real_ivf_decodes_to_every_frame_with_its_size_depth_and_chroma_format() {
    let data = test_data("av1_aomenc_testsrc2.ivf");
    let frames = Av1Decoder::new().unwrap().decode_all(&data).unwrap();
    // ffprobe: 12 frames, 320x240, yuv420p
    assert_eq!(frames.len(), 12);
    for (i, f) in frames.iter().enumerate() {
        assert_eq!((f.width, f.height, f.bit_depth), (320, 240, 8), "frame {i}");
        assert_eq!(f.chroma_format, ChromaFormat::Yuv420, "frame {i}");
        // planes are packed rows; `*_stride` reports the decoder's own (padded) stride
        assert_eq!(f.y_plane.len(), 320 * 240, "frame {i}");
        assert!(f.y_stride >= 320, "frame {i}");
        for plane in [&f.u_plane, &f.v_plane] {
            assert_eq!(
                plane.as_ref().map(|p| p.len()),
                Some(160 * 120),
                "frame {i}"
            );
        }
        validate_frame(f).unwrap_or_else(|e| panic!("frame {i}: {e}"));
    }
}

#[test]
fn decoded_frames_carry_the_pts_of_their_ivf_packets_in_order() {
    let data = test_data("av1_aomenc_testsrc2.ivf");
    let pts = ivf_pts(&data);
    let frames = Av1Decoder::new().unwrap().decode_all(&data).unwrap();
    let got: Vec<i64> = frames.iter().map(|f| f.timestamp).collect();
    assert_eq!(pts, (0..12).collect::<Vec<i64>>(), "the clip's own pts");
    assert_eq!(got, pts);
}

#[test]
fn detect_format_tells_each_container_and_bitstream_from_its_signature() {
    for (file, expected) in [
        ("av1_aomenc_testsrc2.ivf", VideoFormat::Ivf),
        ("sample_vp9.ivf", VideoFormat::Ivf),
        ("sample_h264.mp4", VideoFormat::Mp4),
        ("sample_hevc.mp4", VideoFormat::Mp4),
        ("sample_vp9.mkv", VideoFormat::Mkv),
        ("sample_vp9.webm", VideoFormat::Mkv),
        ("avc_test.h264", VideoFormat::AnnexB),
        ("hevc_test.hevc", VideoFormat::AnnexB),
    ] {
        assert_eq!(detect_format(&test_data(file)), expected, "{file}");
    }
    assert_eq!(detect_format(&[]), VideoFormat::Unknown);
    assert_eq!(detect_format(b"DKI"), VideoFormat::Unknown);
    assert_eq!(detect_format(b"not a video file"), VideoFormat::Unknown);
}

#[test]
fn a_truncated_ivf_is_an_error_not_a_panic_or_a_partial_success() {
    let data = test_data("av1_aomenc_testsrc2.ivf");
    for cut in [33, 100, data.len() / 2, data.len() - 1] {
        let result = Av1Decoder::new().unwrap().decode_all(&data[..cut]);
        assert!(
            result.is_err(),
            "a {cut}-byte prefix of {} decoded",
            data.len()
        );
    }
}

bitvue_test_alloc::install!();
