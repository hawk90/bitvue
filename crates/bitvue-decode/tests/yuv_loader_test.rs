//! `YuvLoader` on real raw-YUV and Y4M files written to a temporary directory. (This file used to
//! hold ten tests that defined their own structs and asserted on literals they had just written,
//! so they could not fail whatever the loader did, #386.)

use bitvue_decode::decoder::ChromaFormat;
use bitvue_decode::{BitDepth, ChromaSubsampling, YuvFileParams, YuvLoader, YuvLoaderError};
use std::path::PathBuf;
use tempfile::TempDir;

fn params(width: u32, height: u32, chroma: ChromaSubsampling, depth: BitDepth) -> YuvFileParams {
    YuvFileParams {
        width,
        height,
        chroma_subsampling: chroma,
        bit_depth: depth,
        frame_rate: (30, 1),
    }
}

/// A frame of 4x2 4:2:0, 8 bit: Y = 8 samples, U = V = 2 samples, every byte `base + position`.
fn frame_420(base: u8) -> Vec<u8> {
    (0..12).map(|i| base.wrapping_add(i)).collect()
}

fn write(dir: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn y4m(header: &str, frames: &[Vec<u8>]) -> Vec<u8> {
    let mut out = format!("{header}\n").into_bytes();
    for f in frames {
        out.extend_from_slice(b"FRAME\n");
        out.extend_from_slice(f);
    }
    out
}

#[test]
fn a_raw_yuv_file_needs_its_parameters() {
    let dir = TempDir::new().unwrap();
    let path = write(&dir, "clip.yuv", &frame_420(0));
    assert!(matches!(
        YuvLoader::open(&path, None),
        Err(YuvLoaderError::InvalidFormat)
    ));
    let p = params(4, 2, ChromaSubsampling::Yuv420, BitDepth::Bit8);
    let loader = YuvLoader::open(&path, Some(p)).unwrap();
    assert_eq!((loader.params().width, loader.params().height), (4, 2));
    assert_eq!(loader.current_frame_index(), 0);
}

#[test]
fn raw_frames_come_back_plane_by_plane_then_end_of_file() {
    let dir = TempDir::new().unwrap();
    let mut bytes = frame_420(0);
    bytes.extend(frame_420(100));
    let path = write(&dir, "clip.yuv", &bytes);
    let mut loader = YuvLoader::open(
        &path,
        Some(params(4, 2, ChromaSubsampling::Yuv420, BitDepth::Bit8)),
    )
    .unwrap();

    for (index, base) in [0u8, 100].into_iter().enumerate() {
        let f = loader.read_frame().unwrap().expect("a frame");
        assert_eq!((f.width, f.height, f.bit_depth), (4, 2, 8));
        assert_eq!(f.timestamp, index as i64);
        assert_eq!(&*f.y_plane, &frame_420(base)[..8]);
        assert_eq!(f.u_plane.as_deref(), Some(&frame_420(base)[8..10]));
        assert_eq!(f.v_plane.as_deref(), Some(&frame_420(base)[10..12]));
        assert_eq!((f.y_stride, f.u_stride, f.v_stride), (4, 2, 2));
        assert_eq!(f.chroma_format, ChromaFormat::Yuv420);
        assert_eq!(loader.current_frame_index(), index + 1);
    }
    assert!(loader.read_frame().unwrap().is_none(), "end of file");
}

#[test]
fn each_chroma_format_reads_its_own_plane_sizes() {
    // 4x2: 4:2:0 -> 2 chroma samples per plane, 4:2:2 -> 4, 4:4:4 -> 8, mono -> none
    for (chroma, uv, expected) in [
        (ChromaSubsampling::Yuv420, 2, ChromaFormat::Yuv420),
        (ChromaSubsampling::Yuv422, 4, ChromaFormat::Yuv422),
        (ChromaSubsampling::Yuv444, 8, ChromaFormat::Yuv444),
        (ChromaSubsampling::Mono, 0, ChromaFormat::Monochrome),
    ] {
        let dir = TempDir::new().unwrap();
        let bytes: Vec<u8> = (0..8 + 2 * uv as u8).collect();
        let path = write(&dir, "clip.yuv", &bytes);
        let mut loader =
            YuvLoader::open(&path, Some(params(4, 2, chroma, BitDepth::Bit8))).unwrap();
        let f = loader.read_frame().unwrap().unwrap();
        assert_eq!(f.chroma_format, expected, "{chroma:?}");
        assert_eq!(f.u_plane.as_ref().map(|p| p.len()), (uv > 0).then_some(uv));
        assert_eq!(f.v_plane.as_ref().map(|p| p.len()), (uv > 0).then_some(uv));
        assert!(loader.read_frame().unwrap().is_none(), "{chroma:?}");
    }
}

#[test]
fn ten_bit_samples_are_two_bytes_each() {
    let dir = TempDir::new().unwrap();
    let bytes: Vec<u8> = (0..24).collect(); // 4x2 4:2:0 at 2 bytes: 16 + 4 + 4
    let path = write(&dir, "clip.yuv", &bytes);
    let mut loader = YuvLoader::open(
        &path,
        Some(params(4, 2, ChromaSubsampling::Yuv420, BitDepth::Bit10)),
    )
    .unwrap();
    let f = loader.read_frame().unwrap().unwrap();
    assert_eq!(f.bit_depth, 10);
    assert_eq!(&*f.y_plane, &bytes[..16]);
    assert_eq!(f.u_plane.as_deref(), Some(&bytes[16..20]));
    assert_eq!(f.v_plane.as_deref(), Some(&bytes[20..24]));
}

#[test]
fn a_frame_cut_short_is_a_size_mismatch_not_a_short_frame() {
    let dir = TempDir::new().unwrap();
    // 5 bytes: the Y plane (8) is not complete
    let path = write(&dir, "clip.yuv", &[1, 2, 3, 4, 5]);
    let mut loader = YuvLoader::open(
        &path,
        Some(params(4, 2, ChromaSubsampling::Yuv420, BitDepth::Bit8)),
    )
    .unwrap();
    assert!(matches!(
        loader.read_frame(),
        Err(YuvLoaderError::FrameSizeMismatch {
            expected: 8,
            actual: 5
        })
    ));
    // Y complete but the chroma planes missing
    let path = write(&dir, "clip2.yuv", &[0; 8]);
    let mut loader = YuvLoader::open(
        &path,
        Some(params(4, 2, ChromaSubsampling::Yuv420, BitDepth::Bit8)),
    )
    .unwrap();
    assert!(loader.read_frame().is_err());
}

#[test]
fn a_y4m_header_gives_size_rate_chroma_and_depth() {
    for (header, chroma, depth) in [
        (
            "YUV4MPEG2 W4 H2 F30000:1001 C420jpeg",
            ChromaSubsampling::Yuv420,
            BitDepth::Bit8,
        ),
        (
            "YUV4MPEG2 W4 H2 F30000:1001 C422",
            ChromaSubsampling::Yuv422,
            BitDepth::Bit8,
        ),
        (
            "YUV4MPEG2 W4 H2 F30000:1001 C444",
            ChromaSubsampling::Yuv444,
            BitDepth::Bit8,
        ),
        (
            "YUV4MPEG2 W4 H2 F30000:1001 Cmono",
            ChromaSubsampling::Mono,
            BitDepth::Bit8,
        ),
        (
            "YUV4MPEG2 W4 H2 F30000:1001 C420p10",
            ChromaSubsampling::Yuv420,
            BitDepth::Bit10,
        ),
        (
            "YUV4MPEG2 W4 H2 F30000:1001 C420p12",
            ChromaSubsampling::Yuv420,
            BitDepth::Bit12,
        ),
    ] {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, "clip.y4m", &y4m(header, &[]));
        let loader = YuvLoader::open(&path, None).unwrap();
        let p = loader.params();
        assert_eq!(
            (p.width, p.height, p.frame_rate),
            (4, 2, (30000, 1001)),
            "{header}"
        );
        assert_eq!(p.chroma_subsampling, chroma, "{header}");
        assert_eq!(p.bit_depth, depth, "{header}");
    }
    // no chroma tag: 4:2:0, 8 bit, 25 fps
    let dir = TempDir::new().unwrap();
    let path = write(&dir, "plain.y4m", &y4m("YUV4MPEG2 W6 H4", &[]));
    let p = YuvLoader::open(&path, None).unwrap().params().clone();
    assert_eq!((p.width, p.height, p.frame_rate), (6, 4, (25, 1)));
    assert_eq!(p.chroma_subsampling, ChromaSubsampling::Yuv420);
}

