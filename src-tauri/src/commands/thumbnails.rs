//! Thumbnail commands
//!
//! Commands for generating frame thumbnails.

use serde::{Deserialize, Serialize};
use base64::Engine;

use crate::commands::AppState;
use crate::services::create_svg_thumbnail;
use crate::commands::frame::{decode_ivf_frame, decode_ivf_frames_batch, decode_container_frame};
use bitvue_core::StreamId;
use bitvue_formats::{detect_container_format, ContainerFormat};
use image::{ImageBuffer, RgbImage, DynamicImage};

/// Thumbnail data for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailData {
    pub frame_index: usize,
    pub thumbnail_data: String,  // Data URL (PNG or SVG)
    pub width: u32,
    pub height: u32,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Indicates whether this thumbnail was served from cache
    /// Useful for tracking cache hit rates and performance
    pub cached: bool,
}

/// Get thumbnails for specified frames
#[tauri::command]
pub async fn get_thumbnails(
    state: tauri::State<'_, AppState>,
    frame_indices: Vec<usize>,
) -> Result<Vec<ThumbnailData>, String> {
    log::info!("get_thumbnails: Requesting {} thumbnails", frame_indices.len());

    // SECURITY: Validate number of frame indices to prevent DoS
    const MAX_THUMBNAIL_REQUEST: usize = 500;
    if frame_indices.len() > MAX_THUMBNAIL_REQUEST {
        return Err(format!(
            "Too many thumbnail requests: {} (maximum {})",
            frame_indices.len(),
            MAX_THUMBNAIL_REQUEST
        ));
    }

    // Rate limiting check (thumbnail generation is CPU-intensive)
    state.rate_limiter.check_rate_limit()
        .map_err(|wait_time| {
            format!("Rate limited: too many requests. Please try again in {:.1}s",
                wait_time.as_secs_f64())
        })?;

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();
    let units_ref = stream_a.units.as_ref().ok_or("No units available")?;

    // Clone the units we need to release the lock
    let unit_count = units_ref.units.len();

    // Create a HashMap for frame_index -> frame_type lookup
    // This fixes the bug where units.get(frame_idx) was using array index instead of frame_index
    use std::collections::HashMap;
    let units_map: HashMap<usize, String> = units_ref.units.iter()
        .filter_map(|u| {
            // Both frame_index and frame_type need to be Some
            match (&u.frame_index, &u.frame_type) {
                (Some(idx), Some(ft)) => Some((*idx, ft.to_string())),
                _ => None,
            }
        })
        .collect();

    drop(stream_a);
    drop(core);

    // Detect container format
    let container_format = detect_container_format(&file_path)
        .unwrap_or(ContainerFormat::Unknown);

    // Use cached file data from decode_service to avoid repeated disk reads
    // Use Arc to avoid cloning the entire file data
    let file_data = state.decode_service.lock()
        .map_err(|e| e.to_string())?
        .get_file_data_arc()?;

    let mut thumbnails = Vec::new();

    // First pass: Check cache and collect uncached frames
    let mut uncached_frames: Vec<(usize, String)> = Vec::new(); // (frame_index, frame_type)
    let mut uncached_indices: Vec<usize> = Vec::new();

    for &frame_idx in &frame_indices {
        // Check cache first
        if let Ok(Some(cached)) = state.thumbnail_service.lock()
            .map_err(|e| e.to_string())?
            .get_cached(frame_idx)
        {
            thumbnails.push(ThumbnailData {
                frame_index: frame_idx,
                thumbnail_data: cached,
                width: 160,
                height: 90,
                success: true,
                error: None,
                cached: true,  // Cache hit
            });
        } else if !units_map.contains_key(&frame_idx) {
            // Frame index out of bounds
            thumbnails.push(ThumbnailData {
                frame_index: frame_idx,
                thumbnail_data: String::new(),
                width: 0,
                height: 0,
                success: false,
                error: Some(format!("Frame index {} out of bounds (total units: {})", frame_idx, unit_count)),
                cached: false,
            });
        } else {
            // Frame exists but not cached - collect for batch processing
            let frame_type = units_map.get(&frame_idx)
                .map(|ft| ft.as_str())
                .unwrap_or("UNKNOWN");
            uncached_frames.push((frame_idx, frame_type.to_string()));
            uncached_indices.push(frame_idx);
        }
    }

    // Second pass: Batch decode uncached frames (for IVF)
    if !uncached_indices.is_empty() && matches!(container_format, ContainerFormat::IVF) {
        // SECURITY: Don't log frame count to prevent information disclosure
        log::info!("get_thumbnails: Batch decoding IVF frames");

        // Batch decode frames
        match decode_ivf_frames_batch(&file_data, &uncached_indices) {
            Ok(decoded_frames) => {
                for (frame_idx, (width, height, rgb_data)) in decoded_frames {
                    // Generate thumbnail from decoded frame
                    match create_thumbnail_from_rgb(width, height, &rgb_data) {
                        Ok(thumbnail_data) => {
                            // Find the frame_type for this frame
                            let frame_type = uncached_frames.iter()
                                .find(|(idx, _)| *idx == frame_idx)
                                .map(|(_, ft)| ft.clone())
                                .unwrap_or("UNKNOWN".to_string());

                            // Cache the thumbnail
                            let _ = state.thumbnail_service.lock()
                                .map_err(|e| e.to_string())?
                                .cache_thumbnail(frame_idx, thumbnail_data.clone(), frame_type.clone())
                                .map_err(|e| {
                                    log::warn!("get_thumbnails: Failed to cache thumbnail for frame {}: {}", frame_idx, e);
                                });

                            thumbnails.push(ThumbnailData {
                                frame_index: frame_idx,
                                thumbnail_data,
                                width: 160,
                                height: 90,
                                success: true,
                                error: None,
                                cached: false,  // Freshly generated
                            });
                        }
                        Err(e) => {
                            // Fall back to SVG placeholder
                            log::warn!("get_thumbnails: Failed to create thumbnail for frame {} ({}), using SVG", frame_idx, e);
                            let frame_type = uncached_frames.iter()
                                .find(|(idx, _)| *idx == frame_idx)
                                .map(|(_, ft)| ft.clone())
                                .unwrap_or("UNKNOWN".to_string());

                            let svg_data = create_svg_thumbnail(&frame_type);

                            thumbnails.push(ThumbnailData {
                                frame_index: frame_idx,
                                thumbnail_data: svg_data,
                                width: 160,
                                height: 90,
                                success: true,
                                error: None,
                                cached: false,  // Freshly generated SVG
                            });
                        }
                    }
                }
            }
            Err(e) => {
                // Batch decode failed, fall back to individual processing with SVG placeholders
                log::warn!("get_thumbnails: Batch decode failed ({}), using SVG placeholders", e);
                for (frame_idx, frame_type) in uncached_frames {
                    let svg_data = create_svg_thumbnail(&frame_type);
                    thumbnails.push(ThumbnailData {
                        frame_index: frame_idx,
                        thumbnail_data: svg_data,
                        width: 160,
                        height: 90,
                        success: true,
                        error: None,
                        cached: false,
                    });
                }
            }
        }
    } else {
        // For non-IVF containers, process individually
        for (frame_idx, frame_type) in uncached_frames {
            let thumbnail_data = match generate_real_thumbnail(&file_data, frame_idx, container_format) {
                Ok(data) => data,
                Err(e) => {
                    // Fall back to SVG placeholder if decoding fails
                    log::warn!("get_thumbnails: Failed to decode frame {} ({}), using SVG placeholder", frame_idx, e);
                    create_svg_thumbnail(&frame_type)
                }
            };

            // Cache the thumbnail
            let _ = state.thumbnail_service.lock()
                .map_err(|e| e.to_string())?
                .cache_thumbnail(frame_idx, thumbnail_data.clone(), frame_type.clone())
                .map_err(|e| {
                    log::warn!("get_thumbnails: Failed to cache thumbnail for frame {}: {}", frame_idx, e);
                });

            thumbnails.push(ThumbnailData {
                frame_index: frame_idx,
                thumbnail_data,
                width: 160,
                height: 90,
                success: true,
                error: None,
                cached: false,  // Freshly generated
            });
        }
    }

    log::info!("get_thumbnails: Generated {} thumbnails", thumbnails.len());
    Ok(thumbnails)
}

