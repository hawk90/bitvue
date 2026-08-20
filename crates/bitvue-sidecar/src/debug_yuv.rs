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
use std::sync::atomic::{AtomicBool, Ordering};

/// [`find_first_diff`]'s error type -- distinguishes a genuine failure from a cooperative
/// mid-decode cancellation, so the caller can report `WireErrorCode::Cancelled` instead of a
/// generic internal error. See `main.rs`'s "Concurrency model" doc: this scans the stream frame
/// by frame from the start until it finds a mismatch (or runs out of frames), so it can take real
/// time on a long stream.
#[derive(Debug)]
pub enum FindFirstDiffError {
    Cancelled,
    Other(String),
}

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
pub fn find_first_diff(
    core: &Core,
    session: &Session,
    cancel_flag: &AtomicBool,
) -> Result<(Option<usize>, usize), FindFirstDiffError> {
    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    let byte_cache = state
        .byte_cache
        .as_ref()
        .ok_or_else(|| FindFirstDiffError::Other("stream A not open".to_string()))?
        .clone();
    drop(state);
    let full_len = byte_cache.len() as usize;
    let data = byte_cache
        .read_range(0, full_len)
        .map_err(|e| FindFirstDiffError::Other(e.to_string()))?;

    let (_hdr, frames) = bitvue_av1_codec::ivf::parse_ivf_frames(data)
        .map_err(|e| FindFirstDiffError::Other(format!("IVF parse error: {e}")))?;
    let mut dec = bitvue_decode::Av1Decoder::new()
        .map_err(|e| FindFirstDiffError::Other(format!("decoder init: {e}")))?;
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
        // Checked once per encoded packet and once per decoded frame below -- each is a real
        // decode/compare, not a cheap instruction, so per-frame is the right granularity. This
        // loop can walk the *entire* stream (worst case: no mismatch exists), so it's one of the
        // handlers worth making cancellable mid-scan rather than only before it starts.
        if cancel_flag.load(Ordering::SeqCst) {
            return Err(FindFirstDiffError::Cancelled);
        }
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| FindFirstDiffError::Other(format!("decode send: {e}")))?;
        while let Ok(decoded) = dec.get_frame() {
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(FindFirstDiffError::Cancelled);
            }
            let index = decoded_count;
            decoded_count += 1;
            if check_one(&decoded, index, &mut checked, &mut found) {
                break 'outer;
            }
        }
    }
    if found.is_none() {
        if cancel_flag.load(Ordering::SeqCst) {
            return Err(FindFirstDiffError::Cancelled);
        }
        // Not dec.flush() -- see decode_bridge::get_decoded_frame_yuv's comment: flush() clears
        // dav1d's internal state instead of draining buffered frames.
        let mut remaining = Vec::new();
        dec.drain_decoder_frames(&mut remaining)
            .map_err(|e| FindFirstDiffError::Other(e.to_string()))?;
        for decoded in &remaining {
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(FindFirstDiffError::Cancelled);
            }
            let index = decoded_count;
            decoded_count += 1;
            if check_one(decoded, index, &mut checked, &mut found) {
                break;
            }
        }
    }

    Ok((found, checked))
}

use bitvue_protocol::{FrameKind, Request, Response, WireError, WireErrorCode};

/// Failure to load isn't a wire-level error -- same "domain outcome vs RPC outcome" split
/// `open_stream`'s module-doc note describes: the call succeeded, `result.success` carries whether
/// the load itself worked, matching the frontend's `YuvDiffContext.loadFile` expectations.
pub fn load_debug_yuv(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    let params: LoadParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    match load(params) {
        Ok(session) => {
            let frame_count = session.frame_count;
            let frame_size = session.frame_size;
            *state.lock().unwrap() = Some(session);
            Response::success(
                request.id,
                serde_json::json!({
                    "success": true,
                    "frame_count": frame_count,
                    "frame_size": frame_size,
                    "error": null,
                }),
            )
        }
        Err(message) => Response::success(
            request.id,
            serde_json::json!({
                "success": false,
                "frame_count": 0,
                "frame_size": 0,
                "error": message,
            }),
        ),
    }
}

