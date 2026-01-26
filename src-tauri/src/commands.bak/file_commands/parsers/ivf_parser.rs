//! IVF file parser
//!
//! IVF format: 32-byte header, then frames with 12-byte header each

use bitvue_av1::frame_header::{parse_frame_header_basic, FrameType};
use bitvue_core::{StreamId, UnitNode};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;

/// Parse IVF file and populate units
pub fn parse_ivf_file(path: &PathBuf, stream_id: StreamId) -> Result<Vec<UnitNode>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut reader = BufReader::new(file);

    // Read IVF header (32 bytes)
    let mut header = [0u8; 32];
    reader.read_exact(&mut header).map_err(|e| format!("Failed to read header: {}", e))?;

    // Verify IVF signature
    if &header[0..4] != b"DKIF" {
        return Err("Not a valid IVF file".to_string());
    }

    // Parse IVF header
    let timebase_den = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);
    let frame_count = u32::from_le_bytes([header[24], header[25], header[26], header[27]]) as usize;

    tracing::info!("IVF header: timebase_den={}, frame_count={}", timebase_den, frame_count);

    let mut units = Vec::new();
    let mut frame_index = 0;
    let mut current_offset = 32u64; // After header

    // Read frames
    loop {
        let frame_start = current_offset;

        // Read frame header (12 bytes)
        let mut frame_header = [0u8; 12];
        match reader.read_exact(&mut frame_header) {
            Ok(_) => {}
            Err(_) if frame_index == 0 => return Err("Failed to read first frame header".to_string()),
            Err(_) => break, // EOF
        }

        let frame_size = u32::from_le_bytes([frame_header[0], frame_header[1], frame_header[2], frame_header[3]]) as usize;
        let pts = u64::from_le_bytes([
            frame_header[4], frame_header[5], frame_header[6], frame_header[7],
            frame_header[8], frame_header[9], frame_header[10], frame_header[11],
        ]);

        // Calculate timestamp in nanoseconds
        let timestamp_ns = if timebase_den > 0 {
            pts as u64 * 1_000_000_000 / timebase_den as u64
        } else {
            0
        };

        tracing::debug!("Frame {}: offset={}, size={}, pts={}, timestamp_ns={}",
            frame_index, current_offset, frame_size, pts, timestamp_ns);

        // Read frame data to determine frame type
        // Read more data for proper frame header parsing (up to 100 bytes)
        let header_read_size = frame_size.min(100);
        let mut frame_data = vec![0u8; header_read_size];
        reader.read_exact(&mut frame_data).map_err(|e| format!("Failed to read frame data: {}", e))?;

        // Skip remaining frame data if we only read partial
        if frame_size > header_read_size {
            reader.seek(SeekFrom::Current((frame_size - header_read_size) as i64)).ok();
        }

        // Parse OBU header and frame header using bitvue-av1
        // Skip OBU header (usually 1-2 bytes with extension/size fields)
        let obu_header = frame_data[0];
        let obu_type = (obu_header >> 3) & 0x0F;
        let has_extension = (obu_header & 0x04) != 0;
        let has_size_field = (obu_header & 0x02) != 0;

        // Calculate OBU header size
        let mut obu_header_size = 1; // Basic OBU header
        if has_extension {
            obu_header_size += 1;
        }
        if has_size_field {
            // Skip LEB128 size field
            let mut offset = obu_header_size;
            while offset < frame_data.len() && (frame_data[offset] & 0x80) != 0 {
                offset += 1;
                obu_header_size += 1;
            }
            if offset < frame_data.len() {
                obu_header_size += 1;
            }
        }

        // Extract frame header payload (skip OBU header)
        let frame_header_payload = if obu_header_size < frame_data.len() {
            &frame_data[obu_header_size..]
        } else {
            &[]
        };

        // Parse frame header using bitvue-av1
        let frame_header = parse_frame_header_basic(frame_header_payload);

        // Determine frame type string
        let frame_type_str = match &frame_header {
            Ok(fh) => match fh.frame_type {
                FrameType::Key => "I".to_string(),
                FrameType::Inter => "P".to_string(), // Use P for Inter (simplified)
                FrameType::IntraOnly => "I".to_string(),
                FrameType::Switch => "I".to_string(),
            },
            Err(_) => {
                // Fallback to OBU type detection
                match obu_type {
                    6 => "I".to_string(), // OBU_FRAME (assume key frame)
                    _ => "P".to_string(), // Assume inter frame
                }
            }
        };

        // Create unit for this frame
        let mut unit = UnitNode::new(stream_id, "FRAME".to_string(), frame_start, frame_size + 12);
        unit.frame_index = Some(frame_index);

        // Store frame type and reference information
        unit.frame_type = Some(frame_type_str.clone());
        unit.pts = Some(timestamp_ns);

        // Store reference frame indices if available
        if let Ok(fh) = &frame_header {
            unit.ref_frame_idx = fh.ref_frame_idx.clone();
            // Store QP if available
            if let Some(qp) = fh.base_q_idx {
                unit.qp_avg = Some(qp);
            }
        }

        tracing::debug!("Frame {}: type={}, OBU_type={}, refs={:?}",
            frame_index, frame_type_str, obu_type,
            unit.ref_frame_idx.as_ref());

        unit.display_name = format!("Frame {} @ 0x{:08X} ({} bytes)", frame_index, frame_start, frame_size);

        units.push(unit);

        current_offset += 12 + frame_size as u64;
        frame_index += 1;

        // Stop if we've read all expected frames
        if frame_count > 0 && frame_index >= frame_count {
            break;
        }
        if frame_index >= 10000 {
            tracing::warn!("Stopping at frame limit 10000");
            break;
        }
    }

    tracing::info!("Parsed {} frames from IVF file", units.len());
    Ok(units)
}
