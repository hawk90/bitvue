//! File operations commands
//!
//! Commands for opening, closing, and querying file/stream information.

use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use crate::commands::{AppState, FileInfo};
use bitvue_core::{Command, Event, StreamId, UnitModel};
use bitvue_av1::parse_ivf_frames;
use bitvue_avc::{avc_frames_to_unit_nodes, extract_annex_b_frames as extract_avc_annex_b_frames};
use bitvue_hevc::{hevc_frames_to_unit_nodes, extract_annex_b_frames as extract_hevc_annex_b_frames};
use bitvue_vp9::{vp9_frames_to_unit_nodes, extract_vp9_frames};
use bitvue_formats::{detect_container_format, ContainerFormat};

/// Validate file path to prevent path traversal and access to sensitive directories
/// This is a public function so other modules (like decode_service) can use it
///
/// SECURITY: Canonicalize FIRST before any validation to fully resolve symlinks
/// and path traversal attempts. This ensures that paths like `/safe/../../../etc/passwd`
/// are properly caught before any existence or type checking occurs.
///
/// Validation order (critical for security):
/// 1. Canonicalize (resolves all .., symlinks, relative paths)
/// 2. Check system directory access
/// 3. Check if path exists and is a file
pub fn validate_file_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);

    // SECURITY CRITICAL: Canonicalize FIRST before any validation
    // This handles:
    // - All `..` components (path traversal)
    // - Symlinks (including nested symlinks)
    // - Relative path resolution
    // - Path separator normalization
    let canonical = path.canonicalize()
        .map_err(|e| format!("Invalid path: cannot resolve path '{}': {}", path.display(), e))?;

    // Validate the canonical path against system directory restrictions
    // This must happen on the canonicalized path to catch traversal attempts
    check_system_directory_access(&canonical.to_string_lossy())
        .map_err(|e| format!("Path validation failed: {}", e))?;

    // Check if canonical path exists and is a file (not a directory)
    // Check existence AFTER canonicalization to ensure we check the actual destination
    if !canonical.exists() {
        return Err(format!("File not found: {}", canonical.display()));
    }

    if !canonical.is_file() {
        return Err(format!("Path is not a file: {}", canonical.display()));
    }

    Ok(canonical)
}

