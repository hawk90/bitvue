//! Debug YUV Commands — VQ Analyzer YUVDiff parity
//!
//! Enables frame-level comparison between the decoded bitstream and an
//! externally provided raw YUV reference file (e.g., from an encoder's
//! internal reconstruction output).
//!
//! Workflow:
//!   1. `load_debug_yuv`       — parse YUV file params, store state
//!   2. `get_debug_yuv_frame`  — return decoded / reference / diff frame
//!   3. `get_yuv_diff_metrics` — compute PSNR / SSIM for a frame pair
//!   4. `find_first_diff_frame`— scan all frames, return first mismatch
//!   5. `unload_debug_yuv`     — release the loaded reference

use crate::commands::{AppState, YUVFrameData};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use tauri::State;

// ─── State ────────────────────────────────────────────────────────────────────

/// In-memory state for the loaded reference YUV file.
/// Stored inside `AppState.debug_yuv`.
#[derive(Debug, Clone)]
pub struct DebugYuvState {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub format: YuvFormat,
    pub bitdepth: u8,
    /// Total number of complete frames in the file.
    pub frame_count: usize,
    /// Bytes per frame (depends on format + bitdepth).
    pub frame_size: usize,
    /// Frame offset: add this to the display frame index to get the reference
    /// frame index.  Allows compensating for encoder delay.
    pub picture_offset: i32,
    /// Crop values (applied to the decoded frame before comparison).
    pub crop: CropValues,
}

/// YUV storage format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YuvFormat {
    /// YUV 4:2:0 planar (I420)
    I420,
    /// YUV 4:2:0 semi-planar (NV12, Y then interleaved UV)
    Nv12,
    /// YUV 4:2:0 semi-planar (NV21, Y then interleaved VU)
    Nv21,
    /// YUV 4:2:2 planar
    I422,
    /// YUV 4:4:4 planar
    I444,
}

impl YuvFormat {
    /// Bytes per pixel × 1000 (to avoid floats).
    fn bpp_millis(&self) -> usize {
        match self {
            YuvFormat::I420 | YuvFormat::Nv12 | YuvFormat::Nv21 => 1500, // 1.5 bpp
            YuvFormat::I422 => 2000,                                       // 2.0 bpp
            YuvFormat::I444 => 3000,                                       // 3.0 bpp
        }
    }

    /// Frame size in bytes for a given resolution and bit depth.
    pub fn frame_size(&self, width: u32, height: u32, bitdepth: u8) -> usize {
        let bytes_per_sample = if bitdepth > 8 { 2usize } else { 1usize };
        let luma_samples = (width as usize) * (height as usize);
        let total_samples = (luma_samples * self.bpp_millis()) / 1000;
        total_samples * bytes_per_sample
    }
}

/// Crop values in pixels (applied to all four sides of the decoded frame).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CropValues {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

// ─── Request / Response types ─────────────────────────────────────────────────

/// Parameters for loading a debug YUV file.
#[derive(Debug, Deserialize)]
pub struct LoadDebugYuvParams {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: YuvFormat,
    pub bitdepth: u8,
    pub picture_offset: Option<i32>,
    pub crop: Option<CropValues>,
}

/// Response returned by `load_debug_yuv`.
#[derive(Debug, Serialize)]
pub struct LoadDebugYuvResult {
    pub success: bool,
    pub frame_count: usize,
    pub frame_size: usize,
    pub error: Option<String>,
}

/// Display mode for `get_debug_yuv_frame`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum YuvDiffMode {
    /// Show the bitstream-decoded frame (default).
    Decoded,
    /// Show the reference YUV frame.
    Reference,
    /// Show |decoded − reference| (abs difference).
    Diff,
    /// Show |decoded − reference| × amplification factor.
    Amplified,
}

/// Request for `get_debug_yuv_frame`.
#[derive(Debug, Deserialize)]
pub struct GetDebugYuvFrameParams {
    pub frame_index: usize,
    pub mode: YuvDiffMode,
    /// Amplification factor applied when `mode == Amplified`.
    pub amplify: Option<u8>,
}

/// PSNR / SSIM metrics for a single frame pair.
#[derive(Debug, Serialize)]
pub struct YuvDiffMetrics {
    pub frame_index: usize,
    pub psnr_y: f64,
    pub psnr_u: f64,
    pub psnr_v: f64,
    pub psnr_avg: f64,
    pub ssim_y: f64,
    pub max_diff_y: u8,
    pub has_mismatch: bool,
}

