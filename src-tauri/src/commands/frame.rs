//! Frame data commands
//!
//! Commands for getting decoded frames, frame analysis, and hex data.

use serde::{Deserialize, Serialize};
use base64::Engine;

use crate::commands::{AppState, validate_frame_index_bounds};
use bitvue_core::StreamId;
use bitvue_formats::{detect_container_format, ContainerFormat};
use image::{ImageBuffer, RgbImage};

/// Decoded frame data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedFrameData {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub frame_data: String,  // Base64 encoded PNG (full resolution)
    pub success: bool,
    pub error: Option<String>,
}

/// YUV frame data for direct rendering
/// More efficient than RGB conversion - decoder outputs YUV natively
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YUVFrameData {
    pub frame_index: usize,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    /// Y plane data (base64 encoded)
    pub y_plane: String,
    /// U plane data (base64 encoded, None for monochrome)
    pub u_plane: Option<String>,
    /// V plane data (base64 encoded, None for monochrome)
    pub v_plane: Option<String>,
    /// Y stride (bytes per row)
    pub y_stride: usize,
    /// U stride (bytes per row)
    pub u_stride: usize,
    /// V stride (bytes per row)
    pub v_stride: usize,
    pub success: bool,
    pub error: Option<String>,
}

/// Get full-resolution decoded frame
#[tauri::command]
pub async fn get_decoded_frame(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<DecodedFrameData, String> {
    log::info!("get_decoded_frame: Requesting frame {}", frame_index);

    // Rate limiting check (frame decoding is CPU-intensive)
    state.rate_limiter.check_rate_limit()
        .map_err(|wait_time| {
            format!("Rate limited: too many frame decode requests. Please try again in {:.1}s",
                wait_time.as_secs_f64())
        })?;

    // SECURITY: Validate frame index early at command boundary (defense in depth)
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();
    let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();
    let total_frames = stream_a.units.as_ref().map(|u| u.units.len()).unwrap_or(0);
    drop(stream_a);
    drop(core);

    // Early validation to avoid unnecessary work for out-of-range indices
    if let Err(e) = validate_frame_index_bounds(frame_index, total_frames) {
        log::warn!("get_decoded_frame: {}", e);
        return Ok(DecodedFrameData {
            frame_index,
            width: 0,
            height: 0,
            frame_data: String::new(),
            success: false,
            error: Some(e),
        });
    }

    // Detect container format
    let container_format = detect_container_format(&file_path)
        .unwrap_or(ContainerFormat::Unknown);

    log::info!("get_decoded_frame: Container format: {:?}", container_format);

    // Get decode_service early to use in closure
    let decode_service = state.decode_service.lock().map_err(|e| e.to_string())?;

    // Use decode_service cache to avoid repeated decoding
    let decode_fn = |file_data: &[u8], idx: usize| -> Result<(u32, u32, Vec<u8>), String> {
        match container_format {
            ContainerFormat::IVF => decode_ivf_frame(file_data, idx),
            ContainerFormat::MP4 | ContainerFormat::Matroska => {
                // Try to use cached samples for MP4/MKV (performance optimization)
                let cached_samples = match container_format {
                    ContainerFormat::MP4 => decode_service.get_or_extract_mp4_samples()?,
                    ContainerFormat::Matroska => decode_service.get_or_extract_mkv_samples()?,
                    _ => None,
                };

                // Use cached samples if available, otherwise extract on demand
                decode_container_frame_with_samples(
                    file_data,
                    idx,
                    container_format,
                    cached_samples.as_ref().map(|s| s.as_slice())
                )
            }
            _ => decode_ivf_frame(file_data, idx),
        }
    };

    let decode_result = decode_service.get_or_decode_frame(frame_index, decode_fn);
    drop(decode_service);

    match decode_result {
        Ok((width, height, rgb_data)) => {
            match create_png_base64(&rgb_data, width, height) {
                Ok(png_base64) => {
                    log::info!("get_decoded_frame: Successfully decoded frame {} ({}x{})", frame_index, width, height);
                    Ok(DecodedFrameData {
                        frame_index,
                        width,
                        height,
                        frame_data: png_base64,
                        success: true,
                        error: None,
                    })
                }
                Err(e) => Ok(DecodedFrameData {
                    frame_index,
                    width,
                    height,
                    frame_data: String::new(),
                    success: false,
                    error: Some(format!("Failed to encode PNG: {}", e)),
                }),
            }
        }
        Err(e) => Ok(DecodedFrameData {
            frame_index,
            width: 0,
            height: 0,
            frame_data: String::new(),
            success: false,
            error: Some(e),
        }),
    }
}

/// Decode multiple frames from IVF file (batch decoding for thumbnails)
pub fn decode_ivf_frames_batch(file_data: &[u8], frame_indices: &[usize]) -> Result<Vec<(usize, (u32, u32, Vec<u8>))>, String> {
    // Parse IVF using shared helper
    let frames = parse_ivf(file_data)?;

    // Find the maximum frame index needed
    let max_idx = *frame_indices.iter().max().unwrap_or(&0);

    // SECURITY: Validate number of frame indices to prevent DoS
    const MAX_BATCH_SIZE: usize = 1000;
    if frame_indices.len() > MAX_BATCH_SIZE {
        return Err(format!(
            "Too many frame indices requested: {} (maximum {})",
            frame_indices.len(),
            MAX_BATCH_SIZE
        ));
    }

    if max_idx >= frames.len() {
        // SECURITY: Don't reveal total frame count
        return Err("Frame index out of range".to_string());
    }

    // Create a single decoder for all frames (more efficient than creating new decoder per frame)
    let mut decoder = bitvue_decode::Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let mut results = Vec::new();
    let mut indices_iter = frame_indices.iter().copied().collect::<Vec<_>>();
    indices_iter.sort(); // Sort to process in order
    indices_iter.dedup(); // Remove duplicates

    let mut next_needed_idx = 0;

    // Decode frames sequentially up to the max index
    // AV1 may need reference frames, so we decode in order
    for idx in 0..=max_idx {
        let frame_data = &frames[idx].data;

        decoder.send_data(frame_data, frames[idx].timestamp as i64)
            .map_err(|e| format!("Failed to send frame data for index {}: {}", idx, e))?;

        match decoder.get_frame() {
            Ok(frame) => {
                // Check if this frame was requested
                if next_needed_idx < indices_iter.len() && indices_iter[next_needed_idx] == idx {
                    let rgb_data = bitvue_decode::yuv_to_rgb(&frame);
                    results.push((idx, (frame.width, frame.height, rgb_data)));
                    next_needed_idx += 1;
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                // EAGAIN is expected when decoder needs more data
                if !err_str.contains("EAGAIN") && !err_str.contains("Try again") {
                    return Err(format!("Failed to decode frame {}: {}", idx, e));
                }
            }
        }
    }

    Ok(results)
}

/// Decode frame from IVF file (single frame decoding for performance)
pub fn decode_ivf_frame(file_data: &[u8], frame_index: usize) -> Result<(u32, u32, Vec<u8>), String> {
    // Parse IVF and validate frame index
    let frames = parse_ivf_and_validate(file_data, frame_index)?;

    // Decode only the requested frame
    let frame_data = &frames[frame_index].data;

    // Create decoder and send only this frame's data
    let mut decoder = bitvue_decode::Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    // Send the frame data
    decoder.send_data(frame_data, frames[frame_index].timestamp as i64)
        .map_err(|e| format!("Failed to send frame data: {}", e))?;

    // Get the decoded frame
    let frame = decoder.get_frame()
        .map_err(|e| format!("Failed to decode frame: {}", e))?;

    let width = frame.width;
    let height = frame.height;
    let rgb_data = bitvue_decode::yuv_to_rgb(&frame);

    Ok((width, height, rgb_data))
}

/// Parse IVF file and validate frame index (shared helper)
fn parse_ivf_and_validate(file_data: &[u8], frame_index: usize) -> Result<Vec<bitvue_av1::IvfFrame>, String> {
    let frames = parse_ivf(file_data)?;
    validate_frame_index_bounds(frame_index, frames.len())?;
    Ok(frames)
}

/// Parse IVF file (shared helper without validation)
fn parse_ivf(file_data: &[u8]) -> Result<Vec<bitvue_av1::IvfFrame>, String> {
    // Check if AV1 file
    if file_data.len() < 4 || &file_data[0..4] != b"DKIF" {
        return Err("Not an AV1 IVF file".to_string());
    }

    // Parse IVF to get frame data without decoding all frames
    let (_header, frames) = bitvue_av1::parse_ivf_frames(file_data)
        .map_err(|e| format!("Failed to parse IVF: {}", e))?;

    Ok(frames)
}

/// Decode frame from MP4/MKV container
///
/// Extracts AV1 samples from the container and decodes the specified frame.
/// Uses raw OBU decoding directly without IVF wrapper for better performance.
pub fn decode_container_frame(
    file_data: &[u8],
    frame_index: usize,
    container_format: ContainerFormat,
) -> Result<(u32, u32, Vec<u8>), String> {
    decode_container_frame_with_samples(file_data, frame_index, container_format, None)
}

/// Decode a container frame with optional pre-extracted samples (for performance optimization)
///
/// # Arguments
/// * `file_data` - Raw file data (used if samples is None)
/// * `frame_index` - Index of frame to decode
/// * `container_format` - Container format (MP4 or Matroska)
/// * `samples` - Optional cached samples to avoid re-extraction
///
/// # Performance
/// This function decodes AV1 OBUs directly without creating an IVF wrapper.
/// The Av1Decoder's `send_data` method accepts raw OBU sequences, which is
/// significantly faster than wrapping in IVF and then parsing the wrapper.
pub fn decode_container_frame_with_samples(
    file_data: &[u8],
    frame_index: usize,
    container_format: ContainerFormat,
    samples: Option<&[Vec<u8>]>,
) -> Result<(u32, u32, Vec<u8>), String> {
    // Extract AV1 samples from container (use cached samples if provided)
    let extracted_samples: Vec<Vec<u8>> = if let Some(cached_samples) = samples {
        log::debug!("decode_container_frame_with_samples: Using {} cached samples", cached_samples.len());
        cached_samples.to_vec()
    } else {
        match container_format {
            ContainerFormat::MP4 => {
                bitvue_formats::mp4::extract_av1_samples(file_data)
                    .map_err(|e| format!("Failed to extract AV1 from MP4: {}", e))?
                    .into_iter().map(|cow| cow.to_vec()).collect()
            }
            ContainerFormat::Matroska => {
                bitvue_formats::mkv::extract_av1_samples(file_data)
                    .map_err(|e| format!("Failed to extract AV1 from MKV: {}", e))?
            }
            _ => return Err("Unsupported container format".to_string()),
        }
    };

    validate_frame_index_bounds(frame_index, extracted_samples.len())?;

    let sample_data = &extracted_samples[frame_index];

    // Decode the sample directly as raw OBU data
    // Av1Decoder.send_data accepts raw OBU sequences - no IVF wrapper needed
    let mut decoder = bitvue_decode::Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    // Send the raw OBU sample to the decoder
    decoder.send_data(sample_data, frame_index as i64)
        .map_err(|e| format!("Failed to send data to decoder: {}", e))?;

    // Get the decoded frame
    let decoded_frame = decoder.get_frame()
        .map_err(|e| format!("Failed to decode frame: {}", e))?;

    let width = decoded_frame.width;
    let height = decoded_frame.height;
    let rgb_data = bitvue_decode::yuv_to_rgb(&decoded_frame);

    log::info!("decode_container_frame: Decoded frame {} from container ({}x{})",
        frame_index, width, height);
    Ok((width, height, rgb_data))
}

/// Raw frame hex data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameHexData {
    pub frame_index: usize,
    pub data: Vec<u8>,       // Raw OBU/frame bytes
    pub size: usize,         // Total size in bytes
    pub truncated: bool,     // Whether data was truncated for display
    pub success: bool,
    pub error: Option<String>,
}

/// Get raw frame hex data for hex view display
#[tauri::command]
pub async fn get_frame_hex_data(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
    max_bytes: Option<usize>,
) -> Result<FrameHexData, String> {
    log::info!("get_frame_hex_data: Requesting hex data for frame {}, max_bytes: {:?}",
        frame_index, max_bytes);

    // Get file path
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();
    let _file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();
    let total_frames = stream_a.units.as_ref().map(|u| u.units.len()).unwrap_or(0);
    drop(stream_a);
    drop(core);

    // Check frame index
    if frame_index >= total_frames {
        return Ok(FrameHexData {
            frame_index,
            data: Vec::new(),
            size: 0,
            truncated: false,
            success: false,
            error: Some(format!("Frame index {} out of range (total: {})", frame_index, total_frames)),
        });
    }

    // Use cached file data from decode_service to avoid repeated disk reads
    let file_data = state.decode_service.lock()
        .map_err(|e| e.to_string())?
        .get_file_data_arc()?;

    // Check if AV1 IVF file
    if file_data.len() < 4 || &file_data[0..4] != b"DKIF" {
        return Ok(FrameHexData {
            frame_index,
            data: Vec::new(),
            size: 0,
            truncated: false,
            success: false,
            error: Some("Not an AV1 IVF file".to_string()),
        });
    }

    // Parse IVF to get frame data
    let frames = match bitvue_av1::parse_ivf_frames(&file_data) {
        Ok((_, ivf_frames)) => ivf_frames,
        Err(e) => {
            return Ok(FrameHexData {
                frame_index,
                data: Vec::new(),
                size: 0,
                truncated: false,
                success: false,
                error: Some(format!("Failed to parse IVF: {}", e)),
            });
        }
    };

    if validate_frame_index_bounds(frame_index, frames.len()).is_err() {
        return Ok(FrameHexData {
            frame_index,
            data: Vec::new(),
            size: 0,
            truncated: false,
            success: false,
            error: Some(format!("Frame {} not found", frame_index)),
        });
    }

    let frame_data = frames[frame_index].data.clone();
    let total_size = frame_data.len();

    // Limit bytes for display (default 2048 bytes)
    // SECURITY: Validate max_bytes to prevent DoS through extremely large values
    const MAX_HEX_BYTES: usize = 1_048_576; // 1MB max for hex display
    let limit = max_bytes.unwrap_or(2048).min(MAX_HEX_BYTES);
    let (return_data, truncated) = if frame_data.len() > limit {
        (frame_data[..limit].to_vec(), true)
    } else {
        (frame_data, false)
    };

    // SECURITY: Don't log byte counts, frame index, or sizes to prevent information disclosure
    log::info!("get_frame_hex_data: Hex data returned successfully");

    Ok(FrameHexData {
        frame_index,
        data: return_data,
        size: total_size,
        truncated,
        success: true,
        error: None,
    })
}

/// Create PNG base64 (without data: URL prefix)
pub fn create_png_base64(rgb_data: &[u8], width: u32, height: u32) -> Result<String, String> {
    // Validate inputs
    if width == 0 || height == 0 {
        return Err(format!("Invalid dimensions: {}x{}", width, height));
    }

    if rgb_data.is_empty() {
        return Err("RGB data is empty".to_string());
    }

    // Validate RGB data size matches dimensions (3 bytes per pixel)
    // SECURITY: Use checked arithmetic to prevent integer overflow
    let expected_size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(3))
        .ok_or_else(|| "Invalid dimensions: would cause integer overflow".to_string())?;

    if rgb_data.len() != expected_size {
        return Err("RGB data size mismatch".to_string());
    }

    // Check for reasonable dimensions (prevent DoS with extremely large images)
    const MAX_DIMENSION: u32 = 16384; // 16K max
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(format!(
            "Dimensions too large (max {}x{}, got {}x{})",
            MAX_DIMENSION, MAX_DIMENSION, width, height
        ));
    }

    // Also limit total size to prevent memory issues
    const MAX_IMAGE_SIZE: usize = 100 * 1024 * 1024; // 100MB max
    if expected_size > MAX_IMAGE_SIZE {
        return Err(format!(
            "Image size too large: {} bytes (max {})",
            expected_size, MAX_IMAGE_SIZE
        ));
    }

    let img: RgbImage = ImageBuffer::from_raw(width, height, rgb_data.to_vec())
        .ok_or("Failed to create image buffer from raw data")?;

    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