#[test]
fn a_bad_y4m_header_is_named_in_the_error() {
    let dir = TempDir::new().unwrap();
    let open = |name: &str, header: &str| {
        let path = write(&dir, name, &y4m(header, &[]));
        YuvLoader::open(&path, None).err().expect("an error")
    };
    assert!(matches!(
        open("a.y4m", "NOTY4M W4 H2"),
        YuvLoaderError::InvalidY4mHeader(m) if m.contains("YUV4MPEG2")
    ));
    assert!(matches!(
        open("b.y4m", "YUV4MPEG2 H2"),
        YuvLoaderError::InvalidY4mHeader(m) if m.contains("width or height")
    ));
    assert!(matches!(
        open("c.y4m", "YUV4MPEG2 Wx H2"),
        YuvLoaderError::InvalidY4mHeader(m) if m.contains("width")
    ));
    assert!(matches!(
        open("d.y4m", "YUV4MPEG2 W4 H2 C411"),
        YuvLoaderError::UnsupportedChromaSubsampling(c) if c == "411"
    ));
}

#[test]
fn y4m_frames_are_read_past_their_frame_headers() {
    let dir = TempDir::new().unwrap();
    let path = write(
        &dir,
        "clip.y4m",
        &y4m("YUV4MPEG2 W4 H2 F30:1 C420", &[frame_420(0), frame_420(50)]),
    );
    let mut loader = YuvLoader::open(&path, None).unwrap();
    let first = loader.read_frame().unwrap().unwrap();
    assert_eq!(&*first.y_plane, &frame_420(0)[..8]);
    let second = loader.read_frame().unwrap().unwrap();
    assert_eq!(&*second.y_plane, &frame_420(50)[..8]);
    assert_eq!(second.v_plane.as_deref(), Some(&frame_420(50)[10..12]));
    assert!(loader.read_frame().unwrap().is_none());

    // a frame header that is not "FRAME"
    let mut bad = y4m("YUV4MPEG2 W4 H2 C420", &[]);
    bad.extend_from_slice(b"XXXXX\n");
    bad.extend(frame_420(0));
    let path = write(&dir, "bad.y4m", &bad);
    let mut loader = YuvLoader::open(&path, None).unwrap();
    assert!(matches!(
        loader.read_frame(),
        Err(YuvLoaderError::InvalidFormat)
    ));
}

