//! File operations commands
//!
//! Commands for opening, closing, and querying file/stream information.

use std::path::PathBuf;
use std::io::Read;
use serde::{Serialize, Deserialize};

use crate::commands::{AppState, FileInfo};
use bitvue_core::{Command, Event, StreamId, UnitModel, ContainerModel, ContainerFormat as CoreContainerFormat};
use bitvue_av1_codec::{parse_ivf_frames, parse_ivf_header, ObuIterator, ObuType, FrameType};
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

    // SECURITY: Use generic error message to avoid revealing exact file location
    if !canonical.exists() {
        return Err("File not found".to_string());
    }

    if !canonical.is_file() {
        return Err("Path is not a file".to_string());
    }

    Ok(canonical)
}

/// Validate and open a file in one operation to minimize TOCTOU window
///
/// This function combines path validation with file opening to reduce the
/// time-of-check-to-time-of-use (TOCTOU) vulnerability window where an attacker
/// could swap the file after validation but before it's opened.
///
/// # Returns
/// * `Ok((PathBuf, File))` - Validated canonical path and open file handle
/// * `Err(String)` - Validation or opening error
pub fn validate_and_open_file(path: &str) -> Result<(PathBuf, std::fs::File), String> {
    let path = PathBuf::from(path);

    // SECURITY CRITICAL: Canonicalize FIRST before any validation
    let canonical = path.canonicalize()
        .map_err(|e| format!("Invalid path: cannot resolve path '{}': {}", path.display(), e))?;

    // Validate the canonical path against system directory restrictions
    check_system_directory_access(&canonical.to_string_lossy())
        .map_err(|e| format!("Path validation failed: {}", e))?;

    // Open file IMMEDIATELY after validation to minimize TOCTOU window
    let file = std::fs::File::open(&canonical)
        .map_err(|e| format!("Cannot open file: {}", e))?;

    // Verify it's actually a file (not a directory/device)
    let metadata = file.metadata()
        .map_err(|e| format!("Cannot get file metadata: {}", e))?;

    if !metadata.is_file() {
        return Err("Path is not a file".to_string());
    }

    Ok((canonical, file))
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
                // SECURITY: Use generic error to avoid revealing which directory was blocked
                return Err("Cannot access system directory".to_string());
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
            // SECURITY: Use generic error to avoid revealing which directory was blocked
            return Err("Cannot access system directory".to_string());
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
    // SECURITY: Don't log file path to prevent information disclosure
    log::info!("open_file: Opening file");

    // SECURITY: Use validate_and_open_file to minimize TOCTOU window
    // This combines path validation and file opening in one atomic operation
    let (path_buf, file_handle) = match validate_and_open_file(&path) {
        Ok((p, f)) => (p, f),
        Err(e) => {
            log::error!("open_file: Path validation or file opening failed: {}", e);
            return Ok(FileInfo {
                path: path.clone(),
                size: 0,
                codec: "unknown".to_string(),
                success: false,
                error: Some(e),
            });
        }
    };

    // Get file size from the already-open file handle (no TOCTOU window)
    let size = file_handle.metadata()
        .map(|m| m.len())
        .unwrap_or(0);

    // SECURITY: Validate file size to prevent memory issues with extremely large files
    // Maximum file size: 2GB (2 * 1024 * 1024 * 1024 bytes)
    const MAX_FILE_SIZE: u64 = 2_147_483_648;
    if size > MAX_FILE_SIZE {
        // SECURITY: Don't log actual file size in error
        log::error!("open_file: File too large");
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

    let codec_from_ext = detect_codec_from_extension(ext);
    // SECURITY: Don't log file size to prevent information disclosure
    log::info!("open_file: Detected codec from extension: {}", codec_from_ext);

    // Final codec will be determined after reading file (for IVF files)
    // Default to extension-based codec, may be updated for IVF files
    let mut final_codec = codec_from_ext.clone();

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

        // Re-open file to read full contents for parsing (original handle was consumed by metadata check)
        let mut file_data = Vec::new();
        let mut file_handle_reopened = std::fs::File::open(&path_buf)
            .map_err(|e| format!("Failed to re-open file for reading: {}", e))?;
        file_handle_reopened.read_to_end(&mut file_data)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Override codec detection for IVF files by reading header
        final_codec = if container_format == ContainerFormat::IVF {
            detect_codec_from_ivf_header(&file_data).unwrap_or_else(|| codec_from_ext.clone())
        } else {
            codec_from_ext.clone()
        };
        log::info!("open_file: Final codec: {}", final_codec);

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

                // Set container metadata with codec info for YUV decode lookup
                stream_a.container = Some(ContainerModel {
                    format: match container_format {
                        ContainerFormat::IVF => CoreContainerFormat::Ivf,
                        ContainerFormat::AnnexB => CoreContainerFormat::Raw,
                        ContainerFormat::MP4 => CoreContainerFormat::Mp4,
                        ContainerFormat::Matroska => CoreContainerFormat::Mkv,
                        _ => CoreContainerFormat::Raw,
                    },
                    codec: final_codec.clone(),
                    track_count: 1,
                    duration_ms: None,
                    bitrate_bps: None,
                    width: None,
                    height: None,
                    bit_depth: None,
                });

                log::info!("open_file: Created UnitModel with {} units, codec={}", unit_count, final_codec);
            } // Lock is dropped here

            // Cache file data in decode_service for faster access
            // Use already-read data to avoid re-reading from disk (optimizes core lock duration)
            if let Err(e) = state.decode_service.lock()
                .map_err(|e| format!("Failed to lock decode service: {}", e))?
                .set_file_with_data(
                    path_buf.clone(),
                    final_codec.clone(),
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
        codec: final_codec,
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

fn unit_to_frame_data(u: &bitvue_core::UnitNode) -> crate::commands::FrameData {
    crate::commands::FrameData {
        frame_index: u.frame_index.unwrap_or(0),
        frame_type: u.frame_type.as_deref().unwrap_or("UNKNOWN").to_string(),
        size: u.size,
        poc: None,
        pts: u.pts,
        key_frame: Some(u.frame_type.as_deref() == Some("I")),
        display_order: u.frame_index,
        coding_order: u.frame_index.unwrap_or(0),
        temporal_id: None,
        spatial_id: None,
        ref_frames: None,
        ref_slots: None,
        duration: None,
    }
}

/// Get frame list for the current stream
#[tauri::command]
pub async fn get_frames(state: tauri::State<'_, AppState>) -> Result<Vec<crate::commands::FrameData>, String> {
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
        .map(unit_to_frame_data)
        .collect();

    // SECURITY: Don't log frame count to prevent information disclosure
    log::info!("get_frames: Frames returned");
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
    log::info!("get_frames_chunk: offset={}, limit={}", offset, limit);

    let core = state.core.lock().map_err(|e| e.to_string())?;
    let stream_a_lock = core.get_stream(StreamId::A);
    let stream_a = stream_a_lock.read();

    let units = stream_a.units.as_ref()
        .ok_or("No units available")?;

    // Convert UnitNode to FrameData
    let all_frames: Vec<crate::commands::FrameData> = units.units.iter()
        .filter(|u| u.frame_index.is_some())
        .map(unit_to_frame_data)
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
        "ivf" => "av1", // Will be overridden by IVF header detection
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

/// Detect codec from IVF header (more accurate than extension-based detection)
/// IVF header contains a 4-byte FourCC at offset 8 that identifies the codec:
/// - "VP80" = VP8
/// - "VP90" = VP9
/// - "AV01" = AV1
fn detect_codec_from_ivf_header(data: &[u8]) -> Option<String> {
    let header = parse_ivf_header(data).ok()?;
    match &header.fourcc {
        b"VP80" => Some("vp8".to_string()),
        b"VP90" => Some("vp9".to_string()),
        b"AV01" => Some("av1".to_string()),
        _ => {
            log::warn!("Unknown IVF FourCC: {:?}", std::str::from_utf8(&header.fourcc));
            None
        }
    }
}

/// Convert video samples to UnitNode list — codec-aware, works for MP4, MKV, and other containers
///
/// Generic over sample type so the same logic handles `Cow<[u8]>` (MP4) and `Vec<u8>` (MKV).
fn samples_to_units<S: AsRef<[u8]>>(samples: Vec<S>, codec: &str) -> Vec<bitvue_core::UnitNode> {
    match codec {
        "avc" => samples.into_iter().enumerate()
            .map(|(idx, s)| parse_avc_sample(idx, codec, s.as_ref()))
            .collect(),
        "hevc" => samples.into_iter().enumerate()
            .map(|(idx, s)| parse_hevc_sample(idx, codec, s.as_ref()))
            .collect(),
        "av1" => samples.into_iter().enumerate()
            .map(|(idx, s)| {
                let data = s.as_ref();
                let frame_type = av1_frame_type_str(data);
                create_placeholder_unit(idx, codec, data, frame_type)
            })
            .collect(),
        _ => samples.into_iter().enumerate()
            .map(|(idx, s)| create_placeholder_unit(idx, codec, s.as_ref(), None))
            .collect(),
    }
}

/// Convert VP9 video samples to UnitNode list
#[allow(dead_code)]
fn vp9_samples_to_units(samples: Vec<Vec<u8>>) -> Vec<bitvue_core::UnitNode> {
    let mut combined_data = Vec::new();
    for sample in &samples {
        combined_data.extend_from_slice(sample);
    }

    match extract_vp9_frames(&combined_data) {
        Ok(vp9_frames) => vp9_frames_to_unit_nodes(&vp9_frames),
        Err(_) => samples.into_iter().enumerate()
            .map(|(idx, s)| create_placeholder_unit(idx, "vp9", &s, None))
            .collect(),
    }
}

/// Find the first NAL unit byte after an Annex B start code or 4-byte length prefix
fn find_first_nal_byte(data: &[u8]) -> Option<u8> {
    // Try Annex B: scan for 0x00 0x00 0x01 or 0x00 0x00 0x00 0x01
    for i in 0..data.len().saturating_sub(3) {
        if data[i] == 0x00 && data[i + 1] == 0x00 {
            if data[i + 2] == 0x01 {
                return data.get(i + 3).copied();
            }
            if i + 3 < data.len() && data[i + 2] == 0x00 && data[i + 3] == 0x01 {
                return data.get(i + 4).copied();
            }
        }
    }
    // Try length-prefixed: NAL byte is after the 4-byte length field
    data.get(4).copied()
}

/// Best-effort AVC/H.264 frame type from NAL unit type byte (IDR detection only)
fn guess_avc_frame_type(data: &[u8]) -> Option<std::sync::Arc<str>> {
    let nal_type = find_first_nal_byte(data)? & 0x1F;
    match nal_type {
        5 => Some("I".into()),  // IDR slice — definitely a key frame
        _ => None,              // Non-IDR: could be P or B, can't determine without slice header
    }
}

/// Best-effort HEVC/H.265 frame type from NAL unit type byte (IDR/IRAP detection only)
fn guess_hevc_frame_type(data: &[u8]) -> Option<std::sync::Arc<str>> {
    let nal_type = (find_first_nal_byte(data)? >> 1) & 0x3F;
    match nal_type {
        16..=21 => Some("I".into()),  // IDR_W_RADL, IDR_N_LP, BLA, CRA — all IRAP/key frames
        _ => None,
    }
}

/// Extract AV1 frame type string from IVF frame data by scanning OBUs
fn av1_frame_type_str(frame_data: &[u8]) -> Option<std::sync::Arc<str>> {
    for obu in ObuIterator::new(frame_data) {
        if let Ok(obu) = obu {
            if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) {
                if let Some(ft) = obu.frame_type {
                    return Some(ft.as_str().into());
                }
            }
        }
    }
    None
}

/// Try parsing a sample as Annex B (direct first, then length-prefixed conversion)
///
/// Returns the first parsed UnitNode with `frame_index` overridden to `idx`, or `None`
/// if both attempts fail.
fn try_parse_sample<V, E: std::fmt::Debug>(
    idx: usize,
    sample_data: &[u8],
    extract: impl Fn(&[u8]) -> Result<Vec<V>, E>,
    to_nodes: impl Fn(&[V]) -> Vec<bitvue_core::UnitNode>,
) -> Option<bitvue_core::UnitNode> {
    // First try: direct Annex B parsing
    if let Ok(frames) = extract(sample_data) {
        if !frames.is_empty() {
            if let Some(mut unit) = to_nodes(&frames).into_iter().next() {
                unit.frame_index = Some(idx);
                return Some(unit);
            }
        }
    }
    // Second try: convert from length-prefixed to Annex B
    let converted = convert_length_prefixed_to_annex_b(sample_data);
    if let Ok(frames) = extract(&converted) {
        if !frames.is_empty() {
            if let Some(mut unit) = to_nodes(&frames).into_iter().next() {
                unit.frame_index = Some(idx);
                return Some(unit);
            }
        }
    }
    None
}

/// Convert length-prefixed NAL units to Annex B format (with start codes)
///
/// MP4 containers use length-prefixed NAL units (4-byte big-endian length),
/// while parsing libraries expect Annex B format (start codes 0x00 0x00 0x00 0x01).
/// This function converts between the two formats.
fn convert_length_prefixed_to_annex_b(sample_data: &[u8]) -> Vec<u8> {
    let mut with_start_codes = Vec::new();
    let mut pos: usize = 0;
    const HEADER_SIZE: usize = 4;

    // SECURITY: Use checked arithmetic to prevent integer overflow
    while pos.checked_add(HEADER_SIZE).map_or(false, |end| end <= sample_data.len()) {
        // Read NAL unit length (big-endian)
        let len = u32::from_be_bytes([
            sample_data[pos],
            sample_data[pos + 1],
            sample_data[pos + 2],
            sample_data[pos + 3],
        ]) as usize;

        // Safe increment using checked arithmetic
        let new_pos = match pos.checked_add(HEADER_SIZE) {
            Some(p) => p,
            None => {
                // If we somehow overflow, stop processing
                log::warn!("NAL unit position overflow detected, stopping conversion");
                break;
            }
        };
        pos = new_pos;

        // Safe length check using checked arithmetic
        let end_pos = match pos.checked_add(len) {
            Some(end) if end <= sample_data.len() => end,
            _ => break, // Overflow or not enough data
        };

        // Add start code
        with_start_codes.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        // Add NAL unit data
        with_start_codes.extend_from_slice(&sample_data[pos..end_pos]);
        pos = end_pos;
    }

    with_start_codes
}

/// Parse a single AVC/H.264 sample; falls back to placeholder with best-effort frame type
fn parse_avc_sample(idx: usize, codec: &str, sample_data: &[u8]) -> bitvue_core::UnitNode {
    try_parse_sample(idx, sample_data, extract_avc_annex_b_frames, |f| avc_frames_to_unit_nodes(f))
        .unwrap_or_else(|| create_placeholder_unit(idx, codec, sample_data, guess_avc_frame_type(sample_data)))
}

/// Parse a single HEVC/H.265 sample; falls back to placeholder with best-effort frame type
fn parse_hevc_sample(idx: usize, codec: &str, sample_data: &[u8]) -> bitvue_core::UnitNode {
    try_parse_sample(idx, sample_data, extract_hevc_annex_b_frames, |f| hevc_frames_to_unit_nodes(f))
        .unwrap_or_else(|| create_placeholder_unit(idx, codec, sample_data, guess_hevc_frame_type(sample_data)))
}

/// Create a placeholder UnitNode for samples that could not be fully parsed
fn create_placeholder_unit(
    idx: usize,
    codec: &str,
    sample_data: &[u8],
    frame_type: Option<std::sync::Arc<str>>,
) -> bitvue_core::UnitNode {
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
        frame_type,
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
            // SECURITY: Don't log frame count to prevent information disclosure
            log::info!("parse_ivf_container: IVF parsing successful");
            Some(frames.into_iter().enumerate().map(|(idx, ivf_frame)| {
                let frame_type = av1_frame_type_str(&ivf_frame.data);
                bitvue_core::UnitNode {
                    key: bitvue_core::UnitKey {
                        stream: StreamId::A,
                        unit_type: "FRAME".into(),
                        offset: 0,
                        size: ivf_frame.size as usize,
                    },
                    unit_type: "FRAME".into(),
                    offset: 0,
                    size: ivf_frame.size as usize,
                    frame_index: Some(idx),
                    frame_type,
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
            // SECURITY: Don't log frame count to prevent information disclosure
            log::info!("parse_annex_b_container: Extracted H.264 frames from Annex B");
            Some(avc_frames_to_unit_nodes(&avc_frames))
        }
        _ => {
            // Try H.265/HEVC
            log::info!("parse_annex_b_container: Trying H.265/HEVC parsing...");
            match extract_hevc_annex_b_frames(file_data) {
                Ok(hevc_frames) if !hevc_frames.is_empty() => {
                    // SECURITY: Don't log frame count to prevent information disclosure
                    log::info!("parse_annex_b_container: Extracted H.265 frames from Annex B");
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
/// Provides detailed error messages for debugging malformed files.
fn parse_mp4_container(file_data: &[u8]) -> Option<Vec<bitvue_core::UnitNode>> {
    log::info!("parse_mp4_container: Attempting to extract video samples from MP4...");

    // SECURITY: Validate minimum file size for MP4 format
    const MIN_MP4_SIZE: usize = 8; // At least need ftyp box header
    if file_data.len() < MIN_MP4_SIZE {
        log::warn!("parse_mp4_container: File too small to be valid MP4 ({} bytes)", file_data.len());
        return None;
    }

    // Check for valid MP4 signature (ftyp box)
    // MP4 files should have "ftyp" at offset 4 (after box size)
    let is_valid_mp4 = if file_data.len() >= 8 {
        &file_data[4..8] == b"ftyp"
    } else {
        false
    };

    if !is_valid_mp4 {
        log::warn!("parse_mp4_container: File does not appear to be MP4 format (missing ftyp box)");
        return None;
    }

    // Try each codec and log specific failures
    #[allow(unused_assignments)]
    let mut last_error = None;

    // Try AV1 first
    match bitvue_formats::mp4::extract_av1_samples(file_data) {
        Ok(av1_samples) if !av1_samples.is_empty() => {
            log::info!("parse_mp4_container: Extracted {} AV1 samples from MP4", av1_samples.len());
            return Some(samples_to_units(av1_samples, "av1"));
        }
        Ok(_) => {
            log::warn!("parse_mp4_container: AV1 track exists but contains no samples");
            last_error = Some("AV1 track exists but contains no samples");
        }
        Err(e) => {
            log::debug!("parse_mp4_container: AV1 extraction failed: {}", e);
            last_error = Some("extraction failed");
        }
    }

    // Try H.265/HEVC
    match bitvue_formats::mp4::extract_hevc_samples(file_data) {
        Ok(hevc_samples) if !hevc_samples.is_empty() => {
            log::info!("parse_mp4_container: Extracted {} HEVC samples from MP4", hevc_samples.len());
            return Some(samples_to_units(hevc_samples, "hevc"));
        }
        Ok(_) => {
            log::warn!("parse_mp4_container: HEVC track exists but contains no samples");
            if last_error.is_none() { last_error = Some("HEVC track exists but contains no samples"); }
        }
        Err(e) => {
            log::debug!("parse_mp4_container: HEVC extraction failed: {}", e);
            if last_error.is_none() { last_error = Some("extraction failed"); }
        }
    }

    // Try H.264/AVC
    match bitvue_formats::mp4::extract_avc_samples(file_data) {
        Ok(avc_samples) if !avc_samples.is_empty() => {
            log::info!("parse_mp4_container: Extracted {} AVC samples from MP4", avc_samples.len());
            return Some(samples_to_units(avc_samples, "avc"));
        }
        Ok(_) => {
            log::warn!("parse_mp4_container: AVC track exists but contains no samples");
            if last_error.is_none() { last_error = Some("AVC track exists but contains no samples"); }
        }
        Err(e) => {
            log::debug!("parse_mp4_container: AVC extraction failed: {}", e);
            if last_error.is_none() { last_error = Some("extraction failed"); }
        }
    }

    // All codecs failed
    log::warn!("parse_mp4_container: Failed to extract any video samples from MP4. Last error: {:?}",
        last_error.unwrap_or(&"No valid video track found"));
    None
}

/// Parse Matroska/WebM container format
///
/// Returns parsed unit nodes from MKV/WebM file, or None if parsing fails.
/// Tries AV1, HEVC, then AVC in order.
/// Provides detailed error messages for debugging malformed files.
fn parse_mkv_container(file_data: &[u8]) -> Option<Vec<bitvue_core::UnitNode>> {
    log::info!("parse_mkv_container: Attempting to extract video samples from Matroska...");

    // SECURITY: Validate minimum file size for MKV format
    const MIN_MKV_SIZE: usize = 4; // At least need EBML header
    if file_data.len() < MIN_MKV_SIZE {
        log::warn!("parse_mkv_container: File too small to be valid MKV/WebM ({} bytes)", file_data.len());
        return None;
    }

    // Check for valid Matroska signature (EBML header)
    let is_valid_mkv = file_data.len() >= 4 && &file_data[0..4] == b"\x1a\x45\xdf\xa3";

    if !is_valid_mkv {
        log::warn!("parse_mkv_container: File does not appear to be Matroska/WebM format (missing EBML header)");
        return None;
    }

    // Try each codec and log specific failures
    #[allow(unused_assignments)]
    let mut last_error = None;

    // Try AV1
    match bitvue_formats::mkv::extract_av1_samples(file_data) {
        Ok(av1_samples) if !av1_samples.is_empty() => {
            log::info!("parse_mkv_container: Extracted {} AV1 samples from MKV", av1_samples.len());
            return Some(samples_to_units(av1_samples, "av1"));
        }
        Ok(_) => {
            log::warn!("parse_mkv_container: AV1 track exists but contains no samples");
            last_error = Some("AV1 track exists but contains no samples");
        }
        Err(e) => {
            log::debug!("parse_mkv_container: AV1 extraction failed: {}", e);
            last_error = Some("extraction failed");
        }
    }

    // Try H.265/HEVC
    match bitvue_formats::mkv::extract_hevc_samples(file_data) {
        Ok(hevc_samples) if !hevc_samples.is_empty() => {
            log::info!("parse_mkv_container: Extracted {} HEVC samples from MKV", hevc_samples.len());
            return Some(samples_to_units(hevc_samples, "hevc"));
        }
        Ok(_) => {
            log::warn!("parse_mkv_container: HEVC track exists but contains no samples");
            if last_error.is_none() { last_error = Some("HEVC track exists but contains no samples"); }
        }
        Err(e) => {
            log::debug!("parse_mkv_container: HEVC extraction failed: {}", e);
            if last_error.is_none() { last_error = Some("extraction failed"); }
        }
    }

    // Try H.264/AVC
    match bitvue_formats::mkv::extract_avc_samples(file_data) {
        Ok(avc_samples) if !avc_samples.is_empty() => {
            log::info!("parse_mkv_container: Extracted {} AVC samples from MKV", avc_samples.len());
            return Some(samples_to_units(avc_samples, "avc"));
        }
        Ok(_) => {
            log::warn!("parse_mkv_container: AVC track exists but contains no samples");
            if last_error.is_none() { last_error = Some("AVC track exists but contains no samples"); }
        }
        Err(e) => {
            log::debug!("parse_mkv_container: AVC extraction failed: {}", e);
            if last_error.is_none() { last_error = Some("extraction failed"); }
        }
    }

    // All codecs failed
    log::warn!("parse_mkv_container: Failed to extract any video samples from MKV. Last error: {:?}",
        last_error.unwrap_or(&"No valid video track found"));
    None
}
