//! Comparison algorithms: produce a display frame in one of four modes (`get_frame`), compute
//! PSNR/SSIM metrics (`compute_frame_metrics`), or scan for the first pixel mismatch
//! (`find_first_diff`) against a loaded reference session.

use super::planes::{decoded_to_planes8, planes_to_wire, read_reference_frame, Planes8};
use super::Session;
use crate::decode_bridge::{self, DecodedYuvFrame};
use bitvue_engine::{Core, StreamId};
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
        descriptor: crate::decode_bridge::FrameDescriptor {
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
        },
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

    metrics_for_planes(frame_index, &ref_planes, &decoded_planes)
}

/// PSNR/SSIM/max-diff of one decoded frame against one reference frame, both already 8-bit planes
/// of the same size (the callers check that). Shared by the single-frame and whole-stream paths.
fn metrics_for_planes(
    frame_index: usize,
    ref_planes: &Planes8,
    decoded_planes: &Planes8,
) -> Result<DiffMetrics, String> {
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

/// Decodes stream A once, in decode order, calling `visit(index, frame)` for every decoded frame
/// until it returns `true` (stop) or the stream runs out. One streaming pass, unlike
/// `get_frame`/`compute_frame_metrics` (one `get_decoded_frame_yuv`-style redecode-from-scratch
/// per call) -- same reasoning as `decode_bridge::get_thumbnails` batching one decode pass across
/// many requested indices. `cancel_flag` is checked once per encoded packet and once per decoded
/// frame: each is a real decode/compare, so per-frame is the right granularity for a loop that
/// can walk the whole stream.
fn for_each_decoded_frame(
    core: &Core,
    cancel_flag: &AtomicBool,
    mut visit: impl FnMut(usize, &bitvue_decode::DecodedFrame) -> bool,
) -> Result<(), FindFirstDiffError> {
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

    for f in &frames {
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
            if visit(index, &decoded) {
                return Ok(());
            }
        }
    }
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
        if visit(index, decoded) {
            break;
        }
    }
    Ok(())
}

/// The reference frame for decoded frame `index` (`index + session.picture_offset`), or `None`
/// once the reference file has no frame there (negative offset, past the end, unreadable).
fn reference_for(session: &Session, index: usize) -> Option<Planes8> {
    let ref_index_signed = index as i64 + session.picture_offset;
    if ref_index_signed < 0 || ref_index_signed as usize >= session.frame_count {
        return None;
    }
    read_reference_frame(session, ref_index_signed as usize).ok()
}

/// Scans stream A in decode order, comparing each decoded frame's Y plane against the
/// corresponding reference frame (`decoded_index + picture_offset`), stopping at the first pixel
/// mismatch. Returns `(first_mismatched_frame_index, frames_actually_compared)` -- comparison
/// stops early (without an error) once the reference file runs out of frames to compare against.
pub fn find_first_diff(
    core: &Core,
    session: &Session,
    cancel_flag: &AtomicBool,
) -> Result<(Option<usize>, usize), FindFirstDiffError> {
    let mut checked = 0usize;
    let mut found: Option<usize> = None;
    for_each_decoded_frame(core, cancel_flag, |index, decoded| {
        let Some(ref_planes) = reference_for(session, index) else {
            return true; // out of reference frames -- stop
        };
        let decoded_planes = decoded_to_planes8(&decode_bridge::to_wire(decoded), session.crop);
        checked += 1;
        if decoded_planes.width == ref_planes.width
            && decoded_planes.height == ref_planes.height
            && decoded_planes
                .y
                .iter()
                .zip(ref_planes.y.iter())
                .any(|(a, b)| a != b)
        {
            found = Some(index);
            return true;
        }
        false
    })?;
    Ok((found, checked))
}

/// PSNR/SSIM of every decoded frame of stream A against the reference, in one decode pass.
/// Stops without an error when the reference runs out of frames (the result then holds fewer
/// entries than the stream has frames). A frame whose size differs from the reference's is an
/// error, same as the single-frame path -- it would otherwise silently produce no number.
pub fn compute_all_metrics(
    core: &Core,
    session: &Session,
    cancel_flag: &AtomicBool,
) -> Result<Vec<DiffMetrics>, FindFirstDiffError> {
    let mut all = Vec::new();
    let mut failure: Option<String> = None;
    for_each_decoded_frame(core, cancel_flag, |index, decoded| {
        let Some(ref_planes) = reference_for(session, index) else {
            return true;
        };
        let decoded_planes = decoded_to_planes8(&decode_bridge::to_wire(decoded), session.crop);
        if decoded_planes.width != ref_planes.width || decoded_planes.height != ref_planes.height {
            failure = Some(format!(
                "decoded frame {index} is {}x{} but reference frame (after crop) is {}x{} -- \
                 load a matching-resolution reference or adjust crop",
                decoded_planes.width, decoded_planes.height, ref_planes.width, ref_planes.height
            ));
            return true;
        }
        match metrics_for_planes(index, &ref_planes, &decoded_planes) {
            Ok(m) => {
                all.push(m);
                false
            }
            Err(message) => {
                failure = Some(message);
                true
            }
        }
    })?;
    match failure {
        Some(message) => Err(FindFirstDiffError::Other(message)),
        None => Ok(all),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_yuv::{LoadParams, YuvFormat};
    use crate::test_support::write_i420_fixture;

    #[test]
    fn get_frame_reference_mode_does_not_need_stream_a_open() {
        let file = write_i420_fixture(4, 4, &[42]);
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
        let core = Core::new(); // no stream opened at all
        let frame = get_frame(&core, &session, 0, "reference", None).unwrap();
        assert_eq!(frame.descriptor.width, 4);
        assert_eq!(frame.descriptor.height, 4);
        assert!(frame.bytes[..frame.descriptor.y_len]
            .iter()
            .all(|&b| b == 42));
    }

    #[test]
    fn get_frame_decoded_mode_without_open_stream_is_a_real_error() {
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
        let core = Core::new();
        assert!(get_frame(&core, &session, 0, "decoded", None).is_err());
    }

    #[test]
    fn get_frame_unknown_mode_is_a_real_error() {
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
        let core = Core::new();
        assert!(get_frame(&core, &session, 0, "not-a-real-mode", None).is_err());
    }
}