/// Create thumbnail from decoded RGB data
fn create_thumbnail_from_rgb(width: u32, height: u32, rgb_data: &[u8]) -> Result<String, String> {
    use crate::constants::thumbnails;
    const THUMBNAIL_WIDTH: u32 = thumbnails::DEFAULT_WIDTH;
    const THUMBNAIL_HEIGHT: u32 = thumbnails::DEFAULT_HEIGHT;

    // Create image from RGB data (need owned data)
    let img: RgbImage = ImageBuffer::from_raw(width, height, rgb_data.to_vec())
        .ok_or("Failed to create image buffer")?;

    // Resize to thumbnail size
    let resized = image::imageops::resize(
        &DynamicImage::ImageRgb8(img),
        THUMBNAIL_WIDTH,
        THUMBNAIL_HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );

    // Encode as PNG
    let mut thumbnail_bytes = Vec::new();
    resized.write_to(&mut std::io::Cursor::new(&mut thumbnail_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode thumbnail: {}", e))?;

    // Return as data URL
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&thumbnail_bytes);
    Ok(format!("data:image/png;base64,{}", base64_data))
}

/// Generate a real thumbnail by decoding the frame and resizing
fn generate_real_thumbnail(
    file_data: &[u8],
    frame_index: usize,
    container_format: ContainerFormat,
) -> Result<String, String> {
    use crate::constants::thumbnails;
    const THUMBNAIL_WIDTH: u32 = thumbnails::DEFAULT_WIDTH;
    const THUMBNAIL_HEIGHT: u32 = thumbnails::DEFAULT_HEIGHT;

    // Decode the frame
    let (_width, _height, rgb_data) = match container_format {
        ContainerFormat::IVF => {
            decode_ivf_frame(file_data, frame_index)?
        }
        ContainerFormat::MP4 | ContainerFormat::Matroska => {
            decode_container_frame(file_data, frame_index, container_format)?
        }
        _ => {
            return Err("Unsupported container format for thumbnail generation".to_string());
        }
    };

    // Create image from RGB data
    let img: RgbImage = ImageBuffer::from_raw(_width, _height, rgb_data)
        .ok_or("Failed to create image buffer")?;

    // Resize to thumbnail size
    let resized = image::imageops::resize(
        &DynamicImage::ImageRgb8(img),
        THUMBNAIL_WIDTH,
        THUMBNAIL_HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );

    // Encode as WebP for smaller size (or PNG as fallback)
    let mut thumbnail_bytes = Vec::new();
    resized.write_to(&mut std::io::Cursor::new(&mut thumbnail_bytes), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode thumbnail: {}", e))?;

    // Return as data URL
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&thumbnail_bytes);
    Ok(format!("data:image/png;base64,{}", base64_data))
}
