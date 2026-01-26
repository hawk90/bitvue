//! Hex view command

use super::types::HexDataResponse;
use crate::commands::AppState;

/// Get hex data from the currently open file
pub async fn get_hex_data_impl(
    offset: u64,
    size: usize,
    state: tauri::State<'_, AppState>,
) -> Result<HexDataResponse, String> {
    tracing::info!("get_hex_data: Request for offset={}, size={}", offset, size);

    // Get decode service from state
    let decode_service = state.decode_service.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    // Get file data from decode service
    let file_data = decode_service.get_file_data(offset, size);

    match file_data {
        Some(data) => {
            // Format hex data as space-separated hex bytes
            let hex_data: Vec<String> = data.iter()
                .map(|b| format!("{:02X}", b))
                .collect();

            // Format ASCII data
            let ascii_data: String = data.iter()
                .map(|&b| if b >= 32 && b <= 126 { b as char } else { '.' })
                .collect();

            tracing::info!("get_hex_data: Returning {} bytes of hex data", data.len());

            Ok(HexDataResponse {
                offset,
                size: data.len(),
                hex_data: hex_data.join(" "),
                ascii_data,
                success: true,
                error: None,
            })
        }
        None => {
            tracing::warn!("get_hex_data: No file data available");
            Ok(HexDataResponse {
                offset,
                size: 0,
                hex_data: String::new(),
                ascii_data: String::new(),
                success: false,
                error: Some("No file loaded. Please open a video file first.".to_string()),
            })
        }
    }
}