/// Check if a canonical path is in a blocked system directory
///
/// This function is public so other modules (like export.rs) can reuse
/// the same system directory validation logic for consistency.
pub fn check_system_directory_access(canonical_str: &str) -> Result<(), String> {
    #[cfg(unix)]
    {
        let blocked_paths = ["/System", "/usr", "/bin", "/sbin", "/etc", "/var",
            "/boot", "/lib", "/lib64", "/root", "/sys", "/proc", "/dev"];
        for blocked in &blocked_paths {
            if canonical_str.starts_with(blocked) {
                return Err(format!("Cannot access system directory ({})", blocked));
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
            return Err("Cannot access system directories".to_string());
        }
    }
    Ok(())
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
        // SAFETY: Use u64 literals to prevent integer overflow in size calculation
        const BYTES_PER_MB: u64 = 1024_u64 * 1024_u64;
        return Ok(FileInfo {
            path: path.clone(),
            size,
            codec: "unknown".to_string(),
            success: false,
            error: Some(format!("File too large: {} MB. Maximum supported size is 2 GB.",
                size / BYTES_PER_MB)),
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

        // Parse based on format (using helper functions for better code organization)
        let parsed_frames = match container_format {
            ContainerFormat::IVF => parse_ivf_container(&file_data),
            ContainerFormat::AnnexB => parse_annex_b_container(&file_data),
            ContainerFormat::MP4 => parse_mp4_container(&file_data),
            ContainerFormat::Matroska => parse_mkv_container(&file_data),
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
            frame_type: u.frame_type.as_deref().unwrap_or("UNKNOWN").to_string(),
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

/// Get frame list in chunks for progressive loading
/// Returns frames from offset to offset + limit (inclusive)
#[tauri::command]
pub async fn get_frames_chunk(
    state: tauri::State<'_, AppState>,
    offset: usize,
    limit: usize,
) -> Result<ChunkedFramesResponse, String> {
    use bitvue_core::StreamId;

    log::info!("get_frames_chunk: offset={}, limit={}", offset, limit);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    let units = stream_a.units.as_ref()
        .ok_or("No units available")?;

    // Convert UnitNode to FrameData
    let all_frames: Vec<crate::commands::FrameData> = units.units.iter()
        .filter(|u| u.frame_index.is_some())
        .map(|u| crate::commands::FrameData {
            frame_index: u.frame_index.unwrap_or(0),
            frame_type: u.frame_type.as_deref().unwrap_or("UNKNOWN").to_string(),
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

    let total_frames = all_frames.len();

    // Calculate the actual range to return
    let start = offset.min(total_frames);
    let end = (offset + limit).min(total_frames);

    let frames_chunk: Vec<crate::commands::FrameData> = if start < end {
        all_frames[start..end].to_vec()
    } else {
        Vec::new()
    };

    let has_more = end < total_frames;

    log::info!(
        "get_frames_chunk: returning {}/{} frames (start={}, end={}, has_more={})",
        frames_chunk.len(),
        total_frames,
        start,
        end,
        has_more
    );

    Ok(ChunkedFramesResponse {
        frames: frames_chunk,
        total_frames,
        has_more,
        offset: start,
    })
}

/// Response structure for chunked frame loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedFramesResponse {
    pub frames: Vec<crate::commands::FrameData>,
    pub total_frames: usize,
    pub has_more: bool,
    pub offset: usize,
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
fn mp4_samples_to_units(samples: Vec<std::borrow::Cow<'_, [u8]>>, codec: &str) -> Vec<bitvue_core::UnitNode> {
    match codec {
        "avc" => {
            // For H.264, parse each sample to get frame type
            samples.into_iter().enumerate().map(|(idx, sample_data)| {
                parse_avc_sample(idx, codec, &sample_data)
            }).collect()
        }
        "hevc" => {
            // For H.265, parse each sample to get frame type
            samples.into_iter().enumerate().map(|(idx, sample_data)| {
                parse_hevc_sample(idx, codec, &sample_data)
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
            samples.into_iter().enumerate().map(|(idx, sample_data)| {
                parse_avc_sample(idx, codec, &sample_data)
            }).collect()
        }
        "hevc" => {
            // For H.265, parse each sample to get frame type
            samples.into_iter().enumerate().map(|(idx, sample_data)| {
                parse_hevc_sample(idx, codec, &sample_data)
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
                        unit_type: "FRAME".into(),
                        offset: 0,
                        size: sample_data.len(),
                    },
                    unit_type: "FRAME".into(),
                    offset: 0,
                    size: sample_data.len(),
                    frame_index: Some(idx),
                    frame_type: None,
                    pts: None,
                    dts: None,
                    display_name: format!("Frame {} (vp9)", idx).into(),
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

/// Convert length-prefixed NAL units to Annex B format (with start codes)
///
/// MP4 containers use length-prefixed NAL units (4-byte big-endian length),
/// while parsing libraries expect Annex B format (start codes 0x00 0x00 0x00 0x01).
/// This function converts between the two formats.
fn convert_length_prefixed_to_annex_b(sample_data: &[u8]) -> Vec<u8> {
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

    with_start_codes
}

/// Parse a single AVC/H.264 sample with fallback to placeholder
///
/// Tries to parse the sample as AVC, first trying direct Annex B parsing,
/// then trying length-prefixed to Annex B conversion if that fails.
fn parse_avc_sample(idx: usize, codec: &str, sample_data: &[u8]) -> bitvue_core::UnitNode {
    // First try: Direct Annex B parsing
    if let Ok(frames) = extract_avc_annex_b_frames(&sample_data) {
        if !frames.is_empty() {
            if let Some(unit) = avc_frames_to_unit_nodes(&frames).into_iter().next() {
                return unit;
            }
        }
    }

    // Second try: Convert from length-prefixed to Annex B
    let with_start_codes = convert_length_prefixed_to_annex_b(sample_data);
    if let Ok(frames) = extract_avc_annex_b_frames(&with_start_codes) {
        if !frames.is_empty() {
            if let Some(unit) = avc_frames_to_unit_nodes(&frames).into_iter().next() {
                return unit;
            }
        }
    }

    // Fallback: Create placeholder
    create_placeholder_unit(idx, codec, sample_data)
}

/// Parse a single HEVC/H.265 sample with fallback to placeholder
///
/// Tries to parse the sample as HEVC, first trying direct Annex B parsing,
/// then trying length-prefixed to Annex B conversion if that fails.
fn parse_hevc_sample(idx: usize, codec: &str, sample_data: &[u8]) -> bitvue_core::UnitNode {
    // First try: Direct Annex B parsing
    if let Ok(frames) = extract_hevc_annex_b_frames(&sample_data) {
        if !frames.is_empty() {
            if let Some(unit) = hevc_frames_to_unit_nodes(&frames).into_iter().next() {
                return unit;
            }
        }
    }

    // Second try: Convert from length-prefixed to Annex B
    let with_start_codes = convert_length_prefixed_to_annex_b(sample_data);
    if let Ok(frames) = extract_hevc_annex_b_frames(&with_start_codes) {
        if !frames.is_empty() {
            if let Some(unit) = hevc_frames_to_unit_nodes(&frames).into_iter().next() {
                return unit;
            }
        }
    }

    // Fallback: Create placeholder
    create_placeholder_unit(idx, codec, sample_data)
}

/// Create a placeholder UnitNode for unparsed samples
fn create_placeholder_unit(idx: usize, codec: &str, sample_data: &[u8]) -> bitvue_core::UnitNode {
    bitvue_core::UnitNode {
        key: bitvue_core::UnitKey {
            stream: StreamId::A,
            unit_type: "FRAME".into(),
            offset: 0,
            size: sample_data.len(),
        },
        unit_type: "FRAME".into(),
        offset: 0,
        size: sample_data.len(),
        frame_index: Some(idx),
        frame_type: None,
        pts: None,
        dts: None,
        display_name: format!("Frame {} ({})", idx, codec).into(),
        children: Vec::new(),
        qp_avg: None,
        mv_grid: None,
        temporal_id: None,
        ref_frames: None,
        ref_slots: None,
    }
}

/// Parse IVF container format (AV1)
///
/// Returns parsed unit nodes from IVF file, or None if parsing fails.
fn parse_ivf_container(file_data: &[u8]) -> Option<Vec<bitvue_core::UnitNode>> {
    log::info!("parse_ivf_container: Parsing IVF byte stream...");
    match parse_ivf_frames(file_data) {
        Ok((_header, frames)) => {
            log::info!("parse_ivf_container: IVF parsing successful, {} frames", frames.len());
            Some(frames.into_iter().enumerate().map(|(idx, ivf_frame)| {
                bitvue_core::UnitNode {
                    key: bitvue_core::UnitKey {
                        stream: StreamId::A,
                        unit_type: "FRAME".into(),
                        offset: 0,  // IVF frames are parsed from memory; file offset not tracked
                        size: ivf_frame.size as usize,
                    },
                    unit_type: "FRAME".into(),
                    offset: 0,  // IVF frames are parsed from memory; file offset not tracked
                    size: ivf_frame.size as usize,
                    frame_index: Some(idx),
                    frame_type: None,  // Will be determined later from parsing
                    pts: Some(ivf_frame.timestamp),
                    dts: None,
                    display_name: format!("Frame {}", idx).into(),
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
            log::error!("parse_ivf_container: IVF parsing failed: {}", e);
            None
        }
    }
}

/// Parse Annex B container format (H.264/H.265 raw files)
///
/// Returns parsed unit nodes from Annex B file, or None if parsing fails.
/// Tries H.264 first, then falls back to H.265/HEVC.
fn parse_annex_b_container(file_data: &[u8]) -> Option<Vec<bitvue_core::UnitNode>> {
    log::info!("parse_annex_b_container: Parsing Annex B byte stream...");

    // Try H.264 first
    match extract_avc_annex_b_frames(file_data) {
        Ok(avc_frames) if !avc_frames.is_empty() => {
            log::info!("parse_annex_b_container: Extracted {} H.264 frames from Annex B", avc_frames.len());
            Some(avc_frames_to_unit_nodes(&avc_frames))
        }
        _ => {
            // Try H.265/HEVC
            log::info!("parse_annex_b_container: Trying H.265/HEVC parsing...");
            match extract_hevc_annex_b_frames(file_data) {
                Ok(hevc_frames) if !hevc_frames.is_empty() => {
                    log::info!("parse_annex_b_container: Extracted {} H.265 frames from Annex B", hevc_frames.len());
                    Some(hevc_frames_to_unit_nodes(&hevc_frames))
                }
                _ => {
                    log::warn!("parse_annex_b_container: Failed to parse as H.264 or H.265");
                    None
                }
            }
        }
    }
}

/// Parse MP4 container format
///
/// Returns parsed unit nodes from MP4 file, or None if parsing fails.
/// Tries AV1, HEVC, then AVC in order.
fn parse_mp4_container(file_data: &[u8]) -> Option<Vec<bitvue_core::UnitNode>> {
    log::info!("parse_mp4_container: Attempting to extract video samples from MP4...");

    // Try AV1 first
    if let Ok(av1_samples) = bitvue_formats::mp4::extract_av1_samples(file_data) {
        log::info!("parse_mp4_container: Extracted {} AV1 samples from MP4", av1_samples.len());
        Some(mp4_samples_to_units(av1_samples, "av1"))
    }
    // Try H.265/HEVC
    else if let Ok(hevc_samples) = bitvue_formats::mp4::extract_hevc_samples(file_data) {
        log::info!("parse_mp4_container: Extracted {} HEVC samples from MP4", hevc_samples.len());
        Some(mp4_samples_to_units(hevc_samples, "hevc"))
    }
    // Try H.264/AVC
    else if let Ok(avc_samples) = bitvue_formats::mp4::extract_avc_samples(file_data) {
        log::info!("parse_mp4_container: Extracted {} AVC samples from MP4", avc_samples.len());
        Some(mp4_samples_to_units(avc_samples, "avc"))
    } else {
        log::warn!("parse_mp4_container: Failed to extract any video samples from MP4");
        None
    }
}

/// Parse Matroska/WebM container format
///
/// Returns parsed unit nodes from MKV/WebM file, or None if parsing fails.
/// Tries AV1, HEVC, then AVC in order.
fn parse_mkv_container(file_data: &[u8]) -> Option<Vec<bitvue_core::UnitNode>> {
    log::info!("parse_mkv_container: Attempting to extract video samples from Matroska...");

    // Try AV1
    if let Ok(av1_samples) = bitvue_formats::mkv::extract_av1_samples(file_data) {
        log::info!("parse_mkv_container: Extracted {} AV1 samples from MKV", av1_samples.len());
        Some(mkv_samples_to_units(av1_samples, "av1"))
    }
    // Try H.265/HEVC
    else if let Ok(hevc_samples) = bitvue_formats::mkv::extract_hevc_samples(file_data) {
        log::info!("parse_mkv_container: Extracted {} HEVC samples from MKV", hevc_samples.len());
        Some(mkv_samples_to_units(hevc_samples, "hevc"))
    }
    // Try H.264/AVC
    else if let Ok(avc_samples) = bitvue_formats::mkv::extract_avc_samples(file_data) {
        log::info!("parse_mkv_container: Extracted {} AVC samples from MKV", avc_samples.len());
        Some(mkv_samples_to_units(avc_samples, "avc"))
    } else {
        log::warn!("parse_mkv_container: Failed to extract any video samples from MKV");
        None
    }
}
