//! Raw YUV file I/O and 8-bit plane normalization -- reads a reference file's frames off disk and
//! converts both reference and decoded-bitstream frames into the common [`Planes8`] shape
//! [`super::compare`]'s diff/metrics algorithms operate on.

use super::{Crop, Session, YuvFormat};
use crate::decode_bridge::DecodedYuvFrame;

pub(crate) fn bytes_per_sample(bitdepth: u8) -> usize {
    if bitdepth > 8 {
        2
    } else {
        1
    }
}

/// Per-frame byte size for `format`/`bitdepth` at `width`x`height` -- the same total regardless of
/// planar (I420/I422/I444) vs semi-planar (NV12/NV21) layout, since both store the same sample
/// count, just arranged differently in memory.
pub(crate) fn frame_byte_size(width: u32, height: u32, format: YuvFormat, bitdepth: u8) -> usize {
    let bps = bytes_per_sample(bitdepth);
    let (h_ratio, v_ratio) = format.chroma_ratio();
    let w = width as usize;
    let h = height as usize;
    let cw = width.div_ceil(h_ratio) as usize;
    let ch = height.div_ceil(v_ratio) as usize;
    let luma = w * h;
    let chroma = 2 * cw * ch;
    (luma + chroma) * bps
}

/// One frame's planes, normalized to 8-bit samples, post-crop -- the common representation both
/// the reference file and a real decoded bitstream frame get converted into before diffing.
/// `pub(super)`: an implementation detail of the `debug_yuv` subsystem, never exposed on the wire
/// (always converted to a [`DecodedYuvFrame`] first via `planes_to_wire`), but shared across the
/// `planes`/`compare` submodule split, hence not fully private to this file.
pub(super) struct Planes8 {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) chroma_width: u32,
    pub(super) chroma_height: u32,
    pub(super) y: Vec<u8>,
    pub(super) u: Vec<u8>,
    pub(super) v: Vec<u8>,
}

fn crop_plane(
    src: &[u8],
    src_w: usize,
    src_h: usize,
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
) -> (Vec<u8>, usize, usize) {
    let new_w = src_w.saturating_sub(left + right).max(1);
    let new_h = src_h.saturating_sub(top + bottom).max(1);
    let mut out = Vec::with_capacity(new_w * new_h);
    for row in 0..new_h {
        let src_row = row + top;
        if src_row >= src_h {
            out.resize(out.len() + new_w, 0);
            continue;
        }
        let start = src_row * src_w + left;
        let end = (start + new_w).min(src.len());
        if start >= src.len() || start >= end {
            out.resize(out.len() + new_w, 0);
            continue;
        }
        out.extend_from_slice(&src[start..end]);
        out.resize((row + 1) * new_w, 0);
    }
    (out, new_w, new_h)
}

fn apply_crop(planes: Planes8, h_ratio: u32, v_ratio: u32, crop: Crop) -> Planes8 {
    if crop.left == 0 && crop.right == 0 && crop.top == 0 && crop.bottom == 0 {
        return planes;
    }
    let (y, yw, yh) = crop_plane(
        &planes.y,
        planes.width as usize,
        planes.height as usize,
        crop.left as usize,
        crop.right as usize,
        crop.top as usize,
        crop.bottom as usize,
    );
    let cl = (crop.left / h_ratio) as usize;
    let cr = (crop.right / h_ratio) as usize;
    let ct = (crop.top / v_ratio) as usize;
    let cb = (crop.bottom / v_ratio) as usize;
    let (u, cw, ch) = crop_plane(
        &planes.u,
        planes.chroma_width as usize,
        planes.chroma_height as usize,
        cl,
        cr,
        ct,
        cb,
    );
    let (v, _, _) = crop_plane(
        &planes.v,
        planes.chroma_width as usize,
        planes.chroma_height as usize,
        cl,
        cr,
        ct,
        cb,
    );
    Planes8 {
        width: yw as u32,
        height: yh as u32,
        chroma_width: cw as u32,
        chroma_height: ch as u32,
        y,
        u,
        v,
    }
}