#[test]
fn seeking_lands_on_the_requested_frame_in_raw_and_y4m_files() {
    let frames: Vec<Vec<u8>> = (0..4).map(|i| frame_420(i * 20)).collect();
    let dir = TempDir::new().unwrap();
    let raw = write(&dir, "clip.yuv", &frames.concat());
    let y4m_path = write(
        &dir,
        "clip.y4m",
        &y4m("YUV4MPEG2 W4 H2 F30:1 C420", &frames),
    );
    for (path, p) in [
        (
            raw,
            Some(params(4, 2, ChromaSubsampling::Yuv420, BitDepth::Bit8)),
        ),
        (y4m_path, None),
    ] {
        let mut loader = YuvLoader::open(&path, p).unwrap();
        let y_of = |l: &mut YuvLoader| l.read_frame().unwrap().unwrap().y_plane.to_vec();
        loader.seek_to_frame(2).unwrap(); // forward
        assert_eq!(loader.current_frame_index(), 2);
        assert_eq!(y_of(&mut loader), frames[2][..8], "{path:?}");
        loader.seek_to_frame(0).unwrap(); // backward
        assert_eq!(y_of(&mut loader), frames[0][..8], "{path:?}");
        loader.seek_to_frame(3).unwrap();
        assert_eq!(y_of(&mut loader), frames[3][..8], "{path:?}");
        loader.seek_to_frame(1).unwrap(); // backward again, not from the start
        assert_eq!(y_of(&mut loader), frames[1][..8], "{path:?}");
        // seeking to where it already is changes nothing
        let at = loader.current_frame_index();
        loader.seek_to_frame(at).unwrap();
        assert_eq!(loader.current_frame_index(), at);
    }
}

#[test]
fn seeking_past_the_end_of_a_y4m_file_is_an_error() {
    let dir = TempDir::new().unwrap();
    let path = write(
        &dir,
        "clip.y4m",
        &y4m("YUV4MPEG2 W4 H2 C420", &[frame_420(0), frame_420(1)]),
    );
    let mut loader = YuvLoader::open(&path, None).unwrap();
    assert!(loader.seek_to_frame(5).is_err());
}

#[test]
fn paths_are_checked_before_anything_is_read() {
    let dir = TempDir::new().unwrap();
    let p = params(4, 2, ChromaSubsampling::Yuv420, BitDepth::Bit8);
    // extension
    let path = write(&dir, "clip.mp4", &frame_420(0));
    assert!(matches!(
        YuvLoader::open(&path, Some(p.clone())),
        Err(YuvLoaderError::InvalidExtension(ext, _)) if ext == "mp4"
    ));
    // missing file
    assert!(matches!(
        YuvLoader::open(dir.path().join("missing.yuv"), Some(p.clone())),
        Err(YuvLoaderError::PathNotFound(_))
    ));
    // parent-directory components
    let traversal = dir.path().join("sub").join("..").join("clip.yuv");
    assert!(matches!(
        YuvLoader::open(&traversal, Some(p)),
        Err(YuvLoaderError::PathTraversal(_))
    ));
}

bitvue_test_alloc::install!();
