//! IVF container parsing into `UnitNode`s.

use anyhow::Result;
use bitvue_av1_codec::frame_header::{parse_frame_header_basic, FrameType};
use bitvue_engine::{StreamId, UnitNode};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;

/// Parse IVF file and return units
pub(crate) fn parse_ivf_file(path: &PathBuf, stream_id: StreamId) -> Result<Vec<UnitNode>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut reader = BufReader::new(file);

    // Read IVF header (32 bytes)
    let mut header = [0u8; 32];
    reader
        .read_exact(&mut header)
        .map_err(|e| format!("Failed to read header: {}", e))?;

    // Verify IVF signature
    if &header[0..4] != b"DKIF" {
        return Err(format!(
            "Not a valid IVF file: signature = {:?}, expected DKIF",
            &header[0..4]
        ));
    }

    // Parse IVF header
    let timebase_den = u32::from_le_bytes([header[16], header[17], header[18], header[19]]);
    let _frame_count =
        u32::from_le_bytes([header[24], header[25], header[26], header[27]]) as usize;

    tracing::info!("IVF header: timebase_den={}", timebase_den);

    let mut units = Vec::new();
    let mut frame_index = 0;
    let mut current_offset = 32u64;

    // Read frames
    loop {
        let frame_start = current_offset;

        // Read frame header (12 bytes)
        let mut frame_header = [0u8; 12];
        match reader.read_exact(&mut frame_header) {
            Ok(_) => {}
            Err(_) if frame_index == 0 => {
                return Err("Failed to read first frame header".to_string())
            }
            Err(_) => break,
        }

        let frame_size = u32::from_le_bytes([
            frame_header[0],
            frame_header[1],
            frame_header[2],
            frame_header[3],
        ]) as usize;
        let pts = u64::from_le_bytes([
            frame_header[4],
            frame_header[5],
            frame_header[6],
            frame_header[7],
            frame_header[8],
            frame_header[9],
            frame_header[10],
            frame_header[11],
        ]);

        // Calculate timestamp in nanoseconds
        let timestamp_ns = if timebase_den > 0 {
            pts * 1_000_000_000 / timebase_den as u64
        } else {
            0
        };

        // Read frame data to determine frame type
        let header_read_size = frame_size.min(100);
        let mut frame_data = vec![0u8; header_read_size];
        reader
            .read_exact(&mut frame_data)
            .map_err(|e| format!("Failed to read frame data: {}", e))?;

        // Skip remaining frame data
        if frame_size > header_read_size {
            reader
                .seek(SeekFrom::Current((frame_size - header_read_size) as i64))
                .ok();
        }

        // Parse OBU header to determine frame type
        let obu_header = frame_data[0];
        let obu_type = (obu_header >> 3) & 0x0F;

        // Skip OBU header to get frame header payload
        let obu_header_size = 1 + ((obu_header & 0x04) != 0) as usize;
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
                FrameType::Inter => "P".to_string(),
                FrameType::BFrame => "B".to_string(),
                FrameType::IntraOnly => "I".to_string(),
                FrameType::Switch => "I".to_string(),
                FrameType::SI => "I".to_string(),
                FrameType::SP => "P".to_string(),
                FrameType::Unknown => "?".to_string(),
            },
            Err(_) => match obu_type {
                6 => "I".to_string(),
                _ => "P".to_string(),
            },
        };

        // Create unit for this frame
        let mut unit = UnitNode::new(stream_id, "FRAME".to_string(), frame_start, frame_size + 12);
        unit.frame_index = Some(frame_index);
        unit.frame_type = Some(std::sync::Arc::from(frame_type_str.as_str()));
        unit.pts = Some(timestamp_ns);
        unit.display_name = std::sync::Arc::from(format!(
            "Frame {} @ 0x{:08X} ({} bytes)",
            frame_index, frame_start, frame_size
        ));

        // Store reference frame indices and QP if available
        if let Ok(fh) = &frame_header {
            // Convert ref_frame_idx from [u8; 3] to Vec<usize>
            if let Some(ref_idx) = fh.ref_frame_idx {
                unit.ref_frames = Some(ref_idx.iter().map(|&x| x as usize).collect());
            }
            if let Some(qp) = fh.base_q_idx {
                unit.qp_avg = Some(qp);
            }
        }

        units.push(unit);

        current_offset += 12 + frame_size as u64;
        frame_index += 1;

        if frame_index >= 10000 {
            break;
        }
    }

    tracing::info!("Parsed {} frames from IVF file", units.len());
    Ok(units)
}
