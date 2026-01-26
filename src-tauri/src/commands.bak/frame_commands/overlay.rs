//! Overlay data commands

use super::types::{OverlayDataResponse, BlockInfo};
use crate::commands::AppState;
use bitvue_core::StreamId;
use bitvue_decode::Av1Decoder;

/// Get overlay data for a frame (QP, MV, block info)
pub async fn get_overlay_data_impl(
    frame_index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<OverlayDataResponse, String> {
    tracing::info!("get_overlay_data: Request for frame {}", frame_index);

    // Get stream state
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    // Check if file is loaded
    if stream_a.units.is_none() {
        return Ok(OverlayDataResponse {
            frame_index,
            width: 0,
            height: 0,
            blocks: vec![],
            success: false,
            error: Some("No file loaded".to_string()),
        });
    }

    // Get decode service
    let decode_service = state.decode_service.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let file_data = decode_service.file_data.as_ref()
        .ok_or_else(|| "No file data available".to_string())?;

    let mut decoder = Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let decoded_frames = decoder.decode_all(file_data)
        .map_err(|e| format!("Decode failed: {}", e))?;

    let decoded = decoded_frames.get(frame_index)
        .ok_or_else(|| format!("Frame {} not found", frame_index))?;

    // Generate mock block data for visualization
    // In real implementation, this would be parsed from the bitstream
    let block_size = 64u32;
    let mut blocks = Vec::new();

    for y in (0..decoded.height).step_by(block_size as usize) {
        for x in (0..decoded.width).step_by(block_size as usize) {
            let bw = block_size.min(decoded.width - x);
            let bh = block_size.min(decoded.height - y);

            // Generate mock QP and MV data
            let qp = decoded.qp_avg.unwrap_or(20) + (x % 32) as u8;

            blocks.push(BlockInfo {
                x,
                y,
                width: bw,
                height: bh,
                qp,
                prediction_mode: if decoded.qp_avg.unwrap_or(0) < 50 { "INTER" } else { "INTRA" }.to_string(),
                has_mv: decoded.frame_type != bitvue_decode::decoder::FrameType::Key,
                mv_x: ((x as i16) % 16 - 8) * 2,
                mv_y: ((y as i16) % 16 - 8) * 2,
                transform_size: "TX_64X64".to_string(),
            });
        }
    }

    tracing::info!("get_overlay_data: Returning {} blocks for frame {}", blocks.len(), frame_index);

    Ok(OverlayDataResponse {
        frame_index,
        width: decoded.width,
        height: decoded.height,
        blocks,
        success: true,
        error: None,
    })
}