/// Reads reference frame `index` (already offset-adjusted by the caller), downshifts to 8-bit if
/// the file is higher bit depth, de-interleaves NV12/NV21, and applies `session.crop`. Reopens the
/// file on every call rather than keeping a handle in `Session` -- simplest thing that works, same
/// "correctness first, not perf first" tradeoff `decode_bridge` already makes for stream decode.
pub(super) fn read_reference_frame(session: &Session, index: usize) -> Result<Planes8, String> {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    if index >= session.frame_count {
        return Err(format!(
            "reference frame {index} out of range (file has {} frames)",
            session.frame_count
        ));
    }
    let mut file =
        File::open(&session.path).map_err(|e| format!("reopen {}: {e}", session.path))?;
    file.seek(SeekFrom::Start((index * session.frame_size) as u64))
        .map_err(|e| format!("seek: {e}"))?;
    let mut raw = vec![0u8; session.frame_size];
    file.read_exact(&mut raw)
        .map_err(|e| format!("read reference frame {index}: {e}"))?;

    let bps = bytes_per_sample(session.bitdepth);
    let downshift = session.bitdepth.saturating_sub(8);
    let w = session.width as usize;
    let h = session.height as usize;
    let (h_ratio, v_ratio) = session.format.chroma_ratio();
    let cw = session.width.div_ceil(h_ratio) as usize;
    let ch = session.height.div_ceil(v_ratio) as usize;

    let unpack = |bytes: &[u8], count: usize| -> Vec<u8> {
        if bps == 1 {
            bytes[..count].to_vec()
        } else {
            (0..count)
                .map(|i| {
                    let sample = (bytes[i * 2] as u16) | ((bytes[i * 2 + 1] as u16) << 8);
                    (sample >> downshift) as u8
                })
                .collect()
        }
    };

    let y_len = w * h;
    let c_len = cw * ch;
    let (y, u, v) = match session.format {
        YuvFormat::I420 | YuvFormat::I422 | YuvFormat::I444 => {
            let y_bytes = &raw[0..y_len * bps];
            let u_bytes = &raw[y_len * bps..(y_len + c_len) * bps];
            let v_bytes = &raw[(y_len + c_len) * bps..(y_len + 2 * c_len) * bps];
            (
                unpack(y_bytes, y_len),
                unpack(u_bytes, c_len),
                unpack(v_bytes, c_len),
            )
        }
        YuvFormat::Nv12 | YuvFormat::Nv21 => {
            let y_bytes = &raw[0..y_len * bps];
            let uv_bytes = &raw[y_len * bps..(y_len + 2 * c_len) * bps];
            let uv = unpack(uv_bytes, 2 * c_len);
            let mut u = Vec::with_capacity(c_len);
            let mut v = Vec::with_capacity(c_len);
            for i in 0..c_len {
                let (a, b) = (uv[i * 2], uv[i * 2 + 1]);
                if session.format == YuvFormat::Nv12 {
                    u.push(a);
                    v.push(b);
                } else {
                    v.push(a);
                    u.push(b);
                }
            }
            (unpack(y_bytes, y_len), u, v)
        }
    };

    let planes = Planes8 {
        width: session.width,
        height: session.height,
        chroma_width: cw as u32,
        chroma_height: ch as u32,
        y,
        u,
        v,
    };
    Ok(apply_crop(planes, h_ratio, v_ratio, session.crop))
}

/// Converts a `decode_bridge::DecodedYuvFrame` (native bit depth, tightly packed) into the same
/// 8-bit `Planes8` representation used for the reference file, applying the session's crop and
/// chroma ratio (from the *decoded* frame's own `chroma_subsampling`, not the reference file's
/// declared format -- they're expected to match for a meaningful diff, but crop math should use
/// whichever frame it's actually cropping).
pub(super) fn decoded_to_planes8(frame: &DecodedYuvFrame, crop: Crop) -> Planes8 {
    let bps = bytes_per_sample(frame.bit_depth);
    let downshift = frame.bit_depth.saturating_sub(8);
    let unpack = |bytes: &[u8]| -> Vec<u8> {
        if bps == 1 {
            bytes.to_vec()
        } else {
            bytes
                .chunks_exact(2)
                .map(|c| {
                    let sample = (c[0] as u16) | ((c[1] as u16) << 8);
                    (sample >> downshift) as u8
                })
                .collect()
        }
    };
    let y = unpack(&frame.bytes[..frame.y_len]);
    let u = unpack(&frame.bytes[frame.y_len..frame.y_len + frame.u_len]);
    let v =
        unpack(&frame.bytes[frame.y_len + frame.u_len..frame.y_len + frame.u_len + frame.v_len]);
    let (h_ratio, v_ratio) = match frame.chroma_subsampling {
        "422" => (2, 1),
        "444" => (1, 1),
        _ => (2, 2), // "420" and any unrecognized value
    };
    let chroma_height = frame.u_len.checked_div(frame.u_stride).unwrap_or(0) as u32;
    let planes = Planes8 {
        width: frame.width,
        height: frame.height,
        chroma_width: frame.u_stride as u32,
        chroma_height,
        y,
        u,
        v,
    };
    apply_crop(planes, h_ratio, v_ratio, crop)
}

