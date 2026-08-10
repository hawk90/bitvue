//! Debug YUV (VQ Analyzer "Load Reference YUV") -- raw planar/semi-planar YUV file reading and
//! frame-level PSNR/SSIM/diff comparison against stream A's decoded output. Distinct from
//! `decode_bridge`: that module decodes *compressed* bitstream frames; this reads *uncompressed*
//! samples straight off disk, no decode -- the "ground truth" reference a user loads via
//! Debug -> Open debug YUV... to check a decoder/encoder against (see
//! `frontend/contexts/YuvDiffContext.tsx`'s module doc for the workflow this backs).
//!
//! Session state (path/dimensions/format/bitdepth/offset/crop) lives in a `Mutex<Option<Session>>`
//! held by `main.rs` (`DebugYuvSlot`), not `bitvue_engine::Core` -- it isn't per-`StreamId` (A/B)
//! state, and `Core` is a leaf crate that can't depend on filesystem/decode concerns anyway (same
//! reasoning as `decode_bridge` living here rather than in `bitvue-indexer`).
//!
//! **Scoping decision**: diff/amplified/metrics all operate at 8-bit precision regardless of the
//! reference file's declared bit depth -- samples above 8 bits are right-shifted down before
//! comparison. This matches `decode_bridge`'s existing wire format (always 8-bit-packed bytes) and
//! the frontend's `VideoCanvas`/`yuv_to_rgb` rendering pipeline, neither of which has a 10/12/16-bit
//! path today. Real high-bit-depth comparison would need a second wire format and renderer path --
//! out of scope here, flagged for later if a real 10-bit+ test asset needs it. "reference" mode
//! (no decoded-frame comparison) still reads the file at its declared bit depth, just downshifts
//! for the wire the same way, so what's displayed is consistent across all four modes.

use crate::decode_bridge::{self, DecodedYuvFrame};
use bitvue_engine::{Core, StreamId};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YuvFormat {
    I420,
    Nv12,
    Nv21,
    I422,
    I444,
}

impl YuvFormat {
    /// (horizontal, vertical) chroma subsampling ratio -- how much smaller each chroma plane's
    /// dimension is than luma's. Same ratio for planar (I420/I422/I444) and semi-planar
    /// (NV12/NV21) layouts; only the byte arrangement in the file differs, not the sample grid.
    fn chroma_ratio(self) -> (u32, u32) {
        match self {
            YuvFormat::I420 | YuvFormat::Nv12 | YuvFormat::Nv21 => (2, 2),
            YuvFormat::I422 => (2, 1),
            YuvFormat::I444 => (1, 1),
        }
    }

