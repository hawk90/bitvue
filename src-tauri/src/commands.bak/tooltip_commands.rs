//! Tooltip-related Tauri commands (Monster Pack v14 compliant)
//!
//! Commands for:
//! - Getting tooltip content for different contexts
//! - Player surface tooltips (pixel/block hover)
//! - Timeline tooltips (frame bars, markers)
//! - Syntax tooltips (field nodes)

use super::AppState;
use super::tooltip_helpers::{
    build_player_tooltip_from_core,
    build_timeline_tooltip_from_core,
    build_metrics_tooltip as build_metrics_tooltip_helper,
};
use bitvue_core::tooltip::{TooltipConfig, TooltipContent};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// Tooltip Types
// ═══════════════════════════════════════════════════════════════════════════

/// Tooltip request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipRequest {
    pub context: TooltipContext,
    pub config: Option<TooltipConfigRequest>,
}

/// Tooltip context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "context")]
pub enum TooltipContext {
    /// Player surface tooltip
    Player {
        frame_index: usize,
        pixel_x: u32,
        pixel_y: u32,
    },
    /// Timeline frame bar tooltip
    Timeline {
        frame_index: usize,
    },
    /// Metrics plot point tooltip
    Metrics {
        frame_index: usize,
        series_name: String,
    },
    /// Custom tooltip
    Custom {
        text: String,
    },
}

/// Tooltip configuration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipConfigRequest {
    pub hover_delay_ms: Option<u32>,
    pub cursor_offset: Option<(i32, i32)>,
    pub max_width: Option<u32>,
    pub enable_copy_actions: Option<bool>,
}

/// Tooltip response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipResponse {
    pub content: String,  // Multi-line formatted text
    pub position: Option<TooltipPosition>,
    pub copy_actions: Vec<CopyActionResponse>,
}

/// Tooltip position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipPosition {
    pub x: i32,
    pub y: i32,
    pub placement: TooltipPlacement,
}

/// Tooltip placement relative to cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TooltipPlacement {
    Top,
    Bottom,
    Left,
    Right,
    Auto,
}

/// Copy action response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyActionResponse {
    pub label: String,
    pub content: String,
}

// ═══════════════════════════════════════════════════════════════════════════
// Tauri Commands
// ═══════════════════════════════════════════════════════════════════════════

/// Get tooltip content for a given context
#[tauri::command]
pub async fn get_tooltip(
    request: TooltipRequest,
    state: tauri::State<'_, AppState>,
) -> Result<TooltipResponse, String> {
    let core = state.core.lock().map_err(|e| e.to_string())?;

    // Convert config
    let _config = request.config.map(|c| TooltipConfig {
        hover_delay_ms: c.hover_delay_ms.unwrap_or(150),
        cursor_offset: c.cursor_offset.unwrap_or((10, 10)),
        max_width: c.max_width.unwrap_or(400),
        enable_copy_actions: c.enable_copy_actions.unwrap_or(true),
    }).unwrap_or_default();

    // Build tooltip content based on context
    let content = match request.context {
        TooltipContext::Player { frame_index, pixel_x, pixel_y } => {
            build_player_tooltip(frame_index, pixel_x, pixel_y, &core)?
        }
        TooltipContext::Timeline { frame_index } => {
            build_timeline_tooltip(frame_index, &core)?
        }
        TooltipContext::Metrics { frame_index, series_name } => {
            build_metrics_tooltip(frame_index, series_name, &core)?
        }
        TooltipContext::Custom { text } => {
            TooltipContent::Custom(text)
        }
    };

    // Format content to string
    let formatted = match &content {
        TooltipContent::Timeline(tip) => tip.format(),
        TooltipContent::Metrics(tip) => tip.format(),
        TooltipContent::Custom(text) => text.clone(),
        _ => "N/A".to_string(),
    };

    // Extract copy actions
    let copy_actions = match &content {
        TooltipContent::Timeline(tip) => {
            tip.copy_actions.iter().map(|a| CopyActionResponse {
                label: a.label.clone(),
                content: a.content.clone(),
            }).collect()
        }
        TooltipContent::Metrics(tip) => {
            tip.copy_actions.iter().map(|a| CopyActionResponse {
                label: a.label.clone(),
                content: a.content.clone(),
            }).collect()
        }
        _ => vec![],
    };

    Ok(TooltipResponse {
        content: formatted,
        position: None,  // Frontend calculates position
        copy_actions,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

fn build_player_tooltip(
    frame_index: usize,
    pixel_x: u32,
    pixel_y: u32,
    core: &bitvue_core::Core,
) -> Result<TooltipContent, String> {
    build_player_tooltip_from_core(frame_index, pixel_x, pixel_y, core)
}

fn build_timeline_tooltip(
    frame_index: usize,
    core: &bitvue_core::Core,
) -> Result<TooltipContent, String> {
    build_timeline_tooltip_from_core(frame_index, core)
}

fn build_metrics_tooltip(
    frame_index: usize,
    series_name: String,
    _core: &bitvue_core::Core,
) -> Result<TooltipContent, String> {
    build_metrics_tooltip_helper(frame_index, series_name)
}
