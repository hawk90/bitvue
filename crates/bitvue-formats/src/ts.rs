//! MPEG-2 Transport Stream (TS) demuxer
//!
//! Parses MPEG-2 TS files to extract AV1 video streams.
//! TS is widely used in broadcasting and streaming (HLS, MPEG-DASH).
//!
//! Reference: ISO/IEC 13818-1 (MPEG-2 Systems)

use bitvue_core::{BitvueError, Result};
use std::collections::HashMap;

/// TS packet size (188 bytes standard)
const TS_PACKET_SIZE: usize = 188;

/// Sync byte for TS packets
const TS_SYNC_BYTE: u8 = 0x47;

/// PAT (Program Association Table) PID
const PAT_PID: u16 = 0x0000;

/// AV1 stream type in PMT
const STREAM_TYPE_AV1: u8 = 0x06; // Private data, need descriptor check

/// H.264/AVC stream type in PMT (ISO/IEC 14496-10)
const STREAM_TYPE_AVC: u8 = 0x1B;

/// H.265/HEVC stream type in PMT (ISO/IEC 23008-2)
const STREAM_TYPE_HEVC: u8 = 0x24;

/// TS packet header
#[derive(Debug, Clone)]
struct TsPacket {
    /// Packet Identifier
    pid: u16,
    /// Payload Unit Start Indicator
    payload_unit_start: bool,
    /// Adaptation field control
    _adaptation_field_control: u8,
    /// Continuity counter
    _continuity_counter: u8,
    /// Payload data
    payload: Vec<u8>,
}

/// Program Association Table entry
#[derive(Debug, Clone)]
struct PatEntry {
    _program_number: u16,
    pmt_pid: u16,
}

/// Program Map Table stream info
#[derive(Debug, Clone)]
struct PmtStream {
    stream_type: u8,
    elementary_pid: u16,
}

/// PES (Packetized Elementary Stream) packet
#[derive(Debug, Clone)]
struct PesPacket {
    _stream_id: u8,
    pts: Option<u64>,
    dts: Option<u64>,
    payload: Vec<u8>,
}

/// A video sample extracted from a TS stream
#[derive(Debug, Clone)]
pub struct VideoSample {
    /// Byte offset in the original data where the NAL unit starts
    pub offset: usize,
    /// Size of the NAL unit in bytes
    pub size: usize,
    /// Presentation timestamp (90 kHz clock)
    pub pts: Option<u64>,
    /// Decode timestamp (90 kHz clock)
    pub dts: Option<u64>,
    /// Whether this is a keyframe (IDR for H.264, IRAP for HEVC)
    pub key_frame: bool,
    /// Raw NAL unit bytes (without Annex B start codes)
    pub data: Vec<u8>,
}

/// TS demuxer information
#[derive(Debug)]
pub struct TsInfo {
    /// Video stream PID
    pub video_pid: Option<u16>,
    /// Number of samples extracted
    pub sample_count: usize,
    /// AV1 samples (OBU data)
    pub samples: Vec<Vec<u8>>,
    /// Presentation timestamps
    pub timestamps: Vec<u64>,
}

/// Check if data is a TS file
pub fn is_ts(data: &[u8]) -> bool {
    if data.len() < TS_PACKET_SIZE {
        return false;
    }

    // Check first sync byte
    if data[0] != TS_SYNC_BYTE {
        return false;
    }

    // Check second packet sync byte (if exists)
    if data.len() >= TS_PACKET_SIZE * 2 {
        return data[TS_PACKET_SIZE] == TS_SYNC_BYTE;
    }

    true
}

