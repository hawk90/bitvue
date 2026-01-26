//! Thumbnail commands
//!
//! Commands for generating frame thumbnails.

use serde::{Deserialize, Serialize};
use base64::Engine;

use crate::commands::AppState;
use crate::services::create_svg_thumbnail;
use crate::commands::frame::{decode_ivf_frame, decode_container_frame};
use bitvue_core::StreamId;
use bitvue_formats::{detect_container_format, ContainerFormat};
use image::{ImageBuffer, RgbImage, DynamicImage};

/// Thumbnail data for a single frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailData {
    pub frame_index: usize,
    pub thumbnail: String,  // SVG data URL
    pub width: u32,
    pub height: u32,
}

/// Get thumbnails for specified frames
#[tauri::command]
pub async fn get_thumbnails(
    state: tauri::State<'_, AppState>,
    frame_indices: Vec<usize>,
) -> Result<Vec<ThumbnailData>, String> {
    log::info!("get_thumbnails: Requesting {} thumbnails", frame_indices.len());

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    let file_path = stream_a.file_path.as_ref().ok_or("No file loaded")?.clone();
    let units_ref = stream_a.units.as_ref().ok_or("No units available")?;

    // Clone the units we need to release the lock
    let unit_count = units_ref.units.len();
    let units: Vec<_> = units_ref.units.iter().filter_map(|u| {
        u.frame_index.map(|idx| (idx, u.frame_type.clone()))
    }).collect();

    drop(stream_a);
    drop(core);

    // Detect container format
    let container_format = detect_container_format(&file_path)
        .unwrap_or(ContainerFormat::Unknown);

    // Read file data for thumbnail generation
    let file_data = std::fs::read(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut thumbnails = Vec::new();

    for &frame_idx in &frame_indices {
        // Check cache first
        if let Some(cached) = state.thumbnail_service.lock()
            .map_err(|e| e.to_string())?
            .get_cached(frame_idx)
        {
            thumbnails.push(ThumbnailData {
                frame_index: frame_idx,
                thumbnail: cached,
                width: 80,
                height: 45,
            });
            continue;
        }

        // Generate thumbnail if not cached
        if frame_idx < unit_count {
            let frame_type = units.get(frame_idx)
                .and_then(|(_, ft)| ft.as_deref())
                .unwrap_or("UNKNOWN");

            // Try to generate real thumbnail from decoded frame
            let thumbnail_data = match generate_real_thumbnail(&file_data, frame_idx, container_format) {
                Ok(data) => data,
                Err(_) => {
                    // Fall back to SVG placeholder if decoding fails
                    log::warn!("get_thumbnails: Failed to decode frame {}, using SVG placeholder", frame_idx);
                    create_svg_thumbnail(frame_type)
                }
            };

            // Cache the thumbnail
            state.thumbnail_service.lock()
                .map_err(|e| e.to_string())?
                .cache_thumbnail(frame_idx, thumbnail_data.clone(), frame_type.to_string());

            thumbnails.push(ThumbnailData {
                frame_index: frame_idx,
                thumbnail: thumbnail_data,
                width: 80,
                height: 45,
            });
        }
    }

    log::info!("get_thumbnails: Generated {} thumbnails", thumbnails.len());
    Ok(thumbnails)
}

/// Generate a real thumbnail by decoding the frame and resizing
fn generate_real_thumbnail(
    file_data: &[u8],
    frame_index: usize,
    container_format: ContainerFormat,
) -> Result<String, String> {
    const THUMBNAIL_WIDTH: u32 = 160;
    const THUMBNAIL_HEIGHT: u32 = 90;

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
