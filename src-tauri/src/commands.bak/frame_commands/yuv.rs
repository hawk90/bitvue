//! YUV data and histogram commands

use super::types::{YuvDataResponse, HistogramResponse};
use crate::commands::AppState;
use base64::prelude::*;
use bitvue_decode::Av1Decoder;

/// Get YUV plane data for a frame
pub async fn get_yuv_data_impl(
    frame_index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<YuvDataResponse, String> {
    tracing::info!("get_yuv_data: Request for frame {}", frame_index);

    // Get decode service from state
    let decode_service = state.decode_service.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    // Check if a file is loaded
    let file_path = decode_service.get_file_path();
    if file_path.is_none() {
        return Ok(YuvDataResponse {
            frame_index,
            width: 0,
            height: 0,
            y_plane: String::new(),
            u_plane: String::new(),
            v_plane: String::new(),
            success: false,
            error: Some("No video file loaded".to_string()),
        });
    }

    // Get the file data
    let file_data = decode_service.file_data.as_ref()
        .ok_or_else(|| "No file data available".to_string())?;

    // Create decoder and decode
    let mut decoder = Av1Decoder::new()
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let decoded_frames = decoder.decode_all(file_data)
        .map_err(|e| format!("Decode failed: {}", e))?;

    let decoded = decoded_frames.get(frame_index)
        .ok_or_else(|| format!("Frame {} not found", frame_index))?;

    Ok(YuvDataResponse {
        frame_index,
        width: decoded.width,
        height: decoded.height,
        y_plane: BASE64_STANDARD.encode(&decoded.y_plane),
        u_plane: BASE64_STANDARD.encode(&decoded.u_plane.as_ref().unwrap_or(&vec![])),
        v_plane: BASE64_STANDARD.encode(&decoded.v_plane.as_ref().unwrap_or(&vec![])),
        success: true,
        error: None,
    })
}

/// Calculate histogram from plane data
pub fn calculate_histogram(plane: &[u8], bins: usize) -> Vec<u32> {
    let mut hist = vec![0u32; bins];
    for &val in plane {
        let bin = (val as usize).min(bins - 1);
        hist[bin] += 1;
    }
    hist
}

/// Get YUV histogram data
pub async fn get_yuv_histogram_impl(
    frame_index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<HistogramResponse, String> {
    tracing::info!("get_yuv_histogram: Request for frame {}", frame_index);

    // Get decode service from state
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

    // Calculate histograms
    let y_hist = calculate_histogram(&decoded.y_plane, 256);
    let u_hist = calculate_histogram(&decoded.u_plane.as_ref().unwrap_or(&vec![]), 256);
    let v_hist = calculate_histogram(&decoded.v_plane.as_ref().unwrap_or(&vec![]), 256);

    Ok(HistogramResponse {
        frame_index,
        y_histogram: y_hist,
        u_histogram: u_hist,
        v_histogram: v_hist,
        y_mean: decoded.y_plane.iter().map(|&x| x as u32).sum::<u32>() / decoded.y_plane.len() as u32,
        u_mean: decoded.u_plane.as_ref().unwrap_or(&vec![]).iter().map(|&x| x as u32).sum::<u32>() / decoded.u_plane.as_ref().unwrap_or(&vec![]).len().max(1) as u32,
        v_mean: decoded.v_plane.as_ref().unwrap_or(&vec![]).iter().map(|&x| x as u32).sum::<u32>() / decoded.v_plane.as_ref().unwrap_or(&vec![]).len().max(1) as u32,
        success: true,
        error: None,
    })
}