/// Parse a single TS packet
fn parse_ts_packet(data: &[u8]) -> Result<TsPacket> {
    if data.len() < 4 {
        return Err(BitvueError::InvalidData("TS packet too short".to_string()));
    }

    // Sync byte
    if data[0] != TS_SYNC_BYTE {
        return Err(BitvueError::InvalidData(format!(
            "Invalid sync byte: 0x{:02X}",
            data[0]
        )));
    }

    // Parse header (4 bytes)
    let byte1 = data[1];
    let byte2 = data[2];
    let byte3 = data[3];

    let payload_unit_start = (byte1 & 0x40) != 0;
    let pid = (((byte1 & 0x1F) as u16) << 8) | (byte2 as u16);
    let adaptation_field_control = (byte3 & 0x30) >> 4;
    let continuity_counter = byte3 & 0x0F;

    // Extract payload
    let mut payload_start: usize = 4;

    // Handle adaptation field
    if adaptation_field_control == 0x02 || adaptation_field_control == 0x03 {
        if data.len() < 5 {
            return Err(BitvueError::InvalidData(
                "Adaptation field length missing".to_string(),
            ));
        }
        let adaptation_length = data[4] as usize;

        // Validate adaptation_length doesn't exceed remaining data
        let remaining_data = data.len().saturating_sub(5);
        if adaptation_length > remaining_data {
            return Err(BitvueError::InvalidData(format!(
                "Adaptation field length {} exceeds remaining data {}",
                adaptation_length, remaining_data
            )));
        }

        // Use checked arithmetic to prevent overflow
        payload_start = match payload_start
            .checked_add(1)
            .and_then(|v| v.checked_add(adaptation_length))
        {
            Some(v) => v,
            None => {
                return Err(BitvueError::InvalidData(
                    "Payload start offset overflow".to_string(),
                ));
            }
        };
    }

    // Extract payload
    let payload = if adaptation_field_control == 0x01 || adaptation_field_control == 0x03 {
        if payload_start < data.len() {
            data[payload_start..].to_vec()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(TsPacket {
        pid,
        payload_unit_start,
        _adaptation_field_control: adaptation_field_control,
        _continuity_counter: continuity_counter,
        payload,
    })
}

/// Parse PAT (Program Association Table)
fn parse_pat(payload: &[u8], pusi: bool) -> Result<Vec<PatEntry>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    // Skip pointer_field (only present when PUSI is set)
    // pointer_field is 1 byte indicating offset to PSI table start
    let mut offset = if pusi {
        let pointer_field = payload[0] as usize;
        1 + pointer_field // Skip pointer_field byte + its value
    } else {
        0
    };

    if offset >= payload.len() {
        return Ok(Vec::new());
    }

    // PSI header
    if payload[offset] != 0x00 {
        // table_id should be 0x00 for PAT
        return Ok(Vec::new());
    }

    offset += 1;
    if offset + 2 > payload.len() {
        return Ok(Vec::new());
    }

    let section_length = (((payload[offset] & 0x0F) as u16) << 8) | (payload[offset + 1] as u16);
    offset += 2;

    // Skip transport_stream_id (2), version (1), section_number (1), last_section_number (1)
    offset += 5;

    let mut entries = Vec::new();
    let end = offset + (section_length as usize) - 9; // -9 for header and CRC

    while offset + 4 <= end {
        let program_number = ((payload[offset] as u16) << 8) | (payload[offset + 1] as u16);
        let pmt_pid = (((payload[offset + 2] & 0x1F) as u16) << 8) | (payload[offset + 3] as u16);

        if program_number != 0 {
            // Skip network PID (program_number == 0)
            entries.push(PatEntry {
                _program_number: program_number,
                pmt_pid,
            });
        }

        offset += 4;
    }

    Ok(entries)
}

/// Parse PMT (Program Map Table)
fn parse_pmt(payload: &[u8], pusi: bool) -> Result<Vec<PmtStream>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    // Skip pointer_field (only present when PUSI is set)
    // pointer_field is 1 byte indicating offset to PSI table start
    let mut offset = if pusi {
        let pointer_field = payload[0] as usize;
        1 + pointer_field // Skip pointer_field byte + its value
    } else {
        0
    };

    if offset >= payload.len() {
        return Ok(Vec::new());
    }

    // PMT table_id should be 0x02
    if payload[offset] != 0x02 {
        return Ok(Vec::new());
    }

    offset += 1;
    if offset + 2 > payload.len() {
        return Ok(Vec::new());
    }

    let section_length = (((payload[offset] & 0x0F) as u16) << 8) | (payload[offset + 1] as u16);
    offset += 2;

    // Skip program_number (2), version (1), section_number (1), last_section_number (1)
    offset = match offset.checked_add(5) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    if offset + 2 > payload.len() {
        return Ok(Vec::new());
    }

    let program_info_length =
        (((payload[offset] & 0x0F) as u16) << 8) | (payload[offset + 1] as u16);
    // Use checked arithmetic to prevent overflow
    offset = match offset
        .checked_add(2)
        .and_then(|v| v.checked_add(program_info_length as usize))
    {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let mut streams = Vec::new();
    let end = offset + (section_length as usize) - 9 - 4 - (program_info_length as usize);

    while offset + 5 <= end && offset < payload.len() {
        if offset + 5 > payload.len() {
            break;
        }

        let stream_type = payload[offset];
        let elementary_pid =
            (((payload[offset + 1] & 0x1F) as u16) << 8) | (payload[offset + 2] as u16);
        let es_info_length =
            (((payload[offset + 3] & 0x0F) as u16) << 8) | (payload[offset + 4] as u16);

        streams.push(PmtStream {
            stream_type,
            elementary_pid,
        });

        // Use checked arithmetic to prevent overflow
        offset = match offset
            .checked_add(5)
            .and_then(|v| v.checked_add(es_info_length as usize))
        {
            Some(v) => v,
            None => break,
        };
    }

    Ok(streams)
}

/// Parse PES packet
fn parse_pes(data: &[u8]) -> Result<PesPacket> {
    if data.len() < 6 {
        return Err(BitvueError::InvalidData("PES too short".to_string()));
    }

    // Check PES start code (0x000001)
    if data[0] != 0x00 || data[1] != 0x00 || data[2] != 0x01 {
        return Err(BitvueError::InvalidData(
            "Invalid PES start code".to_string(),
        ));
    }

    let stream_id = data[3];
    let _pes_packet_length = ((data[4] as u16) << 8) | (data[5] as u16);

    let mut offset = 6;
    let mut pts = None;
    let mut dts = None;

    // Parse optional PES header (for video/audio streams)
    if stream_id != 0xBC
        && stream_id != 0xBF
        && stream_id != 0xF0
        && stream_id != 0xF1
        && stream_id != 0xFF
        && stream_id != 0xF2
        && stream_id != 0xF8
    {
        if data.len() < offset + 3 {
            return Err(BitvueError::InvalidData("PES header too short".to_string()));
        }

        let pts_dts_flags = (data[offset + 1] & 0xC0) >> 6;
        let pes_header_length = data[offset + 2] as usize;
        offset += 3;

        // Parse PTS/DTS
        if pts_dts_flags == 0x02 || pts_dts_flags == 0x03 {
            // PTS present
            if data.len() >= offset + 5 {
                pts = Some(parse_timestamp(&data[offset..offset + 5]));
                offset += 5;
            }
        }

        if pts_dts_flags == 0x03 {
            // DTS present
            if data.len() >= offset + 5 {
                dts = Some(parse_timestamp(&data[offset..offset + 5]));
            }
        }

        // Skip remaining header
        offset = 9 + pes_header_length;
    }

    let payload = if offset < data.len() {
        data[offset..].to_vec()
    } else {
        Vec::new()
    };

    Ok(PesPacket {
        _stream_id: stream_id,
        pts,
        dts,
        payload,
    })
}

/// Parse PTS/DTS timestamp (33 bits)
fn parse_timestamp(data: &[u8]) -> u64 {
    (((data[0] & 0x0E) as u64) << 29)
        | ((data[1] as u64) << 22)
        | (((data[2] & 0xFE) as u64) << 14)
        | ((data[3] as u64) << 7)
        | ((data[4] as u64) >> 1)
}

/// Split an Annex B byte sequence into individual NAL units.
///
/// Scans for 3-byte (0x000001) and 4-byte (0x00000001) start codes and returns
/// the raw NAL unit payload (start code stripped) for each unit found.
fn split_annex_b_nal_units(data: &[u8]) -> Vec<Vec<u8>> {
    let mut nal_units = Vec::new();
    let mut i = 0;

    // Locate all start code positions
    let mut starts: Vec<(usize, usize)> = Vec::new(); // (payload_start, start_code_len)
    while i < data.len() {
        if i + 3 <= data.len() && data[i] == 0x00 && data[i + 1] == 0x00 {
            if i + 4 <= data.len() && data[i + 2] == 0x00 && data[i + 3] == 0x01 {
                starts.push((i + 4, 4));
                i += 4;
                continue;
            } else if data[i + 2] == 0x01 {
                starts.push((i + 3, 3));
                i += 3;
                continue;
            }
        }
        i += 1;
    }

    // Extract NAL unit bytes between consecutive start codes
    for (idx, &(payload_start, _)) in starts.iter().enumerate() {
        let payload_end = if idx + 1 < starts.len() {
            // The next start code begins sc_len bytes before its payload_start;
            // use the minimum of both to trim trailing zeros of the start code prefix.
            let next_payload = starts[idx + 1].0;
            let next_sc_len = starts[idx + 1].1;
            next_payload.saturating_sub(next_sc_len)
        } else {
            data.len()
        };

        if payload_end > payload_start {
            nal_units.push(data[payload_start..payload_end].to_vec());
        }
    }

    nal_units
}

/// Determine whether an H.264 NAL unit represents a keyframe (IDR slice).
///
/// Per ITU-T H.264 Table 7-1, nal_unit_type 5 is an IDR slice.
fn is_avc_keyframe(nal_data: &[u8]) -> bool {
    if nal_data.is_empty() {
        return false;
    }
    let nal_type = nal_data[0] & 0x1F;
    nal_type == 5 // IDR slice
}

/// Determine whether an H.265 NAL unit represents a keyframe (IRAP).
///
/// Per ITU-T H.265 Table 7-1, nal_unit_type values 16–23 are IRAP pictures
/// (BLA, IDR, CRA). Bit layout: nal_unit_type occupies bits [9:4] of the
/// 2-byte NAL header.
fn is_hevc_keyframe(nal_data: &[u8]) -> bool {
    if nal_data.len() < 2 {
        return false;
    }
    // HEVC NAL header: forbidden_zero_bit(1) | nal_unit_type(6) | nuh_layer_id(6) | nuh_temporal_id_plus1(3)
    let nal_type = (nal_data[0] >> 1) & 0x3F;
    // IRAP types: BLA_W_LP=16..CRA_NUT=21, RSV_IRAP_VCL22=22, RSV_IRAP_VCL23=23
    (16..=23).contains(&nal_type)
}

/// Extract PES packets and convert them to VideoSamples for a specific PID.
///
/// Reassembles fragmented TS payloads into complete PES packets, strips the
/// PES header to obtain the elementary stream payload, then splits on Annex B
/// start codes to produce individual VideoSample entries.
fn extract_video_samples_for_pid<F>(
    data: &[u8],
    video_pid: u16,
    is_keyframe_fn: F,
) -> Result<Vec<VideoSample>>
where
    F: Fn(&[u8]) -> bool,
{
    let mut pes_buffers: HashMap<u16, (Vec<u8>, Option<u64>, Option<u64>)> = HashMap::new();
    let mut samples: Vec<VideoSample> = Vec::new();
    let mut byte_offset: usize = 0;

    let mut offset = 0;
    while offset + TS_PACKET_SIZE <= data.len() {
        let packet_data = &data[offset..offset + TS_PACKET_SIZE];
        let packet = parse_ts_packet(packet_data)?;

        if packet.pid == video_pid {
            if packet.payload_unit_start {
                // Flush any previously accumulated PES buffer
                if let Some((buffer, pts, dts)) = pes_buffers.remove(&video_pid) {
                    let nal_units = split_annex_b_nal_units(&buffer);
                    for nal in nal_units {
                        if nal.is_empty() {
                            continue;
                        }
                        let key_frame = is_keyframe_fn(&nal);
                        let size = nal.len();
                        samples.push(VideoSample {
                            offset: byte_offset,
                            size,
                            pts,
                            dts,
                            key_frame,
                            data: nal,
                        });
                        byte_offset += size;
                    }
                }

                // Start a new PES packet: parse its header to extract PTS/DTS
                if let Ok(pes) = parse_pes(&packet.payload) {
                    pes_buffers.insert(video_pid, (pes.payload, pes.pts, pes.dts));
                }
            } else if let Some((buf, _, _)) = pes_buffers.get_mut(&video_pid) {
                buf.extend_from_slice(&packet.payload);
            }
        }

        offset += TS_PACKET_SIZE;
    }

    // Flush the final PES buffer
    if let Some((buffer, pts, dts)) = pes_buffers.remove(&video_pid) {
        let nal_units = split_annex_b_nal_units(&buffer);
        for nal in nal_units {
            if nal.is_empty() {
                continue;
            }
            let key_frame = is_keyframe_fn(&nal);
            let size = nal.len();
            samples.push(VideoSample {
                offset: byte_offset,
                size,
                pts,
                dts,
                key_frame,
                data: nal,
            });
            byte_offset += size;
        }
    }

    Ok(samples)
}

/// Find the video PID in PMT streams matching a specific stream type.
fn find_pid_by_stream_type(pmt_streams: &[PmtStream], stream_type: u8) -> Option<u16> {
    pmt_streams
        .iter()
        .find(|s| s.stream_type == stream_type)
        .map(|s| s.elementary_pid)
}

/// Extract AV1 samples from TS file
pub fn extract_av1_samples(data: &[u8]) -> Result<Vec<Vec<u8>>> {
    let info = parse_ts(data)?;
    Ok(info.samples)
}

/// Extract H.264/AVC samples from TS file.
///
/// Parses the PAT and PMT to locate the H.264 video PID (stream type 0x1B),
/// reassembles PES packets from that PID, and extracts individual NAL units
/// by splitting on Annex B start codes (0x000001 / 0x00000001).
///
/// Returns a `Vec<Vec<u8>>` where each inner Vec is one raw NAL unit payload
/// with start codes stripped.
pub fn extract_avc_samples(data: &[u8]) -> Result<Vec<Vec<u8>>> {
    let (_pat_entries, pmt_streams) = extract_pat_pmt(data)?;

    let video_pid = match find_pid_by_stream_type(&pmt_streams, STREAM_TYPE_AVC) {
        Some(pid) => pid,
        None => {
            // No H.264 stream found in PMT; return empty rather than an error so
            // callers can distinguish "no stream" from "parse failure".
            return Ok(Vec::new());
        }
    };

    let samples = extract_video_samples_for_pid(data, video_pid, is_avc_keyframe)?;
    Ok(samples.into_iter().map(|s| s.data).collect())
}

/// Extract H.265/HEVC samples from TS file.
///
/// Parses the PAT and PMT to locate the HEVC video PID (stream type 0x24),
/// reassembles PES packets from that PID, and extracts individual NAL units
/// by splitting on Annex B start codes (0x000001 / 0x00000001).
///
/// Returns a `Vec<Vec<u8>>` where each inner Vec is one raw NAL unit payload
/// with start codes stripped.
pub fn extract_hevc_samples(data: &[u8]) -> Result<Vec<Vec<u8>>> {
    let (_pat_entries, pmt_streams) = extract_pat_pmt(data)?;

    let video_pid = match find_pid_by_stream_type(&pmt_streams, STREAM_TYPE_HEVC) {
        Some(pid) => pid,
        None => {
            // No HEVC stream found in PMT; return empty rather than an error.
            return Ok(Vec::new());
        }
    };

    let samples = extract_video_samples_for_pid(data, video_pid, is_hevc_keyframe)?;
    Ok(samples.into_iter().map(|s| s.data).collect())
}

/// Extract H.264/AVC video samples as structured VideoSample records.
///
/// Like `extract_avc_samples` but returns full `VideoSample` structs with
/// offset, size, PTS, DTS, and keyframe flag instead of just raw bytes.
pub fn extract_avc_video_samples(data: &[u8]) -> Result<Vec<VideoSample>> {
    let (_pat_entries, pmt_streams) = extract_pat_pmt(data)?;

    let video_pid = match find_pid_by_stream_type(&pmt_streams, STREAM_TYPE_AVC) {
        Some(pid) => pid,
        None => return Ok(Vec::new()),
    };

    extract_video_samples_for_pid(data, video_pid, is_avc_keyframe)
}

/// Extract H.265/HEVC video samples as structured VideoSample records.
///
/// Like `extract_hevc_samples` but returns full `VideoSample` structs with
/// offset, size, PTS, DTS, and keyframe flag instead of just raw bytes.
pub fn extract_hevc_video_samples(data: &[u8]) -> Result<Vec<VideoSample>> {
    let (_pat_entries, pmt_streams) = extract_pat_pmt(data)?;

    let video_pid = match find_pid_by_stream_type(&pmt_streams, STREAM_TYPE_HEVC) {
        Some(pid) => pid,
        None => return Ok(Vec::new()),
    };

    extract_video_samples_for_pid(data, video_pid, is_hevc_keyframe)
}

/// Extract PAT and PMT entries from TS data
///
/// First pass through TS packets to find Program Association Table (PAT)
/// and Program Map Table (PMT) which contain stream mapping information.
fn extract_pat_pmt(data: &[u8]) -> Result<(Vec<PatEntry>, Vec<PmtStream>)> {
    let mut pat_entries = Vec::new();
    let mut pmt_streams = Vec::new();

    let mut offset = 0;
    while offset + TS_PACKET_SIZE <= data.len() {
        let packet_data = &data[offset..offset + TS_PACKET_SIZE];
        let packet = parse_ts_packet(packet_data)?;

        // Extract PAT (Program Association Table)
        if packet.pid == PAT_PID && !packet.payload.is_empty() {
            pat_entries = parse_pat(&packet.payload, packet.payload_unit_start)?;
        }
        // Extract PMT (Program Map Table) using PAT entries
        else if !pat_entries.is_empty() {
            for pat in &pat_entries {
                if packet.pid == pat.pmt_pid && !packet.payload.is_empty() {
                    pmt_streams = parse_pmt(&packet.payload, packet.payload_unit_start)?;
                    break;
                }
            }
        }

        offset += TS_PACKET_SIZE;
    }

    Ok((pat_entries, pmt_streams))
}

/// Find the video stream PID from PMT streams
///
/// Searches through PMT streams to find AV1 video stream.
/// Returns None if no AV1 stream is found.
fn find_video_pid(pmt_streams: &[PmtStream]) -> Option<u16> {
    pmt_streams
        .iter()
        .find(|stream| stream.stream_type == STREAM_TYPE_AV1)
        .map(|stream| stream.elementary_pid)
}

/// Extract PES packets from TS data for a specific PID
///
/// Second pass through TS packets to extract PES (Packetized Elementary Stream)
/// packets for the specified video PID.
fn extract_pes_packets(data: &[u8], video_pid: u16) -> Result<(Vec<Vec<u8>>, Vec<u64>)> {
    let mut pes_buffers: HashMap<u16, Vec<u8>> = HashMap::new();
    let mut samples = Vec::new();
    let mut timestamps = Vec::new();

    let mut offset = 0;
    while offset + TS_PACKET_SIZE <= data.len() {
        let packet_data = &data[offset..offset + TS_PACKET_SIZE];
        let packet = parse_ts_packet(packet_data)?;

        if packet.pid == video_pid {
            if packet.payload_unit_start {
                // New PES packet starts - flush previous buffer
                if let Some(buffer) = pes_buffers.remove(&video_pid) {
                    if let Ok(pes) = parse_pes(&buffer) {
                        samples.push(pes.payload);
                        timestamps.push(pes.pts.unwrap_or(0));
                    }
                }
                pes_buffers.insert(video_pid, packet.payload);
            } else {
                // Continue current PES packet
                pes_buffers
                    .entry(video_pid)
                    .or_default()
                    .extend_from_slice(&packet.payload);
            }
        }

        offset += TS_PACKET_SIZE;
    }

    // Process remaining buffer
    if let Some(buffer) = pes_buffers.remove(&video_pid) {
        if let Ok(pes) = parse_pes(&buffer) {
            samples.push(pes.payload);
            timestamps.push(pes.pts.unwrap_or(0));
        }
    }

    Ok((samples, timestamps))
}

/// Parse TS file and extract AV1 video stream
pub fn parse_ts(data: &[u8]) -> Result<TsInfo> {
    // First pass: extract PAT and PMT
    let (_pat_entries, pmt_streams) = extract_pat_pmt(data)?;

    // Find AV1 video stream PID
    let video_pid = match find_video_pid(&pmt_streams) {
        Some(pid) => pid,
        None => {
            return Ok(TsInfo {
                video_pid: None,
                sample_count: 0,
                samples: Vec::new(),
                timestamps: Vec::new(),
            });
        }
    };

    // Second pass: extract PES packets
    let (samples, timestamps) = extract_pes_packets(data, video_pid)?;

    Ok(TsInfo {
        video_pid: Some(video_pid),
        sample_count: samples.len(),
        samples,
        timestamps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ts() {
        // Valid TS file (two sync bytes)
        let mut data = vec![0x47; TS_PACKET_SIZE * 2];
        data[TS_PACKET_SIZE] = 0x47;
        assert!(is_ts(&data));

        // Invalid - wrong sync byte
        let data = vec![0x48; TS_PACKET_SIZE];
        assert!(!is_ts(&data));

        // Too short
        assert!(!is_ts(&[0x47]));
    }

    #[test]
    fn test_parse_ts_packet() {
        // Minimal valid TS packet
        let mut data = vec![0; TS_PACKET_SIZE];
        data[0] = 0x47; // sync byte
        data[1] = 0x40; // PUSI = 1, PID high = 0
        data[2] = 0x00; // PID low = 0 (PAT)
        data[3] = 0x10; // Adaptation = 01 (payload only), CC = 0

        let packet = parse_ts_packet(&data).unwrap();
        assert_eq!(packet.pid, 0);
        assert!(packet.payload_unit_start);
    }

    #[test]
    fn test_parse_timestamp() {
        // Example PTS bytes with actual timestamp bits set
        // Format: 4 bits marker + 3 bits timestamp[32:30] + 1 bit marker
        //         16 bits timestamp[29:15] + 1 bit marker
        //         15 bits timestamp[14:0] + 1 bit marker
        let data = [0x21, 0x00, 0x01, 0x00, 0x03]; // Last byte 0x03 gives ts = 1
        let ts = parse_timestamp(&data);
        assert_eq!(ts, 1);

        // Test with a larger timestamp
        let data2 = [0x2F, 0xFF, 0xFF, 0xFF, 0xFF]; // Maximum values in relevant bits
        let ts2 = parse_timestamp(&data2);
        assert!(ts2 > 0);
    }

    #[test]
    fn test_parse_ts_packet_invalid() {
        // Too short
        let data = [0x47, 0x40, 0x00];
        assert!(parse_ts_packet(&data).is_err());

        // Invalid sync byte
        let data = vec![0xFF; TS_PACKET_SIZE];
        assert!(parse_ts_packet(&data).is_err());

        // Empty data
        assert!(parse_ts_packet(&[]).is_err());
    }

    #[test]
    fn test_parse_ts_packet_with_adaptation() {
        // TS packet with adaptation field
        let mut data = vec![0; TS_PACKET_SIZE];
        data[0] = 0x47; // sync byte
        data[1] = 0x00; // PUSI = 0, PID high = 0
        data[2] = 0x00; // PID low = 0
        data[3] = 0x30; // Adaptation = 11 (both adaptation and payload), CC = 0
        data[4] = 10; // Adaptation field length = 10

        let packet = parse_ts_packet(&data).unwrap();
        assert_eq!(packet.pid, 0);
        assert!(!packet.payload_unit_start);
        // Payload should start after header(4) + adaptation_length_field(1) + adaptation(10) = 15
        assert!(packet.payload.len() < TS_PACKET_SIZE);
    }

    #[test]
    fn test_parse_pes_invalid() {
        // Too short
        let data = [0x00, 0x00, 0x01];
        assert!(parse_pes(&data).is_err());

        // Invalid start code
        let data = [0xFF, 0xFF, 0xFF, 0xE0, 0x00, 0x10];
        assert!(parse_pes(&data).is_err());

        // Empty data
        assert!(parse_pes(&[]).is_err());
    }

    #[test]
    fn test_parse_pes_basic() {
        // Minimal valid PES packet
        let data = [
            0x00, 0x00, 0x01, // Start code
            0xE0, // Stream ID (video)
            0x00, 0x00, // PES packet length
            0x80, 0x00, 0x00, // Optional header (no PTS/DTS)
        ];

        let pes = parse_pes(&data).unwrap();
        assert_eq!(pes.pts, None);
        assert!(pes.payload.is_empty());
    }

    #[test]
    fn test_parse_pat_empty() {
        // Empty payload
        let result = parse_pat(&[], false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        // Invalid table ID (with PUSI set, pointer_field = 0)
        let data = [0x00, 0xFF, 0x00]; // pointer_field=0, wrong table_id
        let result = parse_pat(&data, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_pmt_empty() {
        // Empty payload
        let result = parse_pmt(&[], false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        // Invalid table ID (with PUSI set, pointer_field = 0)
        let data = [0x00, 0x00, 0x00]; // pointer_field=0, wrong table_id (should be 0x02)
        let result = parse_pmt(&data, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_ts_info() {
        // Test empty TS info
        let info = TsInfo {
            video_pid: None,
            sample_count: 0,
            samples: Vec::new(),
            timestamps: Vec::new(),
        };
        assert_eq!(info.video_pid, None);
        assert_eq!(info.sample_count, 0);
    }

    #[test]
    fn test_parse_ts_empty() {
        // Empty data should fail
        let result = parse_ts(&[]);
        assert!(result.is_ok()); // Returns empty info, not error

        let info = result.unwrap();
        assert_eq!(info.video_pid, None);
        assert_eq!(info.sample_count, 0);
    }

    #[test]
    fn test_extract_av1_samples_empty() {
        let result = extract_av1_samples(&[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_extract_avc_samples_empty() {
        // Empty data: should succeed with no samples (no PMT, no AVC PID)
        let result = extract_avc_samples(&[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_extract_hevc_samples_empty() {
        // Empty data: should succeed with no samples (no PMT, no HEVC PID)
        let result = extract_hevc_samples(&[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_split_annex_b_empty() {
        let result = split_annex_b_nal_units(&[]);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_split_annex_b_3byte_start_code() {
        // Two NAL units separated by 3-byte start codes
        let data = [
            0x00, 0x00, 0x01, 0x67, 0xAA, 0xBB, // SPS NAL (type 7)
            0x00, 0x00, 0x01, 0x65, 0xCC, 0xDD, // IDR NAL (type 5)
        ];
        let nal_units = split_annex_b_nal_units(&data);
        assert_eq!(nal_units.len(), 2);
        assert_eq!(nal_units[0], vec![0x67, 0xAA, 0xBB]);
        assert_eq!(nal_units[1], vec![0x65, 0xCC, 0xDD]);
    }

    #[test]
    fn test_split_annex_b_4byte_start_code() {
        // Two NAL units separated by 4-byte start codes
        let data = [
            0x00, 0x00, 0x00, 0x01, 0x67, 0xAA, // SPS NAL
            0x00, 0x00, 0x00, 0x01, 0x65, 0xBB, // IDR NAL
        ];
        let nal_units = split_annex_b_nal_units(&data);
        assert_eq!(nal_units.len(), 2);
        assert_eq!(nal_units[0], vec![0x67, 0xAA]);
        assert_eq!(nal_units[1], vec![0x65, 0xBB]);
    }

    #[test]
    fn test_is_avc_keyframe() {
        // IDR slice (type 5) is keyframe
        assert!(is_avc_keyframe(&[0x65, 0x00])); // 0x65 & 0x1F = 5
                                                 // Non-IDR slice (type 1) is not a keyframe
        assert!(!is_avc_keyframe(&[0x41, 0x00])); // 0x41 & 0x1F = 1
                                                  // SPS (type 7) is not a keyframe
        assert!(!is_avc_keyframe(&[0x67, 0x00])); // 0x67 & 0x1F = 7
                                                  // Empty is not keyframe
        assert!(!is_avc_keyframe(&[]));
    }

    #[test]
    fn test_is_hevc_keyframe() {
        // IDR_W_RADL (type 19 = 0x13): nal_unit_type in bits [6:1] of first byte
        // First byte = (19 << 1) = 0x26
        let idr_nal = [0x26, 0x01];
        assert!(is_hevc_keyframe(&idr_nal));

        // CRA_NUT (type 21 = 0x15): first byte = (21 << 1) = 0x2A
        let cra_nal = [0x2A, 0x01];
        assert!(is_hevc_keyframe(&cra_nal));

        // TRAIL_R (type 1): first byte = (1 << 1) = 0x02 - not keyframe
        let trail_nal = [0x02, 0x01];
        assert!(!is_hevc_keyframe(&trail_nal));

        // Empty is not keyframe
        assert!(!is_hevc_keyframe(&[]));
    }
}