/// Result of `find_first_diff_frame`.
#[derive(Debug, Serialize)]
pub struct FirstDiffResult {
    /// First frame index where decoded != reference. None if all frames match.
    pub frame_index: Option<usize>,
    pub total_checked: usize,
}

// ─── Helper: read one raw YUV frame from file ─────────────────────────────────

fn read_yuv_frame(
    path: &PathBuf,
    frame_index: usize,
    frame_size: usize,
    picture_offset: i32,
) -> Result<Vec<u8>, String> {
    let ref_index = (frame_index as i64 + picture_offset as i64).max(0) as usize;
    let offset = (ref_index * frame_size) as u64;

    let mut file = File::open(path).map_err(|e| format!("Cannot open YUV file: {e}"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("Seek error: {e}"))?;

    let mut buf = vec![0u8; frame_size];
    file.read_exact(&mut buf)
        .map_err(|e| format!("Read error: {e}"))?;
    Ok(buf)
}

// ─── Diff computation ─────────────────────────────────────────────────────────

fn compute_diff(decoded: &[u8], reference: &[u8], amplify: u8) -> Vec<u8> {
    decoded
        .iter()
        .zip(reference.iter())
        .map(|(&a, &b)| {
            let diff = (a as i16 - b as i16).unsigned_abs() as u8;
            diff.saturating_mul(amplify)
        })
        .collect()
}

fn psnr(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mse: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let diff = (x as f64) - (y as f64);
            diff * diff
        })
        .sum::<f64>()
        / a.len() as f64;

    if mse < 1e-10 {
        return 100.0; // identical
    }
    10.0 * (255.0_f64 * 255.0 / mse).log10()
}

/// Simplified SSIM (luminance + structure, no Gaussian weighting).
fn ssim_simple(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let n = a.len() as f64;
    let mu_a: f64 = a.iter().map(|&x| x as f64).sum::<f64>() / n;
    let mu_b: f64 = b.iter().map(|&x| x as f64).sum::<f64>() / n;
    let var_a: f64 = a.iter().map(|&x| (x as f64 - mu_a).powi(2)).sum::<f64>() / n;
    let var_b: f64 = b.iter().map(|&x| (x as f64 - mu_b).powi(2)).sum::<f64>() / n;
    let cov: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as f64 - mu_a) * (y as f64 - mu_b))
        .sum::<f64>()
        / n;

    let c1 = (0.01 * 255.0_f64).powi(2);
    let c2 = (0.03 * 255.0_f64).powi(2);

    ((2.0 * mu_a * mu_b + c1) * (2.0 * cov + c2))
        / ((mu_a.powi(2) + mu_b.powi(2) + c1) * (var_a + var_b + c2))
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

/// Load a raw YUV reference file and store its metadata in AppState.
///
/// The file is NOT fully read into memory; frames are read on demand.
/// Width/height/format must be supplied by the caller (raw YUV has no header).
#[tauri::command]
pub async fn load_debug_yuv(
    state: State<'_, AppState>,
    params: LoadDebugYuvParams,
) -> Result<LoadDebugYuvResult, String> {
    let path = PathBuf::from(&params.path);

    // Validate path (no directory traversal)
    if !path.exists() {
        return Ok(LoadDebugYuvResult {
            success: false,
            frame_count: 0,
            frame_size: 0,
            error: Some(format!("File not found: {}", params.path)),
        });
    }

    let frame_size = params.format.frame_size(params.width, params.height, params.bitdepth);
    if frame_size == 0 {
        return Ok(LoadDebugYuvResult {
            success: false,
            frame_count: 0,
            frame_size: 0,
            error: Some("Invalid format/resolution: frame size is 0".into()),
        });
    }

    let file_len = std::fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .len() as usize;

    let frame_count = file_len / frame_size;

    let yuv_state = DebugYuvState {
        path,
        width: params.width,
        height: params.height,
        format: params.format,
        bitdepth: params.bitdepth,
        frame_count,
        frame_size,
        picture_offset: params.picture_offset.unwrap_or(0),
        crop: params.crop.unwrap_or_default(),
    };

    let mut debug_yuv = state.debug_yuv.lock().map_err(|e| e.to_string())?;
    *debug_yuv = Some(yuv_state);

    log::info!(
        "load_debug_yuv: loaded {} frames ({} bytes/frame) from {:?}",
        frame_count,
        frame_size,
        params.path
    );

    Ok(LoadDebugYuvResult {
        success: true,
        frame_count,
        frame_size,
        error: None,
    })
}