pub fn unload_debug_yuv(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    *state.lock().unwrap() = None;
    Response::success(request.id, serde_json::json!({}))
}

pub fn no_debug_yuv_loaded(request_id: u32) -> Response {
    Response::failure(
        request_id,
        WireError {
            code: WireErrorCode::NotFound,
            message: "no debug YUV file loaded".to_string(),
            offset: None,
        },
    )
}

#[derive(serde::Deserialize)]
struct SetDebugYuvOffsetParams {
    offset: i64,
}

pub fn set_debug_yuv_offset(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    let params: SetDebugYuvOffsetParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    let mut guard = state.lock().unwrap();
    match guard.as_mut() {
        Some(session) => {
            session.picture_offset = params.offset;
            Response::success(request.id, serde_json::json!({}))
        }
        None => no_debug_yuv_loaded(request.id),
    }
}

#[derive(serde::Deserialize)]
struct SetDebugYuvCropParams {
    crop: Crop,
}

pub fn set_debug_yuv_crop(state: &crate::DebugYuvSlot, request: &Request) -> Response {
    let params: SetDebugYuvCropParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    let mut guard = state.lock().unwrap();
    match guard.as_mut() {
        Some(session) => {
            session.crop = params.crop;
            Response::success(request.id, serde_json::json!({}))
        }
        None => no_debug_yuv_loaded(request.id),
    }
}

#[derive(serde::Deserialize)]
struct DebugYuvFrameIndexParams {
    frame_index: usize,
}

pub fn get_yuv_diff_metrics(
    core: &Core,
    state: &crate::DebugYuvSlot,
    request: &Request,
) -> Response {
    let params: DebugYuvFrameIndexParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            )
        }
    };
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return no_debug_yuv_loaded(request.id),
    };
    match compute_frame_metrics(core, session, params.frame_index) {
        Ok(m) => Response::success(
            request.id,
            serde_json::json!({
                "frame_index": m.frame_index,
                "psnr_y": m.psnr_y,
                "psnr_u": m.psnr_u,
                "psnr_v": m.psnr_v,
                "psnr_avg": m.psnr_avg,
                "ssim_y": m.ssim_y,
                "max_diff_y": m.max_diff_y,
                "has_mismatch": m.has_mismatch,
            }),
        ),
        Err(message) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::FrameNotFound,
                message,
                offset: None,
            },
        ),
    }
}

pub fn find_first_diff_frame(
    core: &Core,
    state: &crate::DebugYuvSlot,
    request: &Request,
    cancel_flag: &AtomicBool,
) -> Response {
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => return no_debug_yuv_loaded(request.id),
    };
    match find_first_diff(core, session, cancel_flag) {
        Ok((frame_index, total_checked)) => Response::success(
            request.id,
            serde_json::json!({
                "frame_index": frame_index,
                "total_checked": total_checked,
            }),
        ),
        Err(FindFirstDiffError::Cancelled) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::Cancelled,
                message: "cancelled".to_string(),
                offset: None,
            },
        ),
        Err(FindFirstDiffError::Other(message)) => Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::Internal,
                message,
                offset: None,
            },
        ),
    }
}

#[derive(serde::Deserialize)]
struct GetDebugYuvFrameParams {
    frame_index: usize,
    mode: String,
    #[serde(default)]
    amplify: Option<u32>,
}

