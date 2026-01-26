//! Decode frame commands

use super::types::DecodedFrameResponse;
use crate::commands::AppState;

/// Decode a single frame
pub async fn decode_frame_impl(
    frame_index: usize,
    state: tauri::State<'_, AppState>,
) -> Result<DecodedFrameResponse, String> {
    tracing::info!("decode_frame: Request for frame {}", frame_index);

    // Get decode service from state
    let decode_service = state.decode_service.lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    // Check if a file is loaded
    let file_path = decode_service.get_file_path();
    if file_path.is_none() {
        tracing::warn!("decode_frame: No file loaded");
        return Ok(DecodedFrameResponse {
            frame_index,
            rgb_data_base64: String::new(),
            width: 0,
            height: 0,
            success: false,
            error: Some("No video file loaded. Please open a video file first (File > Open Bitstream).".to_string()),
        });
    }

    // Check if codec is set
    let codec = decode_service.codec();
    if codec.is_empty() {
        tracing::warn!("decode_frame: No codec set");
        return Ok(DecodedFrameResponse {
            frame_index,
            rgb_data_base64: String::new(),
            width: 0,
            height: 0,
            success: false,
            error: Some("Codec not detected. Please open a valid video file.".to_string()),
        });
    }

    tracing::info!("decode_frame: Decoding frame {} from {} (codec: {})",
        frame_index, file_path.as_ref().unwrap_or(&"<unknown>".to_string()), codec);

    // Attempt to decode the frame
    let decoded = decode_service.decode_frame(frame_index);

    match decoded {
        Ok(data) => {
            if data.success {
                tracing::info!("decode_frame: Successfully decoded frame {} - {}x{}",
                    frame_index, data.width, data.height);
            } else {
                tracing::warn!("decode_frame: Decoder returned success=false for frame {}: {:?}",
                    frame_index, data.error);
            }
            Ok(DecodedFrameResponse {
                frame_index: data.frame_index,
                rgb_data_base64: data.rgb_data_base64,
                width: data.width,
                height: data.height,
                success: data.success,
                error: data.error,
            })
        }
        Err(e) => {
            let error_msg = format!(
                "Failed to decode frame {}: {}\n\nFile: {}\nCodec: {}\n\nPossible causes:\n\
                 - Corrupted video file\n\
                 - Unsupported AV1 features\n\
                 - Invalid IVF format\n\
                 - Decoder not available",
                frame_index, e,
                file_path.as_ref().unwrap_or(&"<unknown>".to_string()),
                codec
            );
            tracing::error!("decode_frame: {}", error_msg);
            Ok(DecodedFrameResponse {
                frame_index,
                rgb_data_base64: String::new(),
                width: 0,
                height: 0,
                success: false,
                error: Some(error_msg),
            })
        }
    }
}

/// Decode a batch of frames
pub async fn decode_frames_batch_impl(
    frame_indices: Vec<usize>,
    _state: tauri::State<'_, AppState>,
) -> Result<Vec<DecodedFrameResponse>, String> {
    tracing::info!("decode_frames_batch: Request for {} frames", frame_indices.len());

    // TODO: Implement batch decoding
    // For now, return empty vector
    Ok(Vec::new())
}
