//! File-related Tauri commands
//!
//! Commands for:
//! - Opening files
//! - Getting stream info
//! - Hex view data

mod types;
mod parsers;
mod open_command;
mod hex_command;
mod dependent_command;
mod stream_command;

// Re-export types
pub use types::{FileInfo, HexDataResponse};

// Tauri commands - wrappers to fix module path issues
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub async fn open_file(
    path: String,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<FileInfo, String> {
    open_command::open_file_impl(path, state).await
}

#[tauri::command]
pub async fn get_hex_data(
    offset: u64,
    size: usize,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<HexDataResponse, String> {
    hex_command::get_hex_data_impl(offset, size, state).await
}

#[tauri::command]
pub async fn open_dependent_file(
    path: String,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<FileInfo, String> {
    dependent_command::open_dependent_file_impl(path, state).await
}

#[tauri::command]
pub async fn get_stream_info(
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<String, String> {
    stream_command::get_stream_info_impl(state).await
}