/// Data-plane command, same `Control` + `Data` two-frame pattern as `get_decoded_frame_yuv` --
/// see `get_frame` for how each of the four display modes is produced.
pub fn get_debug_yuv_frame(
    core: &Core,
    state: &crate::DebugYuvSlot,
    request: &Request,
) -> Vec<(FrameKind, Vec<u8>)> {
    let params: GetDebugYuvFrameParams = match serde_json::from_value(request.params.clone()) {
        Ok(p) => p,
        Err(err) => {
            return crate::command_support::single_control_frame(Response::failure(
                request.id,
                WireError {
                    code: WireErrorCode::InvalidData,
                    message: err.to_string(),
                    offset: None,
                },
            ))
        }
    };
    let guard = state.lock().unwrap();
    let session = match guard.as_ref() {
        Some(s) => s,
        None => {
            return crate::command_support::single_control_frame(no_debug_yuv_loaded(request.id))
        }
    };
    match get_frame(
        core,
        session,
        params.frame_index,
        &params.mode,
        params.amplify,
    ) {
        Ok(frame) => {
            let meta = Response::success(
                request.id,
                serde_json::json!({
                    "width": frame.width,
                    "height": frame.height,
                    "bit_depth": frame.bit_depth,
                    "chroma_subsampling": frame.chroma_subsampling,
                    "y_stride": frame.y_stride,
                    "u_stride": frame.u_stride,
                    "v_stride": frame.v_stride,
                    "y_len": frame.y_len,
                    "u_len": frame.u_len,
                    "v_len": frame.v_len,
                }),
            );
            vec![
                (
                    FrameKind::Control,
                    serde_json::to_vec(&meta).expect("Response always serializes"),
                ),
                (FrameKind::Data, frame.bytes),
            ]
        }
        Err(message) => crate::command_support::single_control_frame(Response::failure(
            request.id,
            WireError {
                code: WireErrorCode::FrameNotFound,
                message,
                offset: None,
            },
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{fresh_debug_yuv_state, open_real_fixture};
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

    fn write_i420_frame(
        width: u32,
        height: u32,
        y: &[u8],
        u: &[u8],
        v: &[u8],
    ) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        assert_eq!(y.len(), (width * height) as usize);
        file.write_all(y).unwrap();
        file.write_all(u).unwrap();
        file.write_all(v).unwrap();
        file
    }

    #[test]
    fn load_debug_yuv_end_to_end_reports_real_frame_count() {
        let file = write_i420_frame(4, 4, &[9u8; 16], &[9u8; 4], &[9u8; 4]);
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 300,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(),
                    "width": 4,
                    "height": 4,
                    "format": "i420",
                    "bitdepth": 8,
                }),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["frame_count"], 1);
        assert_eq!(result["frame_size"], 24);
        assert!(debug_yuv_state.lock().unwrap().is_some());
    }

    #[test]
    fn load_debug_yuv_bad_path_is_a_reported_failure_not_a_wire_error() {
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 301,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": "/nonexistent/does-not-exist.yuv",
                    "width": 4,
                    "height": 4,
                    "format": "i420",
                    "bitdepth": 8,
                }),
            },
        );
        // Same "domain outcome vs RPC outcome" split as open_stream -- the call itself succeeds,
        // result.success carries the real failure.
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["success"], false);
        assert!(result["error"].as_str().unwrap().contains("cannot open"));
        assert!(debug_yuv_state.lock().unwrap().is_none());
    }

    #[test]
    fn unload_debug_yuv_clears_the_session() {
        let file = write_i420_frame(4, 4, &[0u8; 16], &[0u8; 4], &[0u8; 4]);
        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 302,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": 4, "height": 4,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );
        assert!(debug_yuv_state.lock().unwrap().is_some());
        let response = unload_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 303,
                method: "unload_debug_yuv".to_string(),
                params: serde_json::json!({}),
            },
        );
        assert!(response.ok);
        assert!(debug_yuv_state.lock().unwrap().is_none());
    }

    #[test]
    fn set_debug_yuv_offset_and_crop_update_the_live_session() {
        let file = write_i420_frame(8, 8, &[0u8; 64], &[0u8; 16], &[0u8; 16]);
        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 304,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": 8, "height": 8,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let offset_response = set_debug_yuv_offset(
            &debug_yuv_state,
            &Request {
                id: 305,
                method: "set_debug_yuv_offset".to_string(),
                params: serde_json::json!({"offset": -3}),
            },
        );
        assert!(offset_response.ok);
        assert_eq!(
            debug_yuv_state
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .picture_offset,
            -3
        );

        let crop_response = set_debug_yuv_crop(
            &debug_yuv_state,
            &Request {
                id: 306,
                method: "set_debug_yuv_crop".to_string(),
                params: serde_json::json!({"crop": {"left": 2, "right": 0, "top": 0, "bottom": 0}}),
            },
        );
        assert!(crop_response.ok);
        assert_eq!(
            debug_yuv_state.lock().unwrap().as_ref().unwrap().crop.left,
            2
        );
    }

    #[test]
    fn set_debug_yuv_offset_without_a_loaded_session_is_not_found() {
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = set_debug_yuv_offset(
            &debug_yuv_state,
            &Request {
                id: 307,
                method: "set_debug_yuv_offset".to_string(),
                params: serde_json::json!({"offset": 1}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn get_debug_yuv_frame_reference_mode_end_to_end_returns_exact_bytes() {
        let y = (0u8..16).collect::<Vec<u8>>();
        let u = vec![50u8; 4];
        let v = vec![150u8; 4];
        let file = write_i420_frame(4, 4, &y, &u, &v);
        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 310,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": 4, "height": 4,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let core = Core::new(); // reference mode never touches stream A
        let frames = get_debug_yuv_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 311,
                method: "get_debug_yuv_frame".to_string(),
                params: serde_json::json!({"frame_index": 0, "mode": "reference"}),
            },
        );
        assert_eq!(frames.len(), 2, "expected Control + Data frames");
        let ctrl: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(ctrl.ok, "expected ok response, got {ctrl:?}");
        let meta = ctrl.result.unwrap();
        assert_eq!(meta["width"], 4);
        assert_eq!(meta["height"], 4);
        assert_eq!(meta["y_len"], 16);
        assert_eq!(frames[1].1[..16], y[..]);
        assert_eq!(frames[1].1[16..20], u[..]);
        assert_eq!(frames[1].1[20..24], v[..]);
    }

    #[test]
    fn get_debug_yuv_frame_without_a_loaded_session_is_not_found() {
        let core = Core::new();
        let debug_yuv_state = fresh_debug_yuv_state();
        let frames = get_debug_yuv_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 312,
                method: "get_debug_yuv_frame".to_string(),
                params: serde_json::json!({"frame_index": 0, "mode": "reference"}),
            },
        );
        assert_eq!(frames.len(), 1, "error path should not emit a Data frame");
        let response: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    /// End-to-end against the real AV1 fixture: builds a reference file from stream A's *actual*
    /// decoded frame 0 bytes, so "identical to what's already playing" is a real, independently
    /// reproducible fact rather than an assumption -- then confirms the diff/metrics path reports
    /// exactly that (no mismatch, nominally-infinite PSNR).
    fn load_reference_matching_real_fixture_frame_zero(
        core: &Core,
    ) -> (tempfile::NamedTempFile, u32, u32) {
        open_real_fixture(core, "A");
        let decode_request = Request {
            id: 320,
            method: "get_decoded_frame_yuv".to_string(),
            params: serde_json::json!({"stream": "A", "frame_index": 0}),
        };
        let decode_sessions = crate::decode_session::DecodeSessions::new();
        let frames = crate::commands::data_plane::get_decoded_frame_yuv(
            core,
            &decode_sessions,
            &decode_request,
            &std::sync::atomic::AtomicBool::new(false),
        );
        let ctrl: Response = serde_json::from_slice(&frames[0].1).unwrap();
        let meta = ctrl.result.unwrap();
        let width = meta["width"].as_u64().unwrap() as u32;
        let height = meta["height"].as_u64().unwrap() as u32;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&frames[1].1).unwrap();
        (file, width, height)
    }

    #[test]
    fn get_yuv_diff_metrics_identical_reference_reports_no_mismatch() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 321,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let response = get_yuv_diff_metrics(
            &core,
            &debug_yuv_state,
            &Request {
                id: 322,
                method: "get_yuv_diff_metrics".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(result["has_mismatch"], false);
        assert_eq!(result["max_diff_y"], 0);
        assert!(
            result["psnr_avg"].as_f64().unwrap() > 90.0,
            "identical frames should report a very high (~infinite) PSNR, got {result:?}"
        );
    }

    #[test]
    fn get_yuv_diff_metrics_without_a_loaded_session_is_not_found() {
        let core = Core::new();
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = get_yuv_diff_metrics(
            &core,
            &debug_yuv_state,
            &Request {
                id: 323,
                method: "get_yuv_diff_metrics".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }

    #[test]
    fn get_debug_yuv_frame_diff_mode_flags_a_real_injected_mismatch() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);

        // Corrupt one reference Y byte so decoded != reference at a known location.
        let mut bytes = std::fs::read(file.path()).unwrap();
        bytes[0] = bytes[0].wrapping_add(100);
        std::fs::write(file.path(), &bytes).unwrap();

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 330,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let metrics_response = get_yuv_diff_metrics(
            &core,
            &debug_yuv_state,
            &Request {
                id: 331,
                method: "get_yuv_diff_metrics".to_string(),
                params: serde_json::json!({"frame_index": 0}),
            },
        );
        let result = metrics_response.result.unwrap();
        assert_eq!(result["has_mismatch"], true);
        assert!(result["max_diff_y"].as_u64().unwrap() > 0);

        let frames = get_debug_yuv_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 332,
                method: "get_debug_yuv_frame".to_string(),
                params: serde_json::json!({"frame_index": 0, "mode": "diff"}),
            },
        );
        assert_eq!(frames.len(), 2);
        let ctrl: Response = serde_json::from_slice(&frames[0].1).unwrap();
        assert!(ctrl.ok, "expected ok response, got {ctrl:?}");
        assert!(
            frames[1].1.iter().any(|&b| b != 0),
            "diff mode should show a non-zero delta somewhere given the injected mismatch"
        );
    }

    #[test]
    fn find_first_diff_frame_end_to_end_locates_the_injected_mismatch() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);
        let mut bytes = std::fs::read(file.path()).unwrap();
        bytes[0] = bytes[0].wrapping_add(100);
        std::fs::write(file.path(), &bytes).unwrap();

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 340,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let response = find_first_diff_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 341,
                method: "find_first_diff_frame".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(false),
        );
        assert!(response.ok, "expected ok response, got {response:?}");
        let result = response.result.unwrap();
        assert_eq!(
            result["frame_index"], 0,
            "the only reference frame -- and the only one with an injected mismatch -- is index 0"
        );
        assert_eq!(result["total_checked"], 1);
    }

    /// Real cancellation-checkpoint regression test for UIX-ASYNC-006: proves
    /// `find_first_diff_frame` actually stops (and reports `Cancelled`, not a silent success)
    /// instead of scanning to completion when its flag is already set. Before this fix, the
    /// worker thread only ever checked the flag once *before* calling the handler at all
    /// (`spawn_request`'s "cancelled before execution started" branch); this test exercises the
    /// handler's own mid-execution checkpoint directly, which didn't exist at all previously --
    /// the same fixture/mismatch setup as the "locates the injected mismatch" test above would
    /// have returned `frame_index: 0` regardless of cancellation with the old code.
    #[test]
    fn find_first_diff_frame_stops_early_when_already_cancelled() {
        let core = Core::new();
        let (file, width, height) = load_reference_matching_real_fixture_frame_zero(&core);
        let mut bytes = std::fs::read(file.path()).unwrap();
        bytes[0] = bytes[0].wrapping_add(100);
        std::fs::write(file.path(), &bytes).unwrap();

        let debug_yuv_state = fresh_debug_yuv_state();
        load_debug_yuv(
            &debug_yuv_state,
            &Request {
                id: 350,
                method: "load_debug_yuv".to_string(),
                params: serde_json::json!({
                    "path": file.path().to_str().unwrap(), "width": width, "height": height,
                    "format": "i420", "bitdepth": 8,
                }),
            },
        );

        let response = find_first_diff_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 351,
                method: "find_first_diff_frame".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(true),
        );
        assert!(
            !response.ok,
            "expected a failure response, got {response:?}"
        );
        assert_eq!(response.error.unwrap().code, WireErrorCode::Cancelled);
    }

    #[test]
    fn find_first_diff_frame_without_a_loaded_session_is_not_found() {
        let core = Core::new();
        let debug_yuv_state = fresh_debug_yuv_state();
        let response = find_first_diff_frame(
            &core,
            &debug_yuv_state,
            &Request {
                id: 342,
                method: "find_first_diff_frame".to_string(),
                params: serde_json::json!({}),
            },
            &AtomicBool::new(false),
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, WireErrorCode::NotFound);
    }
}
