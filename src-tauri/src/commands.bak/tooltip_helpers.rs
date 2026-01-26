//! Tooltip helpers for bitstream analysis
//!
//! Provides helper functions for building tooltip responses

use bitvue_core::tooltip::{TooltipContent, PlayerTooltip, TimelineTooltip, MetricsTooltip, CopyAction};
use bitvue_core::{Core, StreamId};

/// Extract player tooltip info from core and pixel data
pub fn build_player_tooltip_from_core(
    frame_index: usize,
    pixel_x: u32,
    pixel_y: u32,
    core: &Core,
) -> Result<TooltipContent, String> {
    // Get file data from Core's stream state
    let stream = core.get_stream(StreamId::A);
    let state = stream.read();

    let byte_cache = state.byte_cache.as_ref()
        .ok_or_else(|| "No file data available".to_string())?;

    let file_size = byte_cache.len();
    let file_data = byte_cache.read_range(0, file_size as usize)
        .map_err(|e| format!("Failed to read file data: {}", e))?;

    // Extract real pixel info from AV1 bitstream
    let pixel_info = bitvue_av1::extract_pixel_info(
        file_data,
        frame_index,
        pixel_x,
        pixel_y,
    ).map_err(|e| format!("Failed to extract pixel info: {}", e))?;

    // Convert luma/chroma values
    let luma = pixel_info.luma;
    let chroma = match (pixel_info.chroma_u, pixel_info.chroma_v) {
        (Some(u), Some(v)) => Some((u, v)),
        _ => None,
    };

    let tooltip = PlayerTooltip {
        frame_idx: pixel_info.frame_index,
        pixel_xy: (pixel_info.pixel_x, pixel_info.pixel_y),
        luma,
        chroma,
        block_id: Some(pixel_info.block_id.clone()),
        qp: pixel_info.qp,
        mv: pixel_info.mv,
        partition_info: Some(pixel_info.partition_info.clone()),
        active_overlays: vec!["QP Heatmap".to_string(), "MV Overlay".to_string()],
        syntax_path: Some(pixel_info.syntax_path.clone()),
        bit_offset: pixel_info.bit_offset,
        byte_offset: pixel_info.byte_offset,
        copy_actions: create_player_copy_actions(frame_index, pixel_x, pixel_y, &pixel_info.block_id),
    };

    Ok(TooltipContent::Player(tooltip))
}

/// Extract frame information from core for timeline tooltip
pub fn build_timeline_tooltip_from_core(
    frame_index: usize,
    core: &Core,
) -> Result<TooltipContent, String> {
    let stream = core.get_stream(StreamId::A);
    let stream = stream.read();

    let frame_type = if let Some(units) = &stream.units {
        units.units.get(frame_index)
            .map(|u| u.unit_type.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string())
    } else {
        "UNKNOWN".to_string()
    };

    let copy_actions = create_timeline_copy_actions(frame_index);

    let tooltip = TimelineTooltip {
        frame_idx: frame_index,
        frame_type,
        pts: Some(frame_index as u64 * 33),
        dts: Some(frame_index as u64 * 33),
        time_seconds: Some((frame_index as f64) / 30.0),
        size_bytes: Some(25000 + (frame_index % 100) * 100),
        size_bits: Some((25000 + (frame_index % 100) * 100) * 8),
        markers: vec![],
        decoded: true,
        decode_error: None,
        syntax_path: Some(format!("OBU_FRAME[{}]", frame_index)),
        bit_offset: Some(frame_index as u64 * 200000),
        byte_offset: Some(frame_index as u64 * 25000),
        copy_actions,
    };

    Ok(TooltipContent::Timeline(tooltip))
}

/// Build metrics tooltip for graph points
pub fn build_metrics_tooltip(
    frame_index: usize,
    series_name: String,
) -> Result<TooltipContent, String> {
    let value = calculate_series_value(frame_index, &series_name);

    let tooltip = MetricsTooltip {
        frame_idx: frame_index,
        time_seconds: Some((frame_index as f64) / 30.0),
        series_name,
        value: value as f32,
        unit: "dB".to_string(),
        delta: Some((frame_index as f32 % 3.0) - 1.0),
        copy_actions: vec![
            CopyAction {
                label: "Copy Value".to_string(),
                content: format!("{:.2}", value),
            },
        ],
    };

    Ok(TooltipContent::Metrics(tooltip))
}

/// Calculate value for a metrics series at a given frame
fn calculate_series_value(frame_index: usize, series_name: &str) -> f64 {
    match series_name {
        "PSNR_Y" => 35.0 + (frame_index as f64 % 10.0),
        "SSIM_Y" => 0.95 + (frame_index as f64 % 5.0) / 100.0,
        _ => 0.0,
    }
}

/// Create copy actions for a timeline tooltip
fn create_timeline_copy_actions(frame_index: usize) -> Vec<CopyAction> {
    vec![
        CopyAction {
            label: "Copy Frame Index".to_string(),
            content: frame_index.to_string(),
        },
        CopyAction {
            label: "Copy Byte Offset".to_string(),
            content: (frame_index * 25000).to_string(),
        },
    ]
}

/// Create copy actions for a player tooltip
pub fn create_player_copy_actions(
    frame_index: usize,
    pixel_x: u32,
    pixel_y: u32,
    block_id: &str,
) -> Vec<CopyAction> {
    vec![
        CopyAction {
            label: "Copy Pixel XY".to_string(),
            content: format!("{},{}", pixel_x, pixel_y),
        },
        CopyAction {
            label: "Copy Frame Index".to_string(),
            content: frame_index.to_string(),
        },
        CopyAction {
            label: "Copy Block ID".to_string(),
            content: block_id.to_string(),
        },
    ]
}