/// Get a YUV frame in the requested display mode (decoded / reference / diff / amplified).
///
/// Returns the same `YUVFrameData` structure used by `get_decoded_frame_yuv` so that
/// the frontend can reuse the existing `VideoCanvas` rendering pipeline.
#[tauri::command]
pub async fn get_debug_yuv_frame(
    state: State<'_, AppState>,
    params: GetDebugYuvFrameParams,
) -> Result<YUVFrameData, String> {
    // Block-scope the guard so it is dropped before any `.await` points.
    let (frame_size, w, h, fmt, bd, offset, path) = {
        let guard = state.debug_yuv.lock().map_err(|e| e.to_string())?;
        let s = guard.as_ref().ok_or("No debug YUV loaded. Call load_debug_yuv first.")?;
        (s.frame_size, s.width, s.height, s.format, s.bitdepth, s.picture_offset, s.path.clone())
        // guard is dropped here
    };

    let luma_size = (w as usize) * (h as usize) * (if bd > 8 { 2 } else { 1 });
    let chroma_size = frame_size - luma_size;
    let chroma_u_size = chroma_size / 2;

    let encode = |data: &[u8]| base64::engine::general_purpose::STANDARD.encode(data);

    match params.mode {
        YuvDiffMode::Reference => {
            let raw = read_yuv_frame(&path, params.frame_index, frame_size, offset)?;
            Ok(make_yuv_frame_data(
                params.frame_index, w, h, bd,
                &raw[..luma_size],
                &raw[luma_size..luma_size + chroma_u_size],
                &raw[luma_size + chroma_u_size..],
                fmt,
            ))
        }

        YuvDiffMode::Diff | YuvDiffMode::Amplified => {
            // Get decoded frame from the backend decoder
            let decoded_raw = get_decoded_yuv_raw(&state, params.frame_index, luma_size, chroma_u_size).await?;
            let ref_raw = read_yuv_frame(&path, params.frame_index, frame_size, offset)?;

            let amplify = if params.mode == YuvDiffMode::Amplified {
                params.amplify.unwrap_or(8)
            } else {
                1
            };

            let diff_y = compute_diff(&decoded_raw[..luma_size], &ref_raw[..luma_size], amplify);
            let diff_u = compute_diff(
                &decoded_raw[luma_size..luma_size + chroma_u_size],
                &ref_raw[luma_size..luma_size + chroma_u_size],
                amplify,
            );
            let diff_v = compute_diff(
                &decoded_raw[luma_size + chroma_u_size..],
                &ref_raw[luma_size + chroma_u_size..],
                amplify,
            );

            Ok(YUVFrameData {
                frame_index: params.frame_index,
                width: w,
                height: h,
                bit_depth: 8, // diff is always 8-bit
                y_plane: encode(&diff_y),
                u_plane: Some(encode(&diff_u)),
                v_plane: Some(encode(&diff_v)),
                y_stride: w as usize,
                u_stride: (w as usize) / 2,
                v_stride: (w as usize) / 2,
                success: true,
                error: None,
            })
        }

        YuvDiffMode::Decoded => {
            // Just return the normal decoded frame — frontend can call get_decoded_frame_yuv
            // directly for this case; we support it here for completeness.
            let decoded_raw = get_decoded_yuv_raw(&state, params.frame_index, luma_size, chroma_u_size).await?;
            Ok(make_yuv_frame_data(
                params.frame_index, w, h, bd,
                &decoded_raw[..luma_size],
                &decoded_raw[luma_size..luma_size + chroma_u_size],
                &decoded_raw[luma_size + chroma_u_size..],
                fmt,
            ))
        }
    }
}

