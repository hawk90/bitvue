//! Get selection state command

use super::types::SelectionStateResponse;
use super::converters::selection_state_to_response;
use crate::commands::AppState;

/// Get current selection state
pub async fn get_selection_state_impl(
    state: tauri::State<'_, AppState>,
) -> Result<SelectionStateResponse, String> {
    let core = state.core.lock().map_err(|e| e.to_string())?;

    // Get the selection state (read lock)
    let selection_lock = core.get_selection();
    let selection = selection_lock.read();

    Ok(selection_state_to_response(&selection))
}
