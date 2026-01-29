//! Parser Worker - Parses container and bitstream units
//!
//! Monster Pack v3: WORKER_MODEL.md

use bitvue_core::{
    ByteCache, ContainerFormat, ContainerModel, Result, StreamId, UnitModel, UnitNode,
};
use std::path::Path;
use std::sync::Arc;

/// Parse a file and populate StreamState
///
/// This is the main entry point for Phase 0 parsing.
/// It detects the container format, parses units, and updates the state.
///
/// Returns:
/// - ContainerModel with file metadata
/// - UnitModel with parsed units
/// - Vec<Diagnostic> with any parse errors encountered
pub fn parse_file(
    path: &Path,
    stream_id: StreamId,
    byte_cache: Arc<ByteCache>,
) -> Result<(
    ContainerModel,
    UnitModel,
    Vec<bitvue_core::event::Diagnostic>,
)> {
    tracing::info!("Parsing file: {:?}", path);

    // Detect format (read first 1MB for format detection)
    let header_data = byte_cache.read_range(0, byte_cache.len().min(1024 * 1024) as usize)?;
    let format = detect_format(header_data)?;

    tracing::info!("Detected format: {:?}", format);

    // Read entire file for parsing
    let file_len = byte_cache.len() as usize;
    let data = byte_cache.read_range(0, file_len)?;

    tracing::info!("Reading {} bytes for parsing", file_len);

    // Parse based on format
    let (container, units, diagnostics) = match format {
        ContainerFormat::Ivf => parse_ivf(data, stream_id, &byte_cache)?,
        ContainerFormat::Raw => parse_raw_av1(data, stream_id, &byte_cache)?,
        _ => {
            return Err(bitvue_core::BitvueError::UnsupportedCodec(format!(
                "Format {:?} not yet implemented",
                format
            )));
        }
    };

    Ok((container, units, diagnostics))
}

/// Detect container format from file signature
fn detect_format(data: &[u8]) -> Result<ContainerFormat> {
    if data.len() < 4 {
        return Err(bitvue_core::BitvueError::InvalidFile(
            "File too small".to_string(),
        ));
    }

    // Check IVF signature
    if data.starts_with(b"DKIF") {
        return Ok(ContainerFormat::Ivf);
    }

    // Check MP4/MOV
    if data.len() >= 8 {
        let box_type = &data[4..8];
        if matches!(box_type, b"ftyp" | b"moov" | b"mdat") {
            return Ok(ContainerFormat::Mp4);
        }
    }

    // Check MKV/WebM (EBML header)
    if data[0] == 0x1A && data.len() >= 4 && data[1] == 0x45 && data[2] == 0xDF && data[3] == 0xA3 {
        return Ok(ContainerFormat::Mkv);
    }

    // Check TS (MPEG-2 Transport Stream)
    if data[0] == 0x47 && data.len() >= 188 * 2 && data[188] == 0x47 {
        return Ok(ContainerFormat::Ts);
    }

    // Default to raw AV1 OBU stream
    Ok(ContainerFormat::Raw)
}

