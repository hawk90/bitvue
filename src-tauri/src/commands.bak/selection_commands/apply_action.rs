//! Apply selection action command

use super::types::{SelectionStateResponse, SelectionActionRequest};
use super::converters::{selection_state_to_response, selection_action_from_request};
use crate::commands::AppState;

/// Apply a selection action (reducer pattern)
pub async fn apply_selection_action_impl(
    action: SelectionActionRequest,
    tauri_state: tauri::State<'_, AppState>,
) -> Result<SelectionStateResponse, String> {
    let core = tauri_state.core.lock().map_err(|e| e.to_string())?;

    // Convert request to SelectionAction
    let selection_action = selection_action_from_request(action)?;

    // Apply action using reducer (this will return the new state)
    let new_state = core.apply_selection_action(selection_action);

    Ok(selection_state_to_response(&new_state))
}
