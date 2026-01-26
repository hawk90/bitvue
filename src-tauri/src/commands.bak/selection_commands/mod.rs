//! Selection-related Tauri commands (Monster Pack v14 compliant)
//!
//! Commands for:
//! - Getting current selection state
//! - Updating selection with Tri-Sync authority rules
//! - Selection reducer pattern for immutable updates

mod types;
mod converters;
mod get_state;
mod apply_action;

// Re-export types
pub use types::*;

// Tauri commands - wrappers to fix module path issues
#[tauri::command]
pub async fn get_selection_state(
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<SelectionStateResponse, String> {
    get_state::get_selection_state_impl(state).await
}

#[tauri::command]
pub async fn apply_selection_action(
    action: SelectionActionRequest,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<SelectionStateResponse, String> {
    apply_action::apply_selection_action_impl(action, state).await
}
