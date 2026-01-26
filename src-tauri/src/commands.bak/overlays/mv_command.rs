//! Motion Vector overlay Tauri command (Monster Pack v14 T3-2)

use super::super::AppState;
use super::mv_types::{MVOverlayRequest, MVOverlayResponse, MVRenderData, MVStatsResponse};
use bitvue_core::mv_overlay::{MVOverlay, MVLayer, MVCacheKey};

/// Get Motion Vector overlay data for a frame (T3-2 compliant)
///
/// Implements stride sampling to cap visible vectors at 8000 max.
pub async fn get_mv_overlay_impl(
    request: MVOverlayRequest,
    state: tauri::State<'_, AppState>,
) -> Result<MVOverlayResponse, String> {
    tracing::info!("get_mv_overlay: Request for frame {} (viewport: {:?}, layer: {})",
        request.frame_index, request.viewport, request.layer);

    // Parse layer
    let layer = match request.layer.as_str() {
        "L0" => MVLayer::L0Only,
        "L1" => MVLayer::L1Only,
        "both" => MVLayer::Both,
        _ => return Err(format!("Invalid layer: {}", request.layer)),
    };

    let viewport = request.viewport.clone().into();

    // Get decode service
    let decode_service = state.decode_service.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let _file_path = decode_service.get_file_path()
        .ok_or_else(|| "No file loaded".to_string())?;

    let file_data = decode_service.file_data.as_ref()
        .ok_or_else(|| "No file data available".to_string())?;

    // Extract MV grid from AV1 bitstream (now uses real parser!)
    let mv_grid = bitvue_av1::extract_mv_grid(
        file_data,
        request.frame_index,
    ).map_err(|e| format!("Failed to extract MV grid: {}", e))?;

    // Create overlay
    let mut overlay = MVOverlay::new(mv_grid);
    overlay.set_layer(layer);
    overlay.set_user_scale(request.user_scale);
    overlay.set_opacity(request.opacity);

    // Update cache for viewport
    overlay.update_cache("A".to_string(), request.frame_index, viewport);

    // Get visible blocks
    let visible_blocks = overlay.get_visible_blocks(viewport);

    // Calculate stride and stats
    let stride = bitvue_core::mv_overlay::DensityControl::calculate_stride(overlay.grid.block_count());
    let stats = overlay.statistics();

    // Build render data
    let vectors: Vec<MVRenderData> = visible_blocks.iter().map(|(col, row)| {
        let (center_x, center_y) = overlay.grid.block_center(*col, *row);

        let mv_l0 = overlay.grid.get_l0(*col, *row);
        let mv_l1 = overlay.grid.get_l1(*col, *row);
        let mode = overlay.grid.get_mode(*col, *row);

        let is_intra = mode.map_or(false, |m| matches!(m, bitvue_core::mv_overlay::BlockMode::Intra));

        MVRenderData {
            block_x: *col * overlay.grid.block_w,
            block_y: *row * overlay.grid.block_h,
            center_x,
            center_y,
            mv_l0_dx: mv_l0.filter(|v| !v.is_missing()).map(|v| v.to_pixels().0),
            mv_l0_dy: mv_l0.filter(|v| !v.is_missing()).map(|v| v.to_pixels().1),
            mv_l1_dx: mv_l1.filter(|v| !v.is_missing()).map(|v| v.to_pixels().0),
            mv_l1_dy: mv_l1.filter(|v| !v.is_missing()).map(|v| v.to_pixels().1),
            is_intra,
        }
    }).collect();

    // Generate cache key
    let cache_key_obj = MVCacheKey::new(
        "A".to_string(),
        request.frame_index,
        viewport,
        stride,
        layer,
        request.user_scale,
        request.opacity,
    );

    tracing::info!("get_mv_overlay: Returning {} vectors (stride: {}, total: {})",
        vectors.len(), stride, overlay.grid.block_count());

    let drawn_blocks = vectors.len();

    Ok(MVOverlayResponse {
        frame_index: request.frame_index,
        vectors,
        stride,
        total_blocks: overlay.grid.block_count(),
        visible_blocks: overlay.grid.block_count(),
        drawn_blocks,
        statistics: MVStatsResponse {
            total_blocks: stats.total_blocks,
            l0_present: stats.l0_present,
            l1_present: stats.l1_present,
            l0_avg_magnitude: stats.l0_avg_magnitude,
            l1_avg_magnitude: stats.l1_avg_magnitude,
            l0_max_magnitude: stats.l0_max_magnitude,
            l1_max_magnitude: stats.l1_max_magnitude,
        },
        cache_key: cache_key_obj.to_string(),
        success: true,
        error: None,
    })
}
