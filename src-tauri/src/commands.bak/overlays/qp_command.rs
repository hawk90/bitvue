//! QP Heatmap Tauri command (Monster Pack v14 T3-1)

use super::super::AppState;
use super::qp_types::{QPHeatmapRequest, QPHeatmapResponse};
use bitvue_core::{
    qp_heatmap::{QPHeatmapOverlay, HeatmapResolution, QPScaleMode, QPCacheKeyParams},
    StreamId,
};

/// Get QP Heatmap texture data for a frame (T3-1 compliant)
///
/// Returns RGBA8 texture data with proper 4-stop color ramp:
/// - 0.00 → (  0,  70, 255) - Blue
/// - 0.35 → (  0, 200, 180) - Cyan
/// - 0.65 → (255, 190,   0) - Yellow
/// - 1.00 → (255,  40,  40) - Red
pub async fn get_qp_heatmap_impl(
    request: QPHeatmapRequest,
    state: tauri::State<'_, AppState>,
) -> Result<QPHeatmapResponse, String> {
    tracing::info!("get_qp_heatmap: Request for frame {} (resolution: {}, scale: {})",
        request.frame_index, request.resolution, request.scale_mode);

    // Parse resolution
    let resolution = match request.resolution.as_str() {
        "quarter" => HeatmapResolution::Quarter,
        "half" => HeatmapResolution::Half,
        "full" => HeatmapResolution::Full,
        _ => return Err(format!("Invalid resolution: {}", request.resolution)),
    };

    // Parse scale mode
    let scale_mode = match request.scale_mode.as_str() {
        "auto" => QPScaleMode::Auto,
        "fixed" => QPScaleMode::Fixed,
        _ => return Err(format!("Invalid scale_mode: {}", request.scale_mode)),
    };

    // Get decode service
    let decode_service = state.decode_service.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    // Get file data for hashing
    let file_path = decode_service.get_file_path()
        .ok_or_else(|| "No file loaded".to_string())?;

    // Get frame data to extract QP values
    let file_data = decode_service.file_data.as_ref()
        .ok_or_else(|| "No file data available".to_string())?;

    // Extract QP grid from AV1 bitstream (now uses real parser!)
    let qp_grid = bitvue_av1::extract_qp_grid(
        file_data,
        request.frame_index,
        20, // Default base QP
    ).map_err(|e| format!("Failed to extract QP grid: {}", e))?;

    // Check coverage
    if qp_grid.coverage_percent() < 20.0 {
        return Ok(QPHeatmapResponse {
            frame_index: request.frame_index,
            width: 0,
            height: 0,
            pixels_base64: String::new(),
            qp_min: qp_grid.qp_min,
            qp_max: qp_grid.qp_max,
            coverage_percent: qp_grid.coverage_percent(),
            cache_key: String::new(),
            success: false,
            error: Some(format!("Insufficient QP coverage ({:.1}%, min 20% required)", qp_grid.coverage_percent())),
        });
    }

    // Create heatmap overlay
    let mut overlay = QPHeatmapOverlay::new(qp_grid);
    overlay.set_resolution(resolution);
    overlay.set_scale_mode(scale_mode);
    overlay.set_opacity(request.opacity);

    // Generate cache key
    let cache_params = QPCacheKeyParams {
        stream: StreamId::A,
        frame_idx: request.frame_index,
        resolution,
        scale_mode,
        qp_min: overlay.grid.qp_min,
        qp_max: overlay.grid.qp_max,
        opacity: request.opacity,
        codec: "AV1",
        file_path: &file_path,
    };

    let cache_key_obj = bitvue_core::qp_heatmap::QPHeatmapCacheKey::new(&cache_params);

    // Get texture
    let texture = overlay.get_texture(cache_key_obj.clone());

    // Generate cache key string
    let cache_key = cache_key_obj.to_string(texture.width, texture.height);

    // Encode pixels as base64
    use base64::prelude::*;
    let pixels_base64 = BASE64_STANDARD.encode(&texture.pixels);

    tracing::info!("get_qp_heatmap: Returning {}x{} texture ({} bytes)",
        texture.width, texture.height, texture.pixels.len());

    Ok(QPHeatmapResponse {
        frame_index: request.frame_index,
        width: texture.width,
        height: texture.height,
        pixels_base64,
        qp_min: overlay.grid.qp_min,
        qp_max: overlay.grid.qp_max,
        coverage_percent: overlay.grid.coverage_percent(),
        cache_key,
        success: true,
        error: None,
    })
}