/// Parse IVF container
fn parse_ivf(
    data: &[u8],
    stream_id: StreamId,
    _byte_cache: &ByteCache,
) -> Result<(
    ContainerModel,
    UnitModel,
    Vec<bitvue_core::event::Diagnostic>,
)> {
    use bitvue_av1::{parse_all_obus_resilient, parse_ivf_frames};

    // Parse IVF frames (this also parses the header)
    let (ivf_header, ivf_frames) = parse_ivf_frames(data)?;

    tracing::info!(
        "IVF: {}x{} @ {} fps, {} frames",
        ivf_header.width,
        ivf_header.height,
        ivf_header.framerate_num as f64 / ivf_header.framerate_den as f64,
        ivf_frames.len()
    );

    // Concatenate all frame data into a single OBU stream
    let mut obu_data = Vec::new();
    let ivf_header_size = ivf_header.header_size as u64;

    // Track mapping from obu_data offset to file offset
    let mut obu_data_offset = 0u64;
    let mut frame_offsets = Vec::new();

    for (i, frame) in ivf_frames.iter().enumerate() {
        // Each IVF frame has a 12-byte header before the OBU data
        let file_offset = if i == 0 {
            ivf_header_size + 12 // First frame
        } else {
            // Calculate offset based on previous frames
            let prev_total: u64 = ivf_frames[..i].iter().map(|f| f.size as u64 + 12).sum();
            ivf_header_size + prev_total + 12
        };

        frame_offsets.push((obu_data_offset, file_offset));
        obu_data.extend_from_slice(&frame.data);
        obu_data_offset += frame.data.len() as u64;
    }

    // Parse OBUs with resilience to collect diagnostics
    let (parsed_obus, mut diagnostics) = parse_all_obus_resilient(&obu_data, stream_id);
    tracing::info!(
        "Parsed {} OBUs with {} diagnostics",
        parsed_obus.len(),
        diagnostics.len()
    );

    // Parse sequence header to get dimensions
    let (width, height, bit_depth) = extract_sequence_info(&obu_data, &parsed_obus);

    // Build container model
    let container = ContainerModel {
        format: ContainerFormat::Ivf,
        codec: format!(
            "AV1 (fourcc: {})",
            String::from_utf8_lossy(&ivf_header.fourcc)
        ),
        track_count: 1,
        duration_ms: None,
        bitrate_bps: None,
        width,
        height,
        bit_depth,
    };

    // Build unit model
    let mut units = Vec::new();
    let mut frame_count = 0;

    for obu in parsed_obus {
        // Convert obu_data offset to file offset
        let mut file_offset = ivf_header_size;
        for (obu_off, frame_off) in &frame_offsets {
            if obu.offset >= *obu_off {
                file_offset = frame_off + (obu.offset - obu_off);
            } else {
                break;
            }
        }

        let mut node = UnitNode::new(
            stream_id,
            obu.header.obu_type.name().to_string(),
            file_offset, // Use file offset, not obu_data offset
            obu.total_size as usize,
        );

        // Check if this is a frame
        if obu.header.obu_type.has_frame_data() {
            node.frame_index = Some(frame_count);
            frame_count += 1;

            // Extract frame type from frame header
            if let Some(frame_type) = extract_frame_type(&obu.payload) {
                node.frame_type = Some(frame_type);
            }

            // Extract QP from frame header
            // Per QP_HEATMAP_IMPLEMENTATION_SPEC.md: Extract base_q_idx from bitstream
            if let Some(qp) = extract_qp_from_frame(&obu.payload) {
                node.qp_avg = Some(qp);
            }

            // Extract reference slots (ref_frame_idx from frame header)
            if let Some((ref_slots, ref_frames)) = extract_reference_slots(&obu.payload) {
                node.ref_slots = Some(ref_slots);
                // Only set ref_frames if we have actual frame indices
                if !ref_frames.is_empty() {
                    node.ref_frames = Some(ref_frames);
                }
            }

            // Extract motion vectors from tile data
            if let (Some(w), Some(h)) = (width, height) {
                if let Some(mv_grid) = extract_mv_from_frame(&obu.payload, w, h) {
                    node.mv_grid = Some(mv_grid);
                }
            }
        }

        units.push(node);
    }

    let unit_count = units.len();
    let unit_model = UnitModel {
        units,
        unit_count,
        frame_count,
    };

    // Convert diagnostic offsets from obu_data to file offsets
    for diagnostic in &mut diagnostics {
        let obu_offset = diagnostic.offset_bytes;
        for (obu_off, frame_off) in &frame_offsets {
            if obu_offset >= *obu_off {
                diagnostic.offset_bytes = frame_off + (obu_offset - obu_off);
            } else {
                break;
            }
        }
    }

    Ok((container, unit_model, diagnostics))
}

