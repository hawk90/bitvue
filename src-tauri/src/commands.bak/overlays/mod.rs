//! Overlay-related Tauri commands (Monster Pack v14 compliant)

pub mod qp_types;
pub mod qp_command;
pub mod mv_types;
pub mod mv_command;

// Tauri commands - redeclare to fix module path issues
#[tauri::command]
pub async fn get_qp_heatmap(
    request: qp_types::QPHeatmapRequest,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<qp_types::QPHeatmapResponse, String> {
    qp_command::get_qp_heatmap_impl(request, state).await
}

#[tauri::command]
pub async fn get_mv_overlay(
    request: mv_types::MVOverlayRequest,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<mv_types::MVOverlayResponse, String> {
    mv_command::get_mv_overlay_impl(request, state).await
}