pub(super) fn planes_to_wire(
    planes: &Planes8,
    chroma_subsampling: &'static str,
) -> DecodedYuvFrame {
    DecodedYuvFrame {
        width: planes.width,
        height: planes.height,
        bit_depth: 8,
        chroma_subsampling,
        y_stride: planes.width as usize,
        u_stride: planes.chroma_width as usize,
        v_stride: planes.chroma_width as usize,
        y_len: planes.y.len(),
        u_len: planes.u.len(),
        v_len: planes.v.len(),
        bytes: [
            planes.y.as_slice(),
            planes.u.as_slice(),
            planes.v.as_slice(),
        ]
        .concat(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_yuv::LoadParams;
    use crate::test_support::write_i420_fixture;
    use std::io::Write;

    #[test]
    fn frame_byte_size_i420_matches_the_standard_1_5x_formula() {
        // 4x4 I420: 16 luma + 2*(2*2) chroma = 24 bytes.
        assert_eq!(frame_byte_size(4, 4, YuvFormat::I420, 8), 24);
    }

    #[test]
    fn frame_byte_size_i444_has_full_res_chroma() {
        // 4x4 I444: 16 luma + 2*16 chroma = 48 bytes.
        assert_eq!(frame_byte_size(4, 4, YuvFormat::I444, 8), 48);
    }

    #[test]
    fn frame_byte_size_odd_dimensions_round_up_chroma() {
        // 3x3 I420: chroma dims ceil(3/2)=2 each way -> 9 + 2*4 = 17 bytes.
        assert_eq!(frame_byte_size(3, 3, YuvFormat::I420, 8), 17);
    }

    #[test]
    fn frame_byte_size_10bit_doubles_bytes() {
        assert_eq!(frame_byte_size(4, 4, YuvFormat::I420, 10), 48);
    }

    #[test]
    fn read_reference_frame_returns_the_right_frame_at_the_right_offset() {
        // 3 frames filled with 10, 20, 30 respectively -- reading index 1 should see all-20s.
        let file = write_i420_fixture(4, 4, &[10, 20, 30]);
        let session = super::super::load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        let planes = read_reference_frame(&session, 1).unwrap();
        assert!(planes.y.iter().all(|&b| b == 20));
        assert!(planes.u.iter().all(|&b| b == 20));
        assert_eq!(planes.width, 4);
        assert_eq!(planes.height, 4);
    }

    #[test]
    fn read_reference_frame_out_of_range_is_a_real_error() {
        let file = write_i420_fixture(4, 4, &[0]);
        let session = super::super::load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        assert!(read_reference_frame(&session, 5).is_err());
    }

    #[test]
    fn crop_trims_luma_and_chroma_by_the_declared_ratio() {
        let file = write_i420_fixture(8, 8, &[0]);
        let mut session = super::super::load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 8,
            height: 8,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        session.crop = Crop {
            left: 2,
            right: 2,
            top: 0,
            bottom: 0,
        };
        let planes = read_reference_frame(&session, 0).unwrap();
        assert_eq!(planes.width, 4, "8 - 2 - 2 luma cols");
        assert_eq!(planes.height, 8);
        assert_eq!(planes.chroma_width, 2, "chroma crop halved: 1 + 1");
        assert_eq!(planes.chroma_height, 4);
    }

    #[test]
    fn nv12_and_i420_produce_identical_planes_for_the_same_logical_content() {
        // Build one I420 frame and one NV12 frame with the same logical Y/U/V values, confirm
        // read_reference_frame de-interleaves NV12 back to the same result.
        let (w, h) = (4u32, 4u32);
        let y = vec![100u8; 16];
        let u = vec![50u8; 4];
        let v = vec![150u8; 4];

        let mut i420_bytes = Vec::new();
        i420_bytes.extend_from_slice(&y);
        i420_bytes.extend_from_slice(&u);
        i420_bytes.extend_from_slice(&v);
        let mut i420_file = tempfile::NamedTempFile::new().unwrap();
        i420_file.write_all(&i420_bytes).unwrap();

        let mut nv12_bytes = Vec::new();
        nv12_bytes.extend_from_slice(&y);
        for i in 0..4 {
            nv12_bytes.push(u[i]);
            nv12_bytes.push(v[i]);
        }
        let mut nv12_file = tempfile::NamedTempFile::new().unwrap();
        nv12_file.write_all(&nv12_bytes).unwrap();

        let i420_session = super::super::load(LoadParams {
            path: i420_file.path().to_str().unwrap().to_string(),
            width: w,
            height: h,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        let nv12_session = super::super::load(LoadParams {
            path: nv12_file.path().to_str().unwrap().to_string(),
            width: w,
            height: h,
            format: YuvFormat::Nv12,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();

        let i420_planes = read_reference_frame(&i420_session, 0).unwrap();
        let nv12_planes = read_reference_frame(&nv12_session, 0).unwrap();
        assert_eq!(i420_planes.y, nv12_planes.y);
        assert_eq!(i420_planes.u, nv12_planes.u);
        assert_eq!(i420_planes.v, nv12_planes.v);
    }
}