/// Compute PSNR / SSIM metrics for a specific decoded-vs-reference frame pair.
#[tauri::command]
pub async fn get_yuv_diff_metrics(
    state: State<'_, AppState>,
    frame_index: usize,
) -> Result<YuvDiffMetrics, String> {
    let (frame_size, w, h, bd, offset, path) = {
        let guard = state.debug_yuv.lock().map_err(|e| e.to_string())?;
        let s = guard.as_ref().ok_or("No debug YUV loaded.")?;
        (s.frame_size, s.width, s.height, s.bitdepth, s.picture_offset, s.path.clone())
    };

    let luma_size = (w as usize) * (h as usize) * (if bd > 8 { 2 } else { 1 });
    let chroma_u_size = (frame_size - luma_size) / 2;

    let ref_raw = read_yuv_frame(&path, frame_index, frame_size, offset)?;
    let dec_raw = get_decoded_yuv_raw(&state, frame_index, luma_size, chroma_u_size).await?;

    let ref_y = &ref_raw[..luma_size];
    let ref_u = &ref_raw[luma_size..luma_size + chroma_u_size];
    let ref_v = &ref_raw[luma_size + chroma_u_size..];
    let dec_y = &dec_raw[..luma_size];
    let dec_u = &dec_raw[luma_size..luma_size + chroma_u_size];
    let dec_v = &dec_raw[luma_size + chroma_u_size..];

    let psnr_y = psnr(dec_y, ref_y);
    let psnr_u = psnr(dec_u, ref_u);
    let psnr_v = psnr(dec_v, ref_v);
    let psnr_avg = (6.0 * psnr_y + psnr_u + psnr_v) / 8.0;

    let ssim_y = ssim_simple(dec_y, ref_y);
    let max_diff_y = dec_y
        .iter()
        .zip(ref_y.iter())
        .map(|(&a, &b)| (a as i16 - b as i16).unsigned_abs() as u8)
        .max()
        .unwrap_or(0);

    Ok(YuvDiffMetrics {
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

/// Scan all frames and return the index of the first one where decoded ≠ reference.
#[tauri::command]
pub async fn find_first_diff_frame(
    state: State<'_, AppState>,
) -> Result<FirstDiffResult, String> {
    let (frame_count, frame_size, w, h, bd, offset, path) = {
        let guard = state.debug_yuv.lock().map_err(|e| e.to_string())?;
        let s = guard.as_ref().ok_or("No debug YUV loaded.")?;
        (s.frame_count, s.frame_size, s.width, s.height, s.bitdepth, s.picture_offset, s.path.clone())
    };

    let luma_size = (w as usize) * (h as usize) * (if bd > 8 { 2 } else { 1 });
    let chroma_u_size = (frame_size - luma_size) / 2;

    // Obtain the number of decoded frames from core
    let total_decoded = {
        let core = state.core.lock().map_err(|e| e.to_string())?;
        let stream = core.get_stream(bitvue_core::StreamId::A);
        let s = stream.read();
        s.units.as_ref().map(|u| u.units.len()).unwrap_or(0)
    };

    let frames_to_check = frame_count.min(total_decoded);

    for idx in 0..frames_to_check {
        let ref_raw = match read_yuv_frame(&path, idx, frame_size, offset) {
            Ok(r) => r,
            Err(_) => break,
        };
        let dec_raw = match get_decoded_yuv_raw(&state, idx, luma_size, chroma_u_size).await {
            Ok(r) => r,
            Err(_) => break,
        };

        // Compare luma only for speed — if Y matches, U/V likely match too
        let ref_y = &ref_raw[..luma_size];
        let dec_y = &dec_raw[..luma_size];
        if ref_y != dec_y {
            return Ok(FirstDiffResult {
                frame_index: Some(idx),
                total_checked: idx + 1,
            });
        }
    }

    Ok(FirstDiffResult {
        frame_index: None,
        total_checked: frames_to_check,
    })
}

/// Auto-detect YUV format from file size and resolution.
///
/// Tries common (width, height, format, bitdepth) combinations and returns
/// the best match based on exact file-size divisibility.
#[tauri::command]
pub fn detect_yuv_format(
    path: String,
    width: u32,
    height: u32,
) -> Result<serde_json::Value, String> {
    let file_size = std::fs::metadata(&path)
        .map_err(|e| e.to_string())?
        .len() as usize;

    let candidates: &[(YuvFormat, u8, &str)] = &[
        (YuvFormat::I420, 8, "i420 8-bit"),
        (YuvFormat::Nv12, 8, "nv12 8-bit"),
        (YuvFormat::I422, 8, "i422 8-bit"),
        (YuvFormat::I444, 8, "i444 8-bit"),
        (YuvFormat::I420, 10, "i420 10-bit"),
        (YuvFormat::I422, 10, "i422 10-bit"),
        (YuvFormat::I444, 10, "i444 10-bit"),
        (YuvFormat::I420, 12, "i420 12-bit"),
        (YuvFormat::I420, 16, "i420 16-bit"),
    ];

    let mut results = Vec::new();
    for &(fmt, bd, label) in candidates {
        let fs = fmt.frame_size(width, height, bd);
        if fs > 0 && file_size % fs == 0 {
            results.push(serde_json::json!({
                "format": format!("{:?}", fmt).to_lowercase(),
                "bitdepth": bd,
                "frame_count": file_size / fs,
                "label": label,
            }));
        }
    }

    Ok(serde_json::json!({ "matches": results }))
}

/// Release the loaded debug YUV state.
#[tauri::command]
pub fn unload_debug_yuv(state: State<'_, AppState>) -> Result<(), String> {
    let mut debug_yuv = state.debug_yuv.lock().map_err(|e| e.to_string())?;
    *debug_yuv = None;
    log::info!("unload_debug_yuv: debug YUV unloaded");
    Ok(())
}

/// Update picture offset for the loaded debug YUV.
#[tauri::command]
pub fn set_debug_yuv_offset(
    state: State<'_, AppState>,
    offset: i32,
) -> Result<(), String> {
    let mut debug_yuv = state.debug_yuv.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut s) = *debug_yuv {
        s.picture_offset = offset;
        Ok(())
    } else {
        Err("No debug YUV loaded.".into())
    }
}

/// Update crop values for the loaded debug YUV.
#[tauri::command]
pub fn set_debug_yuv_crop(
    state: State<'_, AppState>,
    crop: CropValues,
) -> Result<(), String> {
    let mut debug_yuv = state.debug_yuv.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut s) = *debug_yuv {
        s.crop = crop;
        Ok(())
    } else {
        Err("No debug YUV loaded.".into())
    }
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Build a `YUVFrameData` from separate Y/U/V slices.
fn make_yuv_frame_data(
    frame_index: usize,
    w: u32,
    h: u32,
    bitdepth: u8,
    y: &[u8],
    u: &[u8],
    v: &[u8],
    _fmt: YuvFormat,
) -> YUVFrameData {
    let encode = |data: &[u8]| base64::engine::general_purpose::STANDARD.encode(data);
    let stride_div = 2usize; // 4:2:0

    YUVFrameData {
        frame_index,
        width: w,
        height: h,
        bit_depth: bitdepth,
        y_plane: encode(y),
        u_plane: Some(encode(u)),
        v_plane: Some(encode(v)),
        y_stride: w as usize,
        u_stride: w as usize / stride_div,
        v_stride: w as usize / stride_div,
        success: true,
        error: None,
    }
}

/// Get decoded YUV frame from the decoder as a flat byte vector [Y…U…V…].
async fn get_decoded_yuv_raw(
    state: &State<'_, AppState>,
    frame_index: usize,
    luma_size: usize,
    chroma_u_size: usize,
) -> Result<Vec<u8>, String> {
    let yuv = crate::commands::frame::decode_frame_yuv_internal(state.inner(), frame_index, false).await
        .map_err(|e| e.to_string())?;

    if !yuv.success {
        return Err(yuv.error.unwrap_or_else(|| "Decoder error".into()));
    }

    let decode_b64 = |s: &str| -> Result<Vec<u8>, String> {
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(|e| e.to_string())
    };

    let y = decode_b64(&yuv.y_plane)?;
    let u = yuv
        .u_plane
        .as_deref()
        .map(decode_b64)
        .transpose()?
        .unwrap_or_else(|| vec![128u8; chroma_u_size]);
    let v = yuv
        .v_plane
        .as_deref()
        .map(decode_b64)
        .transpose()?
        .unwrap_or_else(|| vec![128u8; chroma_u_size]);

    // Truncate or pad to expected sizes
    let mut raw = Vec::with_capacity(luma_size + chroma_u_size * 2);
    raw.extend_from_slice(&y[..luma_size.min(y.len())]);
    if y.len() < luma_size {
        raw.resize(luma_size, 0);
    }
    raw.extend_from_slice(&u[..chroma_u_size.min(u.len())]);
    if u.len() < chroma_u_size {
        raw.resize(luma_size + chroma_u_size, 128);
    }
    raw.extend_from_slice(&v[..chroma_u_size.min(v.len())]);
    if v.len() < chroma_u_size {
        raw.resize(luma_size + chroma_u_size * 2, 128);
    }

    Ok(raw)
}