    fn chroma_subsampling_str(self) -> &'static str {
        match self {
            YuvFormat::I420 | YuvFormat::Nv12 | YuvFormat::Nv21 => "420",
            YuvFormat::I422 => "422",
            YuvFormat::I444 => "444",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct Crop {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoadParams {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: YuvFormat,
    pub bitdepth: u8,
    #[serde(default)]
    pub picture_offset: i64,
    #[serde(default)]
    pub crop: Option<Crop>,
}

/// A loaded reference-YUV session. `picture_offset`/`crop` are mutated in place by
/// `set_debug_yuv_offset`/`set_debug_yuv_crop` (see `main.rs`) rather than replaced wholesale --
/// re-`load`ing would re-stat the file and re-validate frame_size for no reason.
#[derive(Debug, Clone)]
pub struct Session {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: YuvFormat,
    pub bitdepth: u8,
    pub picture_offset: i64,
    pub crop: Crop,
    pub frame_size: usize,
    pub frame_count: usize,
}

fn bytes_per_sample(bitdepth: u8) -> usize {
    if bitdepth > 8 {
        2
    } else {
        1
    }
}

/// Per-frame byte size for `format`/`bitdepth` at `width`x`height` -- the same total regardless of
/// planar (I420/I422/I444) vs semi-planar (NV12/NV21) layout, since both store the same sample
/// count, just arranged differently in memory.
fn frame_byte_size(width: u32, height: u32, format: YuvFormat, bitdepth: u8) -> usize {
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

pub fn load(params: LoadParams) -> Result<Session, String> {
    if params.width == 0 || params.height == 0 {
        return Err("width/height must be non-zero".to_string());
    }
    if !matches!(params.bitdepth, 8 | 10 | 12 | 16) {
        return Err(format!("unsupported bit depth: {}", params.bitdepth));
    }
    let meta =
        std::fs::metadata(&params.path).map_err(|e| format!("cannot open {}: {e}", params.path))?;
    let frame_size = frame_byte_size(params.width, params.height, params.format, params.bitdepth);
    let frame_count = (meta.len() as usize) / frame_size;
    if frame_count == 0 {
        return Err(format!(
            "file is smaller than one frame ({} bytes < {frame_size} bytes/frame -- check resolution/format/bit depth)",
            meta.len()
        ));
    }
    Ok(Session {
        path: params.path,
        width: params.width,
        height: params.height,
        format: params.format,
        bitdepth: params.bitdepth,
        picture_offset: params.picture_offset,
        crop: params.crop.unwrap_or_default(),
        frame_size,
        frame_count,
    })
}

/// One frame's planes, normalized to 8-bit samples, post-crop -- the common representation both
/// the reference file and a real decoded bitstream frame get converted into before diffing.
struct Planes8 {
    width: u32,
    height: u32,
    chroma_width: u32,
    chroma_height: u32,
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
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
fn read_reference_frame(session: &Session, index: usize) -> Result<Planes8, String> {
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
fn decoded_to_planes8(frame: &DecodedYuvFrame, crop: Crop) -> Planes8 {
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

fn planes_to_wire(planes: &Planes8, chroma_subsampling: &'static str) -> DecodedYuvFrame {
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

/// Reads stream A's raw bytes and decodes `frame_index` via the same `decode_bridge` logic
/// `get_decoded_frame_yuv` uses -- shared here so "decoded"/"diff"/"amplified" modes see exactly
/// the same pixels the main preview pane would.
fn decode_stream_a_frame(core: &Core, frame_index: usize) -> Result<DecodedYuvFrame, String> {
    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = state
        .byte_cache
        .as_ref()
        .ok_or_else(|| "stream A not open".to_string())?
        .clone();
    drop(state);
    let full_len = byte_cache.len() as usize;
    let data = byte_cache
        .read_range(0, full_len)
        .map_err(|e| e.to_string())?;
    decode_bridge::get_decoded_frame_yuv(data, frame_index)
}

/// `frame_index` is the *display* index (stream A's decode order); the reference file is read at
/// `frame_index + session.picture_offset`.
pub fn get_frame(
    core: &Core,
    session: &Session,
    frame_index: usize,
    mode: &str,
    amplify: Option<u32>,
) -> Result<DecodedYuvFrame, String> {
    let ref_index_signed = frame_index as i64 + session.picture_offset;
    if ref_index_signed < 0 {
        return Err(format!(
            "frame {frame_index} + offset {} is negative -- no reference frame there",
            session.picture_offset
        ));
    }
    let ref_index = ref_index_signed as usize;

    if mode == "reference" {
        let planes = read_reference_frame(session, ref_index)?;
        return Ok(planes_to_wire(
            &planes,
            session.format.chroma_subsampling_str(),
        ));
    }

    let decoded_wire = decode_stream_a_frame(core, frame_index)?;
    if mode == "decoded" {
        return Ok(decoded_wire);
    }
    if mode != "diff" && mode != "amplified" {
        return Err(format!("unknown debug YUV display mode: {mode}"));
    }

    let decoded_planes = decoded_to_planes8(&decoded_wire, session.crop);
    let ref_planes = read_reference_frame(session, ref_index)?;
    if decoded_planes.width != ref_planes.width || decoded_planes.height != ref_planes.height {
        return Err(format!(
            "decoded frame is {}x{} but reference frame (after crop) is {}x{} -- \
             load a matching-resolution reference or adjust crop",
            decoded_planes.width, decoded_planes.height, ref_planes.width, ref_planes.height
        ));
    }

    // Luma-only diff visualization (neutral mid-gray chroma) -- a standard debug-diff convention,
    // not an attempt at a full color diff. "diff" shows |decoded - reference|; "amplified" centers
    // the signed difference at 128 so both directions stay visible, scaled by `amplify`.
    let amp = amplify.unwrap_or(1).max(1) as i32;
    let y: Vec<u8> = decoded_planes
        .y
        .iter()
        .zip(ref_planes.y.iter())
        .map(|(d, r)| {
            let delta = *d as i32 - *r as i32;
            if mode == "amplified" {
                (128 + delta * amp).clamp(0, 255) as u8
            } else {
                delta.unsigned_abs().min(255) as u8
            }
        })
        .collect();
    let chroma_len = decoded_planes.u.len();
    let u = vec![128u8; chroma_len];
    let v = vec![128u8; chroma_len];

    Ok(DecodedYuvFrame {
        width: decoded_planes.width,
        height: decoded_planes.height,
        bit_depth: 8,
        chroma_subsampling: session.format.chroma_subsampling_str(),
        y_stride: decoded_planes.width as usize,
        u_stride: decoded_planes.chroma_width as usize,
        v_stride: decoded_planes.chroma_width as usize,
        y_len: y.len(),
        u_len: u.len(),
        v_len: v.len(),
        bytes: [y.as_slice(), u.as_slice(), v.as_slice()].concat(),
    })
}

pub struct DiffMetrics {
    pub frame_index: usize,
    pub psnr_y: f64,
    pub psnr_u: f64,
    pub psnr_v: f64,
    pub psnr_avg: f64,
    pub ssim_y: f64,
    pub max_diff_y: u8,
    pub has_mismatch: bool,
}

/// PSNR/SSIM for `frame_index` (display order) against the reference file at
/// `frame_index + session.picture_offset`, via `bitvue-metrics` (already-tested, real PSNR/SSIM --
/// not reimplemented here).
pub fn compute_frame_metrics(
    core: &Core,
    session: &Session,
    frame_index: usize,
) -> Result<DiffMetrics, String> {
    let ref_index_signed = frame_index as i64 + session.picture_offset;
    if ref_index_signed < 0 {
        return Err(format!(
            "frame {frame_index} + offset {} is negative -- no reference frame there",
            session.picture_offset
        ));
    }
    let decoded_wire = decode_stream_a_frame(core, frame_index)?;
    let decoded_planes = decoded_to_planes8(&decoded_wire, session.crop);
    let ref_planes = read_reference_frame(session, ref_index_signed as usize)?;

    if decoded_planes.width != ref_planes.width || decoded_planes.height != ref_planes.height {
        return Err(format!(
            "decoded frame is {}x{} but reference frame (after crop) is {}x{} -- \
             load a matching-resolution reference or adjust crop",
            decoded_planes.width, decoded_planes.height, ref_planes.width, ref_planes.height
        ));
    }

    let reference = bitvue_metrics::YuvFrame {
        y: &ref_planes.y,
        u: &ref_planes.u,
        v: &ref_planes.v,
        width: ref_planes.width as usize,
        height: ref_planes.height as usize,
        chroma_width: ref_planes.chroma_width as usize,
        chroma_height: ref_planes.chroma_height as usize,
    };
    let distorted = bitvue_metrics::YuvFrame {
        y: &decoded_planes.y,
        u: &decoded_planes.u,
        v: &decoded_planes.v,
        width: decoded_planes.width as usize,
        height: decoded_planes.height as usize,
        chroma_width: decoded_planes.chroma_width as usize,
        chroma_height: decoded_planes.chroma_height as usize,
    };
    let (psnr_y, psnr_u, psnr_v) =
        bitvue_metrics::psnr_yuv(&reference, &distorted).map_err(|e| e.to_string())?;
    let (ssim_y, _ssim_u, _ssim_v) =
        bitvue_metrics::ssim_yuv(&reference, &distorted).map_err(|e| e.to_string())?;
    // `psnr()` returns `f64::INFINITY` for byte-identical planes -- JSON has no representation for
    // that (serde_json silently turns it into `null`), so clamp to a finite sentinel. 100 dB is
    // comfortably above any real lossy-codec PSNR and matches the frontend's own `fmt()` helper,
    // which already treats any value >= 99.99 dB as "∞" for display.
    const PSNR_INFINITY_SENTINEL: f64 = 100.0;
    let clamp_psnr = |v: f64| {
        if v.is_finite() {
            v
        } else {
            PSNR_INFINITY_SENTINEL
        }
    };
    let (psnr_y, psnr_u, psnr_v) = (clamp_psnr(psnr_y), clamp_psnr(psnr_u), clamp_psnr(psnr_v));
    // 6:1:1 weighted average PSNR -- standard video-quality convention (luma weighted 6x each
    // chroma plane, matching how the eye perceives luma vs chroma error).
    let psnr_avg = (6.0 * psnr_y + psnr_u + psnr_v) / 8.0;
    let max_diff_y = ref_planes
        .y
        .iter()
        .zip(decoded_planes.y.iter())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);

    Ok(DiffMetrics {
        frame_index,
        psnr_y,
        psnr_u,
        psnr_v,
        psnr_avg,
        ssim_y,
        max_diff_y,
        has_mismatch: max_diff_y > 0,
    })
}

/// Scans stream A in decode order, comparing each decoded frame's Y plane against the
/// corresponding reference frame (`decoded_index + picture_offset`), stopping at the first pixel
/// mismatch. Unlike `get_frame`/`compute_frame_metrics` (one `get_decoded_frame_yuv`-style
/// redecode-from-scratch per call), this does a single streaming decode pass -- same reasoning as
/// `decode_bridge::get_thumbnails` batching one decode pass across many requested indices, just
/// applied to "keep decoding until a mismatch instead of until every requested index is hit."
/// Returns `(first_mismatched_frame_index, frames_actually_compared)` -- comparison stops early
/// (without an error) once the reference file runs out of frames to compare against.
pub fn find_first_diff(core: &Core, session: &Session) -> Result<(Option<usize>, usize), String> {
    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = state
        .byte_cache
        .as_ref()
        .ok_or_else(|| "stream A not open".to_string())?
        .clone();
    drop(state);
    let full_len = byte_cache.len() as usize;
    let data = byte_cache
        .read_range(0, full_len)
        .map_err(|e| e.to_string())?;

    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| format!("IVF parse error: {e}"))?;
    let mut dec = bitvue_decode::Av1Decoder::new().map_err(|e| format!("decoder init: {e}"))?;
    let mut decoded_count = 0usize;
    let mut checked = 0usize;
    let mut found: Option<usize> = None;

    let check_one = |decoded: &bitvue_decode::DecodedFrame,
                     index: usize,
                     checked: &mut usize,
                     found: &mut Option<usize>|
     -> bool {
        let ref_index_signed = index as i64 + session.picture_offset;
        if ref_index_signed < 0 || ref_index_signed as usize >= session.frame_count {
            return true; // out of reference frames -- stop
        }
        let Ok(ref_planes) = read_reference_frame(session, ref_index_signed as usize) else {
            return true;
        };
        let decoded_wire = decode_bridge::to_wire(decoded);
        let decoded_planes = decoded_to_planes8(&decoded_wire, session.crop);
        *checked += 1;
        if decoded_planes.width == ref_planes.width
            && decoded_planes.height == ref_planes.height
            && decoded_planes
                .y
                .iter()
                .zip(ref_planes.y.iter())
                .any(|(a, b)| a != b)
        {
            *found = Some(index);
            return true;
        }
        false
    };

    'outer: for f in &frames {
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| format!("decode send: {e}"))?;
        while let Ok(decoded) = dec.get_frame() {
            let index = decoded_count;
            decoded_count += 1;
            if check_one(&decoded, index, &mut checked, &mut found) {
                break 'outer;
            }
        }
    }
    if found.is_none() {
        // Not dec.flush() -- see decode_bridge::get_decoded_frame_yuv's comment: flush() clears
        // dav1d's internal state instead of draining buffered frames.
        let mut remaining = Vec::new();
        dec.drain_decoder_frames(&mut remaining)
            .map_err(|e| e.to_string())?;
        for decoded in &remaining {
            let index = decoded_count;
            decoded_count += 1;
            if check_one(decoded, index, &mut checked, &mut found) {
                break;
            }
        }
    }

    Ok((found, checked))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_i420_fixture(width: u32, height: u32, frames: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let frame_size = frame_byte_size(width, height, YuvFormat::I420, 8);
        for &fill in frames {
            file.write_all(&vec![fill; frame_size]).unwrap();
        }
        file
    }

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
    fn load_computes_real_frame_count_from_file_size() {
        let file = write_i420_fixture(4, 4, &[0, 1, 2]); // 3 frames of 24 bytes each
        let session = load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        assert_eq!(session.frame_count, 3);
        assert_eq!(session.frame_size, 24);
    }

    #[test]
    fn load_file_smaller_than_one_frame_is_a_real_error() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&[0u8; 5]).unwrap();
        let result = load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn read_reference_frame_returns_the_right_frame_at_the_right_offset() {
        // 3 frames filled with 10, 20, 30 respectively -- reading index 1 should see all-20s.
        let file = write_i420_fixture(4, 4, &[10, 20, 30]);
        let session = load(LoadParams {
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
        let session = load(LoadParams {
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
        let mut session = load(LoadParams {
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

        let i420_session = load(LoadParams {
            path: i420_file.path().to_str().unwrap().to_string(),
            width: w,
            height: h,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        let nv12_session = load(LoadParams {
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

    #[test]
    fn get_frame_reference_mode_does_not_need_stream_a_open() {
        let file = write_i420_fixture(4, 4, &[42]);
        let session = load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        let core = Core::new(); // no stream opened at all
        let frame = get_frame(&core, &session, 0, "reference", None).unwrap();
        assert_eq!(frame.width, 4);
        assert_eq!(frame.height, 4);
        assert!(frame.bytes[..frame.y_len].iter().all(|&b| b == 42));
    }

    #[test]
    fn get_frame_decoded_mode_without_open_stream_is_a_real_error() {
        let file = write_i420_fixture(4, 4, &[0]);
        let session = load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        let core = Core::new();
        assert!(get_frame(&core, &session, 0, "decoded", None).is_err());
    }

    #[test]
    fn get_frame_unknown_mode_is_a_real_error() {
        let file = write_i420_fixture(4, 4, &[0]);
        let session = load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        let core = Core::new();
        assert!(get_frame(&core, &session, 0, "not-a-real-mode", None).is_err());
    }
}