/// Decode YUV frame from IVF file (single frame decoding for performance)
pub fn decode_ivf_frame_yuv(file_data: &[u8], frame_index: usize) -> Result<bitvue_decode::DecodedFrame, String> {
    // Check if AV1 file
    if file_data.len() < 4 || &file_data[0..4] != b"DKIF" {
        return Err("Not an AV1 IVF file".to_string());
    }

    // Parse IVF to get frame data without decoding all frames
    let (_header, frames) = bitvue_av1::parse_ivf_frames(file_data)
        .map_err(|e| format!("Failed to parse IVF: {}", e))?;

    validate_frame_index_bounds(frame_index, frames.len())?;

    // Decode only the requested frame
    let frame_data = &frames[frame_index].data;

    // Create decoder and send only this frame's data
    let mut decoder = bitvue_decode::Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    // Send the frame data
    decoder.send_data(frame_data, frames[frame_index].timestamp as i64)
        .map_err(|e| format!("Failed to send frame data: {}", e))?;

    // Get the decoded frame
    decoder.get_frame()
        .map_err(|e| format!("Failed to decode frame: {}", e))
}

/// Decode YUV frame from MP4/MKV container
#[allow(dead_code)]
pub fn decode_container_frame_yuv(
    file_data: &[u8],
    frame_index: usize,
    container_format: ContainerFormat,
) -> Result<bitvue_decode::DecodedFrame, String> {
    decode_container_frame_yuv_with_samples(file_data, frame_index, container_format, None)
}