/// Parse raw AV1 OBU stream
fn parse_raw_av1(
    data: &[u8],
    stream_id: StreamId,
    _byte_cache: &ByteCache,
) -> Result<(
    ContainerModel,
    UnitModel,
    Vec<bitvue_core::event::Diagnostic>,
)> {
    use bitvue_av1::parse_all_obus_resilient;

    // Parse OBUs directly with resilience
    let (obus, diagnostics) = parse_all_obus_resilient(data, stream_id);
    tracing::info!(
        "Parsed {} OBUs from raw stream with {} diagnostics",
        obus.len(),
        diagnostics.len()
    );

    // Parse sequence header to get dimensions
    let (width, height, bit_depth) = extract_sequence_info(data, &obus);

    // Build container model
    let container = ContainerModel {
        format: ContainerFormat::Raw,
        codec: "AV1".to_string(),
        track_count: 1,
        duration_ms: None,
        bitrate_bps: None,
        width,
        height,
        bit_depth,
    };

    // Build unit model
    let mut units = Vec::new();
    let mut frame_count = 0;

    for obu in obus {
        let mut node = UnitNode::new(
            stream_id,
            obu.header.obu_type.name().to_string(),
            obu.offset,
            obu.total_size as usize,
        );

        // Check if this is a frame
        if obu.header.obu_type.has_frame_data() {
            node.frame_index = Some(frame_count);
            frame_count += 1;

            // Extract frame type from frame header
            if let Some(frame_type) = extract_frame_type(&obu.payload) {
                node.frame_type = Some(frame_type);
            }

            // Extract reference slots (ref_frame_idx from frame header)
            if let Some((ref_slots, ref_frames)) = extract_reference_slots(&obu.payload) {
                node.ref_slots = Some(ref_slots);
                if !ref_frames.is_empty() {
                    node.ref_frames = Some(ref_frames);
                }
            }
        }

        units.push(node);
    }

    let unit_count = units.len();
    let unit_model = UnitModel {
        units,
        unit_count,
        frame_count,
    };

    Ok((container, unit_model, diagnostics))
}

/// Extract QP from frame OBU payload
/// Returns base_q_idx (0-255 for AV1)
fn extract_qp_from_frame(payload: &[u8]) -> Option<u8> {
    use bitvue_av1::parse_frame_header_basic;

    // Try to parse frame header
    match parse_frame_header_basic(payload) {
        Ok(header) => header.base_q_idx,
        Err(_) => None,
    }
}

/// Extract frame type from frame OBU payload
fn extract_frame_type(payload: &[u8]) -> Option<String> {
    use bitvue_av1::{parse_frame_header_basic, FrameType};

    // Try to parse frame header
    match parse_frame_header_basic(payload) {
        Ok(header) => {
            let type_str = match header.frame_type {
                FrameType::Key => "KEY",
                FrameType::Inter => "INTER",
                FrameType::BFrame => "B",
                FrameType::IntraOnly => "INTRA_ONLY",
                FrameType::Switch => "SWITCH",
                FrameType::SI => "SI",
                FrameType::SP => "SP",
                FrameType::Unknown => "UNKNOWN",
            };
            Some(type_str.to_string())
        }
        Err(_) => None,
    }
}

