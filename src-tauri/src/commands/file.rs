//! File operations commands
//!
//! Commands for opening, closing, and querying file/stream information.
#![allow(clippy::unnecessary_filter_map)]

use std::path::{Path, PathBuf};
use serde::Serialize;

use crate::commands::{AppState, FileInfo};
use bitvue_core::{Command, Event, StreamId, UnitModel};
use bitvue_av1::parse_ivf_frames;
use bitvue_avc::{avc_frames_to_unit_nodes, extract_annex_b_frames as extract_avc_annex_b_frames};
use bitvue_hevc::{hevc_frames_to_unit_nodes, extract_annex_b_frames as extract_hevc_annex_b_frames};
use bitvue_vp9::{vp9_frames_to_unit_nodes, extract_vp9_frames};
use bitvue_formats::{detect_container_format, ContainerFormat};

/// Validate file path to prevent path traversal and access to sensitive directories
/// This is a public function so other modules (like decode_service) can use it
pub fn validate_file_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);

    // Check for path traversal attempts (.. components)
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("Invalid path: path traversal (..) not allowed".to_string());
    }

    // Check if path exists and is a file (not a directory)
    if !path.exists() {
        return Err(format!("File not found: {:?}", path));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {:?}", path));
    }

    // SECURITY: Detect and validate symlinks to prevent symlink-based attacks
    // Symlinks could be used to bypass directory restrictions
    if path.is_symlink() {
        let canonical = path.canonicalize()
            .map_err(|e| format!("Cannot resolve symlink: {}", e))?;

        // Check if the symlink target contains path traversal
        if canonical.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err("Invalid path: symlink target contains path traversal".to_string());
        }

        // Validate the symlink target against system directory restrictions
        let canonical_str = canonical.to_string_lossy();
        #[cfg(unix)]
        {
            let blocked_paths = ["/System", "/usr", "/bin", "/sbin", "/etc", "/var",
                "/boot", "/lib", "/lib64", "/root", "/sys", "/proc", "/dev"];
            for blocked in &blocked_paths {
                if canonical_str.starts_with(blocked) {
                    return Err(format!("Cannot access system directory via symlink ({})", blocked));
                }
            }
        }
        #[cfg(windows)]
        {
            let path_lower = canonical_str.to_lowercase();
            if path_lower.starts_with("c:\\windows")
                || path_lower.starts_with("c:\\program files")
                || path_lower.starts_with("c:\\program files (x86)")
                || path_lower.starts_with("c:\\programdata") {
                return Err("Cannot access system directories via symlink".to_string());
            }
        }

        // Use the canonical (resolved) path for further operations
        return Ok(canonical);
    }

    // Additional check: reject absolute paths to system directories
    if path.is_absolute() {
        let path_str = path.to_string_lossy();
        #[cfg(unix)]
        {
            let blocked_paths = ["/System", "/usr", "/bin", "/sbin", "/etc", "/var",
                "/boot", "/lib", "/lib64", "/root", "/sys", "/proc", "/dev"];
            for blocked in &blocked_paths {
                if path_str.starts_with(blocked) {
                    return Err(format!("Cannot access system directory ({})", blocked));
                }
            }
        }
        #[cfg(windows)]
        {
            let path_lower = path_str.to_lowercase();
            if path_lower.starts_with("c:\\windows")
                || path_lower.starts_with("c:\\program files")
                || path_lower.starts_with("c:\\program files (x86)")
                || path_lower.starts_with("c:\\programdata") {
                return Err("Cannot access system directories".to_string());
            }
        }
    }

    Ok(path)
}