/// Decode YUV frame from MP4/MKV with optional pre-extracted samples
///
/// Decodes AV1 samples from MP4/MKV containers and returns YUV frame data
/// without unnecessary IVF wrapper overhead.
pub fn decode_container_frame_yuv_with_samples(
    file_data: &[u8],
    frame_index: usize,
    container_format: ContainerFormat,
    samples: Option<&[Vec<u8>]>,
) -> Result<bitvue_decode::DecodedFrame, String> {
    // Extract AV1 samples from container (use cached samples if provided)
    let extracted_samples: Vec<Vec<u8>> = if let Some(cached_samples) = samples {
        log::debug!("decode_container_frame_yuv_with_samples: Using {} cached samples", cached_samples.len());
        cached_samples.to_vec()
    } else {
        match container_format {
            ContainerFormat::MP4 => {
                bitvue_formats::mp4::extract_av1_samples(file_data)
                    .map_err(|e| format!("Failed to extract AV1 from MP4: {}", e))?
                    .into_iter().map(|cow| cow.to_vec()).collect()
            }
            ContainerFormat::Matroska => {
                bitvue_formats::mkv::extract_av1_samples(file_data)
                    .map_err(|e| format!("Failed to extract AV1 from MKV: {}", e))?
            }
            _ => return Err("Unsupported container format".to_string()),
        }
    };

    validate_frame_index_bounds(frame_index, extracted_samples.len())?;

    // Decode directly as raw OBU data - no IVF wrapper needed
    let sample_data = &extracted_samples[frame_index];
    let mut decoder = bitvue_decode::Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    decoder.send_data(sample_data, frame_index as i64)
        .map_err(|e| format!("Failed to send data to decoder: {}", e))?;

    let decoded_frame = decoder.get_frame()
        .map_err(|e| format!("Failed to decode frame: {}", e))?;

    Ok(decoded_frame)
}