/// Extract reference slot indices from frame OBU payload
/// Returns (ref_slots, ref_frames) tuple
/// ref_slots: Raw slot indices from bitstream (e.g., [0, 3, 6] for LAST, GOLDEN, ALTREF)
/// ref_frames: Actual frame indices (requires tracking reference state, returns heuristic for now)
fn extract_reference_slots(payload: &[u8]) -> Option<(Vec<u8>, Vec<usize>)> {
    use bitvue_av1::parse_frame_header_basic;

    match parse_frame_header_basic(payload) {
        Ok(header) => {
            // ref_frame_idx is [LAST, GOLDEN, ALTREF] slot indices (3 bits each)
            if let Some(ref_idx) = header.ref_frame_idx {
                // Convert to vec, filtering out invalid values (7 is reserved/unused)
                let slots: Vec<u8> = ref_idx.iter().filter(|&&x| x < 7).copied().collect();

                if slots.is_empty() {
                    return None;
                }

                // For now, return empty ref_frames since we don't track reference state
                // TODO: Implement proper reference frame state tracking
                let ref_frames: Vec<usize> = vec![];

                Some((slots, ref_frames))
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Extract motion vectors from frame OBU payload
///
/// Parses tile data to extract actual motion vectors from INTER blocks.
/// Returns MVGrid with real MV data from bitstream.
fn extract_mv_from_frame(payload: &[u8], width: u32, height: u32) -> Option<bitvue_core::MVGrid> {
    use bitvue_av1::{parse_frame_header_basic, parse_superblock, SymbolDecoder};

    // Parse frame header to get frame type and header size
    let header = parse_frame_header_basic(payload).ok()?;

    // KEY frames are INTRA only (no motion vectors)
    let is_key_frame = header.frame_type == bitvue_av1::FrameType::Key;
    tracing::debug!(
        "Frame type: {:?}, is_key_frame: {}, header_size: {} bytes",
        header.frame_type,
        is_key_frame,
        header.header_size_bytes
    );
    if is_key_frame {
        tracing::debug!("Skipping KEY frame (no motion vectors)");
        return None;
    }

    // Use calculated header size to find tile data start
    let tile_start = header.header_size_bytes;
    if payload.len() <= tile_start {
        tracing::warn!(
            "Payload ({} bytes) too small for header ({} bytes)",
            payload.len(),
            tile_start
        );
        return None;
    }

    let tile_data = &payload[tile_start..];
    tracing::debug!(
        "Tile data starts at byte {}, {} bytes available",
        tile_start,
        tile_data.len()
    );

    // Parse ALL superblocks to get complete MV data
    let sb_size = 64; // Use 64x64 superblock
    let sb_cols = width.div_ceil(sb_size);
    let sb_rows = height.div_ceil(sb_size);
    let total_sbs = sb_cols * sb_rows;

    tracing::debug!(
        "Frame {}x{} -> {} x {} superblocks, parsing ALL {} superblocks",
        width,
        height,
        sb_cols,
        sb_rows,
        total_sbs
    );

    match SymbolDecoder::new(tile_data) {
        Ok(mut decoder) => {
            let mut all_mvs = Vec::new();

            // Create MV predictor context
            let mut mv_ctx = bitvue_av1::tile::MvPredictorContext::new(sb_cols, sb_rows);

            // Parse ALL superblocks in the frame
            let mut parsed_sbs = 0;
            for sb_idx in 0..total_sbs {
                let sb_x = (sb_idx % sb_cols) * sb_size;
                let sb_y = (sb_idx / sb_cols) * sb_size;

                // Default QP and delta_q_enabled for MVP
                let base_qp = 128_i16;
                match parse_superblock(
                    &mut decoder,
                    sb_x,
                    sb_y,
                    sb_size,
                    false,
                    base_qp,
                    false,
                    &mut mv_ctx,
                ) {
                    Ok((superblock, _final_qp)) => {
                        let mvs = superblock.motion_vectors();
                        if !mvs.is_empty() {
                            tracing::debug!(
                                "SB {} at ({}, {}): {} MVs",
                                sb_idx,
                                sb_x,
                                sb_y,
                                mvs.len()
                            );
                        }
                        all_mvs.extend(mvs);
                        parsed_sbs += 1;
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to parse SB {} at ({}, {}): {:?}, stopping",
                            sb_idx,
                            sb_x,
                            sb_y,
                            e
                        );
                        break;
                    }
                }
            }

            tracing::info!(
                "Extracted {} total motion vectors from {} / {} superblocks",
                all_mvs.len(),
                parsed_sbs,
                total_sbs
            );

            if all_mvs.is_empty() {
                tracing::debug!("No motion vectors found in any superblock");
                return None;
            }

            // Create MV grid
            // For MVP, use 16x16 block size
            let block_size = 16;
            let grid_w = width.div_ceil(block_size);
            let grid_h = height.div_ceil(block_size);

            // Initialize with missing MVs so we don't draw non-existent vectors
            let mut mv_l0 =
                vec![bitvue_core::mv_overlay::MotionVector::MISSING; (grid_w * grid_h) as usize];

            // Fill in actual MVs from parsed data
            for (x, y, w, h, mv) in all_mvs {
                // Convert block to grid coordinates
                let grid_x = x / block_size;
                let grid_y = y / block_size;
                let grid_w_blocks = w.div_ceil(block_size);
                let grid_h_blocks = h.div_ceil(block_size);

                // Fill grid cells covered by this block
                for dy in 0..grid_h_blocks {
                    for dx in 0..grid_w_blocks {
                        let gx = grid_x + dx;
                        let gy = grid_y + dy;

                        if gx < grid_w && gy < grid_h {
                            let idx = (gy * grid_w + gx) as usize;
                            if idx < mv_l0.len() {
                                // Pass qpel values directly without division
                                mv_l0[idx] = bitvue_core::mv_overlay::MotionVector::new(mv.x, mv.y);
                            }
                        }
                    }
                }
            }

            let mv_grid = bitvue_core::MVGrid::new(
                width,
                height,
                block_size,
                block_size,
                mv_l0,
                // Initialize L1 with missing too; MVP extracts only L0 for now
                vec![bitvue_core::mv_overlay::MotionVector::MISSING; (grid_w * grid_h) as usize],
                None,
            );
            tracing::info!(
                "Created MVGrid: {}x{} blocks, block_size={}x{}",
                grid_w,
                grid_h,
                block_size,
                block_size
            );
            Some(mv_grid)
        }
        Err(e) => {
            tracing::warn!("Failed to create symbol decoder: {:?}", e);
            None
        }
    }
}

/// Extract sequence info (dimensions, bit depth) from OBUs
///
/// Searches for SEQUENCE_HEADER OBU and parses it to extract video parameters.
fn extract_sequence_info(
    _data: &[u8],
    obus: &[bitvue_av1::Obu],
) -> (Option<u32>, Option<u32>, Option<u8>) {
    use bitvue_av1::{parse_sequence_header, ObuType};

    // Find sequence header OBU
    for obu in obus {
        if obu.header.obu_type == ObuType::SequenceHeader {
            // Parse sequence header from payload
            if let Ok(seq_header) = parse_sequence_header(&obu.payload) {
                let width = seq_header.max_frame_width;
                let height = seq_header.max_frame_height;
                let bit_depth = seq_header.color_config.bit_depth;

                tracing::info!(
                    "Extracted sequence info: {}x{} @ {} bits",
                    width,
                    height,
                    bit_depth
                );

                return (Some(width), Some(height), Some(bit_depth));
            } else {
                tracing::warn!("Failed to parse sequence header");
            }
        }
    }

    tracing::warn!("No sequence header found in stream");
    (None, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_ivf() {
        let ivf_data = b"DKIF\x00\x00\x20\x00";
        assert_eq!(detect_format(ivf_data).unwrap(), ContainerFormat::Ivf);
    }

    #[test]
    fn test_detect_format_mp4() {
        let mut mp4_data = Vec::new();
        mp4_data.extend_from_slice(&20u32.to_be_bytes());
        mp4_data.extend_from_slice(b"ftyp");
        assert_eq!(detect_format(&mp4_data).unwrap(), ContainerFormat::Mp4);
    }

    #[test]
    fn test_detect_format_mkv() {
        let mkv_data = [0x1A, 0x45, 0xDF, 0xA3, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_format(&mkv_data).unwrap(), ContainerFormat::Mkv);
    }

    #[test]
    fn test_detect_format_raw() {
        let raw_data = b"\x12\x00\x0A\x00"; // Some random OBU-like data (min 4 bytes)
        assert_eq!(detect_format(raw_data).unwrap(), ContainerFormat::Raw);
    }
}