/// Open a video file and parse its structure
///
/// Opens a video file, parses its structure, and adds it to recent files history.
#[tauri::command]
pub async fn open_file(
    path: String,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<FileInfo, String> {
    log::info!("open_file: Opening file at path: {}", path);

    // Validate file path for security
    let path_buf = match validate_file_path(&path) {
        Ok(p) => p,
        Err(e) => {
            log::error!("open_file: Path validation failed: {}", e);
            return Ok(FileInfo {
                path: path.clone(),
                size: 0,
                codec: "unknown".to_string(),
                success: false,
                error: Some(e),
            });
        }
    };

    // Get file size and detect codec from extension
    let size = std::fs::metadata(&path_buf)
        .map(|m| m.len())
        .unwrap_or(0);

    // SECURITY: Validate file size to prevent memory issues with extremely large files
    // Maximum file size: 2GB (2 * 1024 * 1024 * 1024 bytes)
    const MAX_FILE_SIZE: u64 = 2_147_483_648;
    if size > MAX_FILE_SIZE {
        log::error!("open_file: File too large: {} bytes (max: {} bytes)", size, MAX_FILE_SIZE);
        return Ok(FileInfo {
            path: path.clone(),
            size,
            codec: "unknown".to_string(),
            success: false,
            error: Some(format!("File too large: {} MB. Maximum supported size is 2 GB.",
                size / (1024 * 1024))),
        });
    }

    let ext = path_buf.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    let codec = detect_codec_from_extension(ext);
    log::info!("open_file: Detected codec: {}, file size: {} bytes", codec, size);

    // Use bitvue-core to open the file
    let (success, error) = {
        let core = state.core.lock().map_err(|e| e.to_string())?;
        let events = core.handle_command(Command::OpenFile {
            stream: StreamId::A,
            path: path_buf.clone(),
        });

        // Check for errors in events
        let mut success = false;
        let mut error = None;

        for event in events {
            match event {
                Event::ModelUpdated { kind: _, stream: _ } => {
                    success = true;
                    log::info!("open_file: ModelUpdated event received");
                }
                Event::DiagnosticAdded { diagnostic } => {
                    error = Some(diagnostic.message.clone());
                    log::error!("open_file: DiagnosticAdded: {}", diagnostic.message);
                }
                _ => {}
            }
        }
        (success, error)
    }; // Lock is dropped here

    // Try to parse the file (basic IVF/AV1 parsing for now)
    if success {
        // Detect container format
        let container_format = detect_container_format(&path_buf)
            .unwrap_or(ContainerFormat::Unknown);

        log::info!("open_file: Detected container format: {:?}", container_format);

        // Update thumbnail service with new file (clears cache)
        {
            let thumbnail_service = state.thumbnail_service.lock()
                .map_err(|e| format!("Failed to lock thumbnail service: {}", e))?;
            let _ = thumbnail_service.set_file(path_buf.clone())
                .map_err(|e| {
                    log::warn!("open_file: Failed to update thumbnail service: {}", e);
                });
        } // Lock is dropped here

        // Read file data
        let file_data = std::fs::read(&path_buf)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Parse based on format
        let parsed_frames = match container_format {
            ContainerFormat::IVF => {
                // For IVF files, do basic parsing to extract frame headers
                match parse_ivf_frames(&file_data) {
                    Ok((_header, frames)) => {
                        log::info!("open_file: IVF parsing successful, {} frames", frames.len());
                        Some(frames.into_iter().enumerate().map(|(idx, ivf_frame)| {
                            bitvue_core::UnitNode {
                                key: bitvue_core::UnitKey {
                                    stream: StreamId::A,
                                    unit_type: "FRAME".to_string(),
                                    offset: 0,  // IVF frames are parsed from memory; file offset not tracked
                                    size: ivf_frame.size as usize,
                                },
                                unit_type: "FRAME".to_string(),
                                offset: 0,  // IVF frames are parsed from memory; file offset not tracked
                                size: ivf_frame.size as usize,
                                frame_index: Some(idx),
                                frame_type: None,  // Will be determined later from parsing
                                pts: Some(ivf_frame.timestamp),
                                dts: None,
                                display_name: format!("Frame {}", idx),
                                children: Vec::new(),
                                qp_avg: None,
                                mv_grid: None,
                                temporal_id: None,
                                ref_frames: None,
                                ref_slots: None,
                            }
                        }).collect::<Vec<_>>())
                    }
                    Err(e) => {
                        log::error!("open_file: IVF parsing failed: {}", e);
                        None
                    }
                }
            }
            ContainerFormat::AnnexB => {
                // For H.264/H.265 Annex B raw files
                log::info!("open_file: Parsing Annex B byte stream...");

                // Try H.264 first
                match extract_avc_annex_b_frames(&file_data) {
                    Ok(avc_frames) if !avc_frames.is_empty() => {
                        log::info!("open_file: Extracted {} H.264 frames from Annex B", avc_frames.len());
                        Some(avc_frames_to_unit_nodes(&avc_frames))
                    }
                    _ => {
                        // Try H.265/HEVC
                        log::info!("open_file: Trying H.265/HEVC parsing...");
                        match extract_hevc_annex_b_frames(&file_data) {
                            Ok(hevc_frames) if !hevc_frames.is_empty() => {
                                log::info!("open_file: Extracted {} H.265 frames from Annex B", hevc_frames.len());
                                Some(hevc_frames_to_unit_nodes(&hevc_frames))
                            }
                            _ => {
                                log::warn!("open_file: Failed to parse as H.264 or H.265");
                                None
                            }
                        }
                    }
                }
            }
            ContainerFormat::MP4 => {
                // For MP4, extract AV1/H.264/H.265/VP9 samples
                log::info!("open_file: Attempting to extract video samples from MP4...");

                // Try AV1 first
                if let Ok(av1_samples) = bitvue_formats::mp4::extract_av1_samples(&file_data) {
                    log::info!("open_file: Extracted {} AV1 samples from MP4", av1_samples.len());
                    Some(mp4_samples_to_units(av1_samples, "av1"))
                }
                // Try VP9 (not yet implemented)
                // else if let Ok(vp9_samples) = bitvue_formats::mp4::extract_vp9_samples(&file_data) {
                //     log::info!("open_file: Extracted {} VP9 samples from MP4", vp9_samples.len());
                //     Some(vp9_samples_to_units(vp9_samples))
                // }
                // Try H.265/HEVC
                else if let Ok(hevc_samples) = bitvue_formats::mp4::extract_hevc_samples(&file_data) {
                    log::info!("open_file: Extracted {} HEVC samples from MP4", hevc_samples.len());
                    Some(mp4_samples_to_units(hevc_samples, "hevc"))
                }
                // Try H.264/AVC
                else if let Ok(avc_samples) = bitvue_formats::mp4::extract_avc_samples(&file_data) {
                    log::info!("open_file: Extracted {} AVC samples from MP4", avc_samples.len());
                    Some(mp4_samples_to_units(avc_samples, "avc"))
                } else {
                    log::warn!("open_file: Failed to extract any video samples from MP4");
                    None
                }
            }
            ContainerFormat::Matroska => {
                // For MKV/WebM, extract samples
                log::info!("open_file: Attempting to extract video samples from Matroska...");

                // Try AV1
                if let Ok(av1_samples) = bitvue_formats::mkv::extract_av1_samples(&file_data) {
                    log::info!("open_file: Extracted {} AV1 samples from MKV", av1_samples.len());
                    Some(mkv_samples_to_units(av1_samples, "av1"))
                }
                // Try VP9 (common in WebM, not yet implemented)
                // else if let Ok(vp9_samples) = bitvue_formats::mkv::extract_vp9_samples(&file_data) {
                //     log::info!("open_file: Extracted {} VP9 samples from MKV/WebM", vp9_samples.len());
                //     Some(vp9_samples_to_units(vp9_samples))
                // }
                // Try H.265/HEVC
                else if let Ok(hevc_samples) = bitvue_formats::mkv::extract_hevc_samples(&file_data) {
                    log::info!("open_file: Extracted {} HEVC samples from MKV", hevc_samples.len());
                    Some(mkv_samples_to_units(hevc_samples, "hevc"))
                }
                // Try H.264/AVC
                else if let Ok(avc_samples) = bitvue_formats::mkv::extract_avc_samples(&file_data) {
                    log::info!("open_file: Extracted {} AVC samples from MKV", avc_samples.len());
                    Some(mkv_samples_to_units(avc_samples, "avc"))
                } else {
                    log::warn!("open_file: Failed to extract any video samples from MKV");
                    None
                }
            }
            _ => {
                log::info!("open_file: Format {:?} not yet supported for extraction", container_format);
                None
            }
        };

        // If we successfully parsed frames, populate the stream
        if let Some(units) = parsed_frames {
            let unit_count = units.len();
            let frame_count = units.len();

            log::info!("open_file: Parsed {} units from file", unit_count);
            // Debug: log first few units
            for (i, u) in units.iter().take(5).enumerate() {
                log::info!("open_file: Unit[{}] frame_index={:?}, frame_type={:?}", i, u.frame_index, u.frame_type);
            }

            // Get stream state and populate units (re-acquire lock)
            {
                let core = state.core.lock().map_err(|e| e.to_string())?;
                let stream_a_lock = core.get_stream(StreamId::A);
                let mut stream_a = stream_a_lock.write();

                // Create UnitModel from parsed frames
                stream_a.units = Some(UnitModel {
                    units,
                    unit_count,
                    frame_count,
                });

                log::info!("open_file: Created UnitModel with {} units", unit_count);
            } // Lock is dropped here

            // Cache file data in decode_service for faster access
            // Use already-read data to avoid re-reading from disk (optimizes core lock duration)
            if let Err(e) = state.decode_service.lock()
                .map_err(|e| format!("Failed to lock decode service: {}", e))?
                .set_file_with_data(
                    path_buf.clone(),
                    codec.clone(),
                    file_data.clone()
                )
            {
                log::warn!("open_file: Failed to cache file data in decode_service: {}", e);
            }

            // Add to recent files on successful open
            if let Err(e) = crate::commands::recent_files::add_recent_file(app.clone(), path.clone()).await {
                log::warn!("open_file: Failed to add to recent files: {}", e);
            }
        }
    }

    Ok(FileInfo {
        path: path.clone(),
        size,
        codec,
        success,
        error,
    })
}

/// Close the current file
#[tauri::command]
pub async fn close_file(state: tauri::State<'_, AppState>) -> Result<(), String> {
    log::info!("close_file: Closing current file");

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let _events = core.handle_command(Command::CloseFile {
        stream: StreamId::A,
    });

    // Clear thumbnail cache
    let thumbnail_service = state.thumbnail_service.lock()
        .map_err(|e| format!("Failed to lock thumbnail service: {}", e))?;
    let _ = thumbnail_service.set_file(PathBuf::new())
        .map_err(|e| {
            log::warn!("close_file: Failed to update thumbnail service: {}", e);
        });

    // Clear decode service cache
    let decode_service = state.decode_service.lock()
        .map_err(|e| format!("Failed to lock decode service: {}", e))?;
    let _ = decode_service.clear_cache()
        .map_err(|e| {
            log::warn!("close_file: Failed to clear decode service cache: {}", e);
        });

    log::info!("close_file: File closed");
    Ok(())
}

/// Get stream information as JSON string
#[tauri::command]
pub async fn get_stream_info(state: tauri::State<'_, AppState>) -> Result<StreamInfo, String> {
    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    let file_path = stream_a.file_path.clone().map(|p| p.to_string_lossy().to_string());
    let unit_count = stream_a.units.as_ref().map(|u| u.unit_count).unwrap_or(0);
    let frame_count = stream_a.units.as_ref().map(|u| u.frame_count).unwrap_or(0);

    Ok(StreamInfo {
        file_path,
        unit_count,
        frame_count,
    })
}

/// Stream information (serializable)
#[derive(Debug, Clone, Serialize)]
pub struct StreamInfo {
    pub file_path: Option<String>,
    pub unit_count: usize,
    pub frame_count: usize,
}

/// Get frame list for the current stream
#[tauri::command]
pub async fn get_frames(state: tauri::State<'_, AppState>) -> Result<Vec<crate::commands::FrameData>, String> {
    use bitvue_core::StreamId;

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    let units = stream_a.units.as_ref()
        .ok_or("No units available")?;

    log::info!("get_frames: Total units: {}", units.units.len());
    log::info!("get_frames: Units with frame_index: {}", units.units.iter().filter(|u| u.frame_index.is_some()).count());

    // Debug: log first few units
    for (i, u) in units.units.iter().take(5).enumerate() {
        log::info!("get_frames: Unit[{}] frame_index={:?}, frame_type={:?}", i, u.frame_index, u.frame_type);
    }

    // Convert UnitNode to FrameData
    let frames: Vec<crate::commands::FrameData> = units.units.iter()
        .filter(|u| u.frame_index.is_some())
        .map(|u| crate::commands::FrameData {
            frame_index: u.frame_index.unwrap_or(0),
            frame_type: u.frame_type.clone().unwrap_or("UNKNOWN".to_string()),
            size: u.size,
            poc: None,
            pts: u.pts,
            key_frame: Some(u.frame_type.as_deref() == Some("KEY") || u.frame_type.as_deref() == Some("INTRA_ONLY")),
            display_order: u.frame_index,
            coding_order: u.frame_index.unwrap_or(0),
            temporal_id: None,
            spatial_id: None,
            ref_frames: None,
            ref_slots: None,
            duration: None,
        })
        .collect();

    log::info!("get_frames: Returning {} frames", frames.len());
    Ok(frames)
}

fn detect_codec_from_extension(ext: &str) -> String {
    match ext.to_lowercase().as_str() {
        "ivf" => "av1",
        "webm" => "vp9",
        "mkv" => "av1/vp9",
        "mp4" => "avc/hevc",
        "mov" => "avc/hevc",
        "h264" | "264" => "avc",
        "h265" | "265" => "hevc",
        "av1" => "av1",
        _ => "unknown",
    }.to_string()
}

/// Convert MP4 video samples to UnitNode list
fn mp4_samples_to_units(samples: Vec<Vec<u8>>, codec: &str) -> Vec<bitvue_core::UnitNode> {
    match codec {
        "avc" => {
            // For H.264, parse each sample to get frame type
            samples.into_iter().enumerate().filter_map(|(idx, sample_data)| {
                // Try to parse as H.264 Annex B (samples may have start codes)
                match extract_avc_annex_b_frames(&sample_data) {
                    Ok(frames) if !frames.is_empty() => {
                        Some(avc_frames_to_unit_nodes(&frames).into_iter().next().unwrap_or_else(|| {
                            // Fallback to basic node if parsing fails
                            bitvue_core::UnitNode {
                                key: bitvue_core::UnitKey {
                                    stream: StreamId::A,
                                    unit_type: "FRAME".to_string(),
                                    offset: 0,
                                    size: sample_data.len(),
                                },
                                unit_type: "FRAME".to_string(),
                                offset: 0,
                                size: sample_data.len(),
                                frame_index: Some(idx),
                                frame_type: None,
                                pts: None,
                                dts: None,
                                display_name: format!("Frame {} ({})", idx, codec),
                                children: Vec::new(),
                                qp_avg: None,
                                mv_grid: None,
                                temporal_id: None,
                                ref_frames: None,
                                ref_slots: None,
                            }
                        }))
                    }
                    _ => {
                        // Try parsing without start codes (length-prefixed mode)
                        // For MP4, samples are usually length-prefixed NAL units
                        // We need to add start codes for parsing
                        let mut with_start_codes = Vec::new();
                        let mut pos = 0;

                        while pos + 4 <= sample_data.len() {
                            // Read NAL unit length (big-endian)
                            let len = u32::from_be_bytes([
                                sample_data[pos],
                                sample_data[pos + 1],
                                sample_data[pos + 2],
                                sample_data[pos + 3],
                            ]) as usize;

                            pos += 4;

                            // Check if we have enough data
                            if pos + len <= sample_data.len() {
                                // Add start code
                                with_start_codes.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                                // Add NAL unit data
                                with_start_codes.extend_from_slice(&sample_data[pos..pos + len]);
                                pos += len;
                            } else {
                                break;
                            }
                        }

                        match extract_avc_annex_b_frames(&with_start_codes) {
                            Ok(frames) if !frames.is_empty() => {
                                Some(avc_frames_to_unit_nodes(&frames).into_iter().next().unwrap_or_else(|| {
                                    create_placeholder_unit(idx, codec, &sample_data)
                                }))
                            }
                            _ => Some(create_placeholder_unit(idx, codec, &sample_data))
                        }
                    }
                }
            }).collect()
        }
        "hevc" => {
            // For H.265, parse each sample to get frame type
            samples.into_iter().enumerate().filter_map(|(idx, sample_data)| {
                // Try to parse as H.265 Annex B (samples may have start codes)
                match extract_hevc_annex_b_frames(&sample_data) {
                    Ok(frames) if !frames.is_empty() => {
                        Some(hevc_frames_to_unit_nodes(&frames).into_iter().next().unwrap_or_else(|| {
                            // Fallback to basic node if parsing fails
                            bitvue_core::UnitNode {
                                key: bitvue_core::UnitKey {
                                    stream: StreamId::A,
                                    unit_type: "FRAME".to_string(),
                                    offset: 0,
                                    size: sample_data.len(),
                                },
                                unit_type: "FRAME".to_string(),
                                offset: 0,
                                size: sample_data.len(),
                                frame_index: Some(idx),
                                frame_type: None,
                                pts: None,
                                dts: None,
                                display_name: format!("Frame {} ({})", idx, codec),
                                children: Vec::new(),
                                qp_avg: None,
                                mv_grid: None,
                                temporal_id: None,
                                ref_frames: None,
                                ref_slots: None,
                            }
                        }))
                    }
                    _ => {
                        // Try parsing without start codes (length-prefixed mode)
                        // For MP4, samples are usually length-prefixed NAL units
                        // We need to add start codes for parsing
                        let mut with_start_codes = Vec::new();
                        let mut pos = 0;

                        while pos + 4 <= sample_data.len() {
                            // Read NAL unit length (big-endian)
                            let len = u32::from_be_bytes([
                                sample_data[pos],
                                sample_data[pos + 1],
                                sample_data[pos + 2],
                                sample_data[pos + 3],
                            ]) as usize;

                            pos += 4;

                            // Check if we have enough data
                            if pos + len <= sample_data.len() {
                                // Add start code
                                with_start_codes.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                                // Add NAL unit data
                                with_start_codes.extend_from_slice(&sample_data[pos..pos + len]);
                                pos += len;
                            } else {
                                break;
                            }
                        }

                        match extract_hevc_annex_b_frames(&with_start_codes) {
                            Ok(frames) if !frames.is_empty() => {
                                Some(hevc_frames_to_unit_nodes(&frames).into_iter().next().unwrap_or_else(|| {
                                    create_placeholder_unit(idx, codec, &sample_data)
                                }))
                            }
                            _ => Some(create_placeholder_unit(idx, codec, &sample_data))
                        }
                    }
                }
            }).collect()
        }
        _ => {
            // For other codecs, use basic placeholder
            samples.into_iter().enumerate().map(|(idx, sample_data)| {
                create_placeholder_unit(idx, codec, &sample_data)
            }).collect()
        }
    }
}

/// Convert MKV video samples to UnitNode list
fn mkv_samples_to_units(samples: Vec<Vec<u8>>, codec: &str) -> Vec<bitvue_core::UnitNode> {
    match codec {
        "avc" => {
            // For H.264, parse each sample to get frame type
            samples.into_iter().enumerate().filter_map(|(idx, sample_data)| {
                match extract_avc_annex_b_frames(&sample_data) {
                    Ok(frames) if !frames.is_empty() => {
                        Some(avc_frames_to_unit_nodes(&frames).into_iter().next().unwrap_or_else(|| {
                            create_placeholder_unit(idx, codec, &sample_data)
                        }))
                    }
                    _ => Some(create_placeholder_unit(idx, codec, &sample_data))
                }
            }).collect()
        }
        "hevc" => {
            // For H.265, parse each sample to get frame type
            samples.into_iter().enumerate().filter_map(|(idx, sample_data)| {
                match extract_hevc_annex_b_frames(&sample_data) {
                    Ok(frames) if !frames.is_empty() => {
                        Some(hevc_frames_to_unit_nodes(&frames).into_iter().next().unwrap_or_else(|| {
                            create_placeholder_unit(idx, codec, &sample_data)
                        }))
                    }
                    _ => Some(create_placeholder_unit(idx, codec, &sample_data))
                }
            }).collect()
        }
        _ => {
            // For other codecs, use basic placeholder
            samples.into_iter().enumerate().map(|(idx, sample_data)| {
                create_placeholder_unit(idx, codec, &sample_data)
            }).collect()
        }
    }
}

/// Convert VP9 video samples to UnitNode list
#[allow(dead_code)]
fn vp9_samples_to_units(samples: Vec<Vec<u8>>) -> Vec<bitvue_core::UnitNode> {
    // For VP9, concatenate samples and parse as VP9 stream
    // VP9 frames are directly concatenated (or in superframes)
    let mut combined_data = Vec::new();
    let mut frame_offsets = Vec::new();

    for sample in &samples {
        frame_offsets.push(combined_data.len());
        combined_data.extend_from_slice(sample);
    }

    // Parse VP9 frames from combined data
    match extract_vp9_frames(&combined_data) {
        Ok(vp9_frames) => {
            // Map to UnitNodes
            vp9_frames_to_unit_nodes(&vp9_frames)
        }
        Err(_) => {
            // Fallback to placeholder units
            samples.into_iter().enumerate().map(|(idx, sample_data)| {
                bitvue_core::UnitNode {
                    key: bitvue_core::UnitKey {
                        stream: StreamId::A,
                        unit_type: "FRAME".to_string(),
                        offset: 0,
                        size: sample_data.len(),
                    },
                    unit_type: "FRAME".to_string(),
                    offset: 0,
                    size: sample_data.len(),
                    frame_index: Some(idx),
                    frame_type: None,
                    pts: None,
                    dts: None,
                    display_name: format!("Frame {} (vp9)", idx),
                    children: Vec::new(),
                    qp_avg: None,
                    mv_grid: None,
                    temporal_id: None,
                    ref_frames: None,
                    ref_slots: None,
                }
            }).collect()
        }
    }
}

/// Create a placeholder UnitNode for unparsed samples
fn create_placeholder_unit(idx: usize, codec: &str, sample_data: &[u8]) -> bitvue_core::UnitNode {
    bitvue_core::UnitNode {
        key: bitvue_core::UnitKey {
            stream: StreamId::A,
            unit_type: "FRAME".to_string(),
            offset: 0,
            size: sample_data.len(),
        },
        unit_type: "FRAME".to_string(),
        offset: 0,
        size: sample_data.len(),
        frame_index: Some(idx),
        frame_type: None,
        pts: None,
        dts: None,
        display_name: format!("Frame {} ({})", idx, codec),
        children: Vec::new(),
        qp_avg: None,
        mv_grid: None,
        temporal_id: None,
        ref_frames: None,
        ref_slots: None,
    }
}