/// Get decoded YUV frame (more efficient than RGB conversion)
#[tauri::command]
pub async fn get_decoded_frame_yuv(
    state: tauri::State<'_, AppState>,
    frame_index: usize,
) -> Result<YUVFrameData, String> {
    log::info!("get_decoded_frame_yuv: Requesting YUV frame {}", frame_index);

    // SECURITY: Validate frame index early at command boundary (defense in depth)
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();
    let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();
    let total_frames = stream_a.units.as_ref().map(|u| u.units.len()).unwrap_or(0);
    drop(stream_a);
    drop(core);

    // Early validation to avoid unnecessary work for out-of-range indices
    if let Err(e) = validate_frame_index_bounds(frame_index, total_frames) {
        log::warn!("get_decoded_frame_yuv: {}", e);
        return Ok(YUVFrameData {
            frame_index,
            width: 0,
            height: 0,
            bit_depth: 8,
            y_plane: String::new(),
            u_plane: None,
            v_plane: None,
            y_stride: 0,
            u_stride: 0,
            v_stride: 0,
            success: false,
            error: Some(e),
        });
    }

    // Detect container format
    let container_format = detect_container_format(&file_path)
        .unwrap_or(ContainerFormat::Unknown);

    log::info!("get_decoded_frame_yuv: Container format: {:?}", container_format);

    // Get decode_service early to use in closure
    let decode_service = state.decode_service.lock().map_err(|e| e.to_string())?;

    // Use decode_service cache to avoid repeated YUV frame decoding
    let decode_fn = |file_data: &[u8], idx: usize| -> Result<bitvue_decode::DecodedFrame, String> {
        match container_format {
            ContainerFormat::IVF => decode_ivf_frame_yuv(file_data, idx),
            ContainerFormat::MP4 | ContainerFormat::Matroska => {
                // Try to use cached samples for MP4/MKV (performance optimization)
                let cached_samples = match container_format {
                    ContainerFormat::MP4 => decode_service.get_or_extract_mp4_samples()?,
                    ContainerFormat::Matroska => decode_service.get_or_extract_mkv_samples()?,
                    _ => None,
                };

                // Use cached samples if available, otherwise extract on demand
                decode_container_frame_yuv_with_samples(
                    file_data,
                    idx,
                    container_format,
                    cached_samples.as_ref().map(|s| s.as_slice())
                )
            }
            _ => decode_ivf_frame_yuv(file_data, idx),
        }
    };

    let decode_result = decode_service.get_or_decode_frame_yuv(frame_index, decode_fn);
    drop(decode_service);

    match decode_result {
        Ok(frame) => {
            // Validate frame data before encoding
            if frame.y_plane.is_empty() {
                log::warn!("get_decoded_frame_yuv: Frame {} has empty Y plane", frame_index);
                return Ok(YUVFrameData {
                    frame_index,
                    width: frame.width,
                    height: frame.height,
                    bit_depth: frame.bit_depth,
                    y_plane: String::new(),
                    u_plane: None,
                    v_plane: None,
                    y_stride: frame.y_stride,
                    u_stride: frame.u_stride,
                    v_stride: frame.v_stride,
                    success: false,
                    error: Some("Frame data is empty (Y plane)".to_string()),
                });
            }

            // Validate dimensions are reasonable
            if frame.width == 0 || frame.height == 0 {
                log::warn!("get_decoded_frame_yuv: Frame {} has invalid dimensions: {}x{}",
                    frame_index, frame.width, frame.height);
                return Ok(YUVFrameData {
                    frame_index,
                    width: frame.width,
                    height: frame.height,
                    bit_depth: frame.bit_depth,
                    y_plane: String::new(),
                    u_plane: None,
                    v_plane: None,
                    y_stride: frame.y_stride,
                    u_stride: frame.u_stride,
                    v_stride: frame.v_stride,
                    success: false,
                    error: Some(format!("Invalid frame dimensions: {}x{}", frame.width, frame.height)),
                });
            }

            // Validate strides are reasonable
            if frame.y_stride == 0 || frame.y_stride < frame.width as usize {
                log::warn!("get_decoded_frame_yuv: Frame {} has invalid Y stride: {}",
                    frame_index, frame.y_stride);
                return Ok(YUVFrameData {
                    frame_index,
                    width: frame.width,
                    height: frame.height,
                    bit_depth: frame.bit_depth,
                    y_plane: String::new(),
                    u_plane: None,
                    v_plane: None,
                    y_stride: frame.y_stride,
                    u_stride: frame.u_stride,
                    v_stride: frame.v_stride,
                    success: false,
                    error: Some(format!("Invalid Y stride: {}", frame.y_stride)),
                });
            }

            // Encode YUV planes as base64
            use base64::Engine;

            let y_plane = base64::engine::general_purpose::STANDARD.encode(&*frame.y_plane);
            let u_plane = frame.u_plane.as_ref().map(|p| base64::engine::general_purpose::STANDARD.encode(&**p));
            let v_plane = frame.v_plane.as_ref().map(|p| base64::engine::general_purpose::STANDARD.encode(&**p));

            log::info!("get_decoded_frame_yuv: Successfully decoded YUV frame {} ({}x{}, {}bit)",
                frame_index, frame.width, frame.height, frame.bit_depth);

            Ok(YUVFrameData {
                frame_index,
                width: frame.width,
                height: frame.height,
                bit_depth: frame.bit_depth,
                y_plane,
                u_plane,
                v_plane,
                y_stride: frame.y_stride,
                u_stride: frame.u_stride,
                v_stride: frame.v_stride,
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(YUVFrameData {
            frame_index,
            width: 0,
            height: 0,
            bit_depth: 8,
            y_plane: String::new(),
            u_plane: None,
            v_plane: None,
            y_stride: 0,
            u_stride: 0,
            v_stride: 0,
            success: false,
            error: Some(e),
        }),
    }
}
