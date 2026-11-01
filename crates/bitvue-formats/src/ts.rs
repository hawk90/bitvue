//! MPEG-2 Transport Stream (TS) demuxer
//!
//! Parses MPEG-2 TS files to extract AV1 video streams.
//! TS is widely used in broadcasting and streaming (HLS, MPEG-DASH).
//!
//! Reference: ISO/IEC 13818-1 (MPEG-2 Systems)

use bitvue_engine::{BitvueError, Result};
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
    /// Byte offset of this sample in the concatenation of all samples of the stream
    pub offset: usize,
    /// Size of the sample in bytes
    pub size: usize,
    /// Presentation timestamp (90 kHz clock)
    pub pts: Option<u64>,
    /// Decode timestamp (90 kHz clock)
    pub dts: Option<u64>,
    /// Whether this is a keyframe (IDR for H.264, IRAP for HEVC)
    pub key_frame: bool,
    /// Sample bytes: the whole access unit in Annex B form (4-byte start codes) when it comes from
    /// `extract_*_video_samples`
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

    let section_length =
        (((payload[offset] & 0x0F) as usize) << 8) | (payload[offset + 1] as usize);
    offset += 2;
    // section_length counts the bytes after its own field, CRC_32 included.
    let section_end = offset
        .saturating_add(section_length)
        .saturating_sub(4) // CRC_32
        .min(payload.len());

    // Skip transport_stream_id (2), version (1), section_number (1), last_section_number (1)
    offset += 5;

    let mut entries = Vec::new();

    while offset + 4 <= section_end {
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

    let section_length =
        (((payload[offset] & 0x0F) as usize) << 8) | (payload[offset + 1] as usize);
    offset += 2;
    // section_length counts the bytes after its own field, CRC_32 included.
    let section_end = offset
        .saturating_add(section_length)
        .saturating_sub(4) // CRC_32
        .min(payload.len());

    // Skip program_number (2), version (1), section_number (1), last_section_number (1), then
    // PCR_PID (2) -- ISO/IEC 13818-1 puts it before program_info_length (#261).
    offset = match offset.checked_add(5 + 2) {
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
    let end = section_end;

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
    Ok(extract_nals_for_pid(data, video_pid, is_keyframe_fn)?
        .into_iter()
        .map(|(_, nal)| nal)
        .collect())
}

/// Like `extract_video_samples_for_pid`, with each NAL tagged by the index of the PES packet it
/// came from (what `group_access_units` needs).
fn extract_nals_for_pid<F>(
    data: &[u8],
    video_pid: u16,
    is_keyframe_fn: F,
) -> Result<Vec<(usize, VideoSample)>>
where
    F: Fn(&[u8]) -> bool,
{
    // (payload bytes, PTS, DTS)
    type PesBuffer = (Vec<u8>, Option<u64>, Option<u64>);
    let mut pes_buffers: HashMap<u16, PesBuffer> = HashMap::new();
    let mut samples: Vec<(usize, VideoSample)> = Vec::new();
    let mut pes_index: usize = 0;
    let mut byte_offset: usize = 0;

    let mut offset = 0;
    while offset + TS_PACKET_SIZE <= data.len() {
        let packet_data = &data[offset..offset + TS_PACKET_SIZE];
        let packet = parse_ts_packet(packet_data)?;

        if packet.pid == video_pid {
            if packet.payload_unit_start {
                // Flush any previously accumulated PES buffer
                if let Some((buffer, pts, dts)) = pes_buffers.remove(&video_pid) {
                    flush_pes(
                        &buffer,
                        (pts, dts),
                        pes_index,
                        &is_keyframe_fn,
                        &mut byte_offset,
                        &mut samples,
                    );
                    pes_index += 1;
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
        flush_pes(
            &buffer,
            (pts, dts),
            pes_index,
            &is_keyframe_fn,
            &mut byte_offset,
            &mut samples,
        );
    }

    Ok(samples)
}

/// Splits one finished PES payload into its Annex B NAL units and appends one `VideoSample` per
/// non-empty NAL (all sharing the PES packet's PTS/DTS), advancing `byte_offset`.
fn flush_pes(
    buffer: &[u8],
    (pts, dts): (Option<u64>, Option<u64>),
    pes_index: usize,
    is_keyframe_fn: &impl Fn(&[u8]) -> bool,
    byte_offset: &mut usize,
    samples: &mut Vec<(usize, VideoSample)>,
) {
    for nal in split_annex_b_nal_units(buffer) {
        if nal.is_empty() {
            continue;
        }
        let key_frame = is_keyframe_fn(&nal);
        let size = nal.len();
        samples.push((
            pes_index,
            VideoSample {
                offset: *byte_offset,
                size,
                pts,
                dts,
                key_frame,
                data: nal,
            },
        ));
        *byte_offset += size;
    }
}

/// Join the NAL units of each access unit (one coded picture) into one sample.
///
/// A new access unit starts at every PES packet and, inside one PES packet, at an access unit
/// delimiter (`is_aud`). The sample carries that unit's NALs in Annex B form (a 4-byte start code
/// before each), the PES packet's PTS/DTS, and is a key frame if any of its NALs is. `offset` is the
/// position in the concatenation of all samples.
fn group_access_units(
    nals: Vec<(usize, VideoSample)>,
    is_aud: impl Fn(&[u8]) -> bool,
) -> Vec<VideoSample> {
    const START_CODE: [u8; 4] = [0, 0, 0, 1];
    let mut units: Vec<VideoSample> = Vec::new();
    let mut current_pes = None;

    for (pes, nal) in nals {
        let starts_unit = current_pes != Some(pes) || (is_aud(&nal.data) && !units.is_empty());
        if starts_unit {
            current_pes = Some(pes);
            let offset = units.last().map_or(0, |u| u.offset + u.size);
            units.push(VideoSample {
                offset,
                size: 0,
                pts: nal.pts,
                dts: nal.dts,
                key_frame: false,
                data: Vec::new(),
            });
        }
        let unit = units.last_mut().expect("a unit was started above");
        unit.key_frame |= nal.key_frame;
        unit.data.extend_from_slice(&START_CODE);
        unit.data.extend_from_slice(&nal.data);
        unit.size = unit.data.len();
    }
    units
}

/// H.264 access unit delimiter (nal_unit_type 9).
fn is_avc_aud(nal: &[u8]) -> bool {
    nal.first().is_some_and(|b| b & 0x1F == 9)
}

/// H.265 access unit delimiter (nal_unit_type 35).
fn is_hevc_aud(nal: &[u8]) -> bool {
    nal.first().is_some_and(|b| (b >> 1) & 0x3F == 35)
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

/// Extract H.264/AVC video samples: one `VideoSample` per access unit (coded picture).
///
/// Unlike `extract_avc_samples`, which lists NAL units, each sample holds all NAL units of one
/// picture (Annex B, 4-byte start codes) with its PTS/DTS and key-frame flag, so
/// `result.len()` is the frame count.
pub fn extract_avc_video_samples(data: &[u8]) -> Result<Vec<VideoSample>> {
    let (_pat_entries, pmt_streams) = extract_pat_pmt(data)?;

    let video_pid = match find_pid_by_stream_type(&pmt_streams, STREAM_TYPE_AVC) {
        Some(pid) => pid,
        None => return Ok(Vec::new()),
    };

    let nals = extract_nals_for_pid(data, video_pid, is_avc_keyframe)?;
    Ok(group_access_units(nals, is_avc_aud))
}

/// Extract H.265/HEVC video samples: one `VideoSample` per access unit (coded picture).
///
/// Unlike `extract_hevc_samples`, which lists NAL units, each sample holds all NAL units of one
/// picture (Annex B, 4-byte start codes) with its PTS/DTS and key-frame flag, so
/// `result.len()` is the frame count.
pub fn extract_hevc_video_samples(data: &[u8]) -> Result<Vec<VideoSample>> {
    let (_pat_entries, pmt_streams) = extract_pat_pmt(data)?;

    let video_pid = match find_pid_by_stream_type(&pmt_streams, STREAM_TYPE_HEVC) {
        Some(pid) => pid,
        None => return Ok(Vec::new()),
    };

    let nals = extract_nals_for_pid(data, video_pid, is_hevc_keyframe)?;
    Ok(group_access_units(nals, is_hevc_aud))
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

/// Stream-level tests: build TS packets / PSI sections / PES packets by hand and run the demux paths on them.
#[cfg(test)]
mod stream_tests {
    use super::*;

    const VIDEO_PID: u16 = 0x101;
    const PMT_PID: u16 = 0x1000;

    /// One 188-byte TS packet; short payloads are padded with an adaptation-field stuffing run, as muxers do.
    fn packet(pid: u16, pusi: bool, payload: &[u8]) -> Vec<u8> {
        assert!(payload.len() <= 184);
        let mut p = vec![
            TS_SYNC_BYTE,
            ((pusi as u8) << 6) | (pid >> 8) as u8,
            pid as u8,
        ];
        if payload.len() == 184 {
            p.push(0x10);
        } else {
            p.push(0x30);
            let stuffing = 183 - payload.len();
            p.push(stuffing as u8);
            if stuffing > 0 {
                p.push(0x00);
                p.extend(vec![0xFF; stuffing - 1]);
            }
        }
        p.extend_from_slice(payload);
        assert_eq!(p.len(), TS_PACKET_SIZE);
        p
    }

    /// PAT section with a leading zero pointer_field (PUSI form).
    fn pat(programs: &[(u16, u16)]) -> Vec<u8> {
        let section_length = 9 + 4 * programs.len();
        let mut s = vec![
            0x00,
            0x00,
            0xB0 | (section_length >> 8) as u8,
            section_length as u8,
            0x00,
            0x01,
            0xC1,
            0x00,
            0x00,
        ];
        for &(number, pid) in programs {
            s.extend_from_slice(&[(number >> 8) as u8, number as u8]);
            s.extend_from_slice(&[0xE0 | (pid >> 8) as u8, pid as u8]);
        }
        s.extend_from_slice(&[0, 0, 0, 0]); // CRC (not checked)
        s
    }

    /// `(stream_type, pid, es_info bytes)`
    type Es<'a> = (u8, u16, &'a [u8]);

    /// PMT section (PUSI form) in the ISO/IEC 13818-1 layout: a 2-byte PCR_PID precedes program_info_length.
    fn pmt(program_info: &[u8], streams: &[Es]) -> Vec<u8> {
        let mut body = Vec::new();
        for &(stream_type, pid, es_info) in streams {
            body.push(stream_type);
            body.extend_from_slice(&[0xE0 | (pid >> 8) as u8, pid as u8]);
            body.extend_from_slice(&[0xF0 | (es_info.len() >> 8) as u8, es_info.len() as u8]);
            body.extend_from_slice(es_info);
        }
        let section_length = 13 + program_info.len() + body.len();
        let mut s = vec![
            0x00,
            0x02,
            0xB0 | (section_length >> 8) as u8,
            section_length as u8,
            0x00,
            0x01,
            0xC1,
            0x00,
            0x00,
        ];
        s.extend_from_slice(&[0xE0 | (VIDEO_PID >> 8) as u8, VIDEO_PID as u8]);
        s.extend_from_slice(&[
            0xF0 | (program_info.len() >> 8) as u8,
            program_info.len() as u8,
        ]);
        s.extend_from_slice(program_info);
        s.extend_from_slice(&body);
        s.extend_from_slice(&[0, 0, 0, 0]); // CRC (not checked)
        s
    }

    fn timestamp(prefix: u8, t: u64) -> [u8; 5] {
        [
            (prefix << 4) | (((t >> 30) & 0x07) as u8) << 1 | 1,
            (t >> 22) as u8,
            ((((t >> 15) & 0x7F) as u8) << 1) | 1,
            (t >> 7) as u8,
            (((t & 0x7F) as u8) << 1) | 1,
        ]
    }

    /// PES packet; the optional header carries PTS (and DTS) like a real muxer writes it.
    fn pes(stream_id: u8, pts: Option<u64>, dts: Option<u64>, es: &[u8]) -> Vec<u8> {
        let mut ts = Vec::new();
        let flags = match (pts, dts) {
            (Some(p), Some(d)) => {
                ts.extend_from_slice(&timestamp(0x3, p));
                ts.extend_from_slice(&timestamp(0x1, d));
                0xC0
            }
            (Some(p), None) => {
                ts.extend_from_slice(&timestamp(0x2, p));
                0x80
            }
            _ => 0x00,
        };
        let mut v = vec![0, 0, 1, stream_id, 0, 0, 0x80, flags, ts.len() as u8];
        v.extend_from_slice(&ts);
        v.extend_from_slice(es);
        v
    }

    /// Cut a PES into TS packets of `chunk` payload bytes; the first one has PUSI set.
    fn pes_packets(pid: u16, pes: &[u8], chunk: usize) -> Vec<u8> {
        pes.chunks(chunk)
            .enumerate()
            .flat_map(|(i, c)| packet(pid, i == 0, c))
            .collect()
    }

    fn psi_packets(streams: &[Es]) -> Vec<u8> {
        let mut d = packet(PAT_PID, true, &pat(&[(1, PMT_PID)]));
        d.extend(packet(PMT_PID, true, &pmt(&[], streams)));
        d
    }

    const IDR: [u8; 3] = [0x65, 0xAA, 0xBB];
    const NON_IDR: [u8; 2] = [0x41, 0xCC];

    fn annex_b(nals: &[(&[u8], &[u8])]) -> Vec<u8> {
        nals.iter()
            .flat_map(|(sc, nal)| sc.iter().chain(nal.iter()).copied())
            .collect()
    }

    // ---- parse_ts_packet ----

    #[test]
    fn packet_header_fields_are_decoded() {
        let mut d = vec![0u8; TS_PACKET_SIZE];
        d[..4].copy_from_slice(&[0x47, 0x5F, 0xFF, 0x1A]);
        let p = parse_ts_packet(&d).unwrap();
        assert_eq!(p.pid, 0x1FFF);
        assert!(p.payload_unit_start);
        assert_eq!(p._adaptation_field_control, 0x01);
        assert_eq!(p._continuity_counter, 0x0A);
        assert_eq!(p.payload.len(), 184);
    }

    #[test]
    fn adaptation_field_only_or_reserved_has_no_payload() {
        for afc in [0x00u8, 0x20] {
            let mut d = vec![0u8; TS_PACKET_SIZE];
            d[..4].copy_from_slice(&[0x47, 0x01, 0x02, afc]);
            d[4] = 10;
            let p = parse_ts_packet(&d).unwrap();
            assert!(p.payload.is_empty(), "afc {afc:#x}");
        }
    }

    #[test]
    fn adaptation_field_is_skipped_before_the_payload() {
        let p = parse_ts_packet(&packet(0x100, false, &[1, 2, 3])).unwrap();
        assert_eq!(p.payload, vec![1, 2, 3]);
        assert_eq!(p._adaptation_field_control, 0x03);
    }

    #[test]
    fn adaptation_length_up_to_the_end_is_ok_and_one_past_is_an_error() {
        let mut d = vec![0u8; TS_PACKET_SIZE];
        d[..4].copy_from_slice(&[0x47, 0x00, 0x10, 0x30]);
        d[4] = 183; // exactly what is left after the length byte
        assert!(parse_ts_packet(&d).unwrap().payload.is_empty());
        d[4] = 184;
        let e = parse_ts_packet(&d).unwrap_err().to_string();
        assert!(e.contains("exceeds remaining data"), "{e}");
    }

    #[test]
    fn header_only_packets() {
        // payload-only, nothing after the header
        let p = parse_ts_packet(&[0x47, 0x00, 0x10, 0x10]).unwrap();
        assert!(p.payload.is_empty());
        // adaptation flag but no length byte
        let e = parse_ts_packet(&[0x47, 0x00, 0x10, 0x30]).unwrap_err();
        assert!(e.to_string().contains("length missing"), "{e}");
    }

    #[test]
    fn invalid_sync_byte_is_named_in_the_error() {
        let e = parse_ts_packet(&[0x48, 0, 0, 0]).unwrap_err().to_string();
        assert!(e.contains("0x48"), "{e}");
    }

    #[test]
    fn is_ts_checks_the_second_sync_byte_only_when_present() {
        let mut one = vec![0u8; TS_PACKET_SIZE];
        one[0] = 0x47;
        assert!(is_ts(&one));
        let mut two = vec![0u8; TS_PACKET_SIZE * 2];
        two[0] = 0x47;
        assert!(!is_ts(&two));
        two[TS_PACKET_SIZE] = 0x47;
        assert!(is_ts(&two));
        assert!(!is_ts(&one[..TS_PACKET_SIZE - 1]));
    }

    // ---- parse_pat ----

    #[test]
    fn pat_lists_programs_and_skips_the_network_entry() {
        let e = parse_pat(&pat(&[(0, 0x10), (1, 0x1000), (2, 0x1001)]), true).unwrap();
        let got: Vec<(u16, u16)> = e.iter().map(|x| (x._program_number, x.pmt_pid)).collect();
        assert_eq!(got, vec![(1, 0x1000), (2, 0x1001)]);
    }

    #[test]
    fn pat_pointer_field_is_honoured_only_with_pusi() {
        let mut with_pointer = vec![3, 0xAA, 0xBB, 0xCC];
        with_pointer.extend(&pat(&[(1, 0x1000)])[1..]); // drop the zero pointer_field
        assert_eq!(parse_pat(&with_pointer, true).unwrap().len(), 1);

        // without PUSI there is no pointer_field: the table starts at byte 0
        assert_eq!(
            parse_pat(&pat(&[(1, 0x1000)])[1..], false).unwrap().len(),
            1
        );
    }

    #[test]
    fn pat_rejects_or_ignores_malformed_sections() {
        assert!(parse_pat(&[], true).unwrap().is_empty());
        // pointer_field points past the payload
        assert!(parse_pat(&[9, 0, 0], true).unwrap().is_empty());
        // table_id is not 0
        let mut wrong = pat(&[(1, 0x1000)]);
        wrong[1] = 0x02;
        assert!(parse_pat(&wrong, true).unwrap().is_empty());
        // truncated right after table_id / section_length
        assert!(parse_pat(&[0, 0x00], true).unwrap().is_empty());
        // zero programs (section_length == 9)
        assert!(parse_pat(&pat(&[]), true).unwrap().is_empty());
    }

    #[test]
    fn pat_stops_at_section_end_even_when_more_bytes_follow() {
        let mut p = pat(&[(1, 0x1000)]);
        // the CRC + stuffing after the section must not be read as a 2nd program
        p.extend_from_slice(&[0, 2, 0xF0, 0x01, 0xFF, 0xFF]);
        assert_eq!(parse_pat(&p, true).unwrap().len(), 1);
    }

    // ---- parse_pmt ----

    #[test]
    fn pmt_lists_streams_and_skips_program_and_es_descriptors() {
        let p = pmt(
            &[0x05, 0x04, 0x48, 0x45, 0x56, 0x43],
            &[
                (STREAM_TYPE_AVC, 0x100, &[0x28, 0x01, 0x7A, 0x00]),
                (STREAM_TYPE_HEVC, 0x101, &[]),
                (0x0F, 0x102, &[0x0A, 0x04, b'e', b'n', b'g', 0]),
            ],
        );
        let s = parse_pmt(&p, true).unwrap();
        let got: Vec<(u8, u16)> = s
            .iter()
            .map(|x| (x.stream_type, x.elementary_pid))
            .collect();
        assert_eq!(got, vec![(0x1B, 0x100), (0x24, 0x101), (0x0F, 0x102)]);
    }

    #[test]
    fn pmt_pointer_field_is_honoured_only_with_pusi() {
        let body = pmt(&[], &[(STREAM_TYPE_AVC, 0x100, &[])]);
        let mut with_pointer = vec![2, 0xAA, 0xBB];
        with_pointer.extend(&body[1..]);
        assert_eq!(parse_pmt(&with_pointer, true).unwrap().len(), 1);
        assert_eq!(parse_pmt(&body[1..], false).unwrap().len(), 1);
    }

    #[test]
    fn pmt_rejects_or_ignores_malformed_sections() {
        assert!(parse_pmt(&[], false).unwrap().is_empty());
        assert!(parse_pmt(&[9, 0, 0], true).unwrap().is_empty());
        let mut wrong = pmt(&[], &[(STREAM_TYPE_AVC, 0x100, &[])]);
        wrong[1] = 0x00; // table_id 0 = PAT
        assert!(parse_pmt(&wrong, true).unwrap().is_empty());
        // truncated after table_id, and after the fixed header
        assert!(parse_pmt(&[0, 0x02, 0xB0], true).unwrap().is_empty());
        assert!(parse_pmt(&[0, 0x02, 0xB0, 0x20, 0, 1, 0xC1, 0, 0], true)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn pmt_stops_at_a_truncated_stream_entry() {
        let mut p = pmt(
            &[],
            &[
                (STREAM_TYPE_AVC, 0x100, &[]),
                (STREAM_TYPE_HEVC, 0x101, &[]),
            ],
        );
        // keep the 1st entry whole and 3 bytes of the 2nd; section_length still claims both
        let cut = 1 + 3 + 5 + 2 + 5 + 3;
        p.truncate(cut);
        let s = parse_pmt(&p, true).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].stream_type, STREAM_TYPE_AVC);
    }

    #[test]
    fn pmt_stops_at_the_end_given_by_section_length() {
        let mut p = pmt(&[], &[(STREAM_TYPE_AVC, 0x100, &[])]);
        // append a second, well-formed entry after the section end (before the CRC would be)
        p.extend_from_slice(&[STREAM_TYPE_HEVC, 0xE1, 0x01, 0xF0, 0x00]);
        assert_eq!(parse_pmt(&p, true).unwrap().len(), 1);
    }

    // ---- parse_pes ----

    #[test]
    fn pes_without_optional_header_for_system_stream_ids() {
        for id in [0xBC, 0xBF, 0xF0, 0xF1, 0xF2, 0xF8, 0xFF] {
            let d = [0, 0, 1, id, 0, 4, 0xDE, 0xAD, 0xBE, 0xEF];
            let p = parse_pes(&d).unwrap();
            assert_eq!(p._stream_id, id);
            assert_eq!((p.pts, p.dts), (None, None));
            assert_eq!(p.payload, vec![0xDE, 0xAD, 0xBE, 0xEF], "stream id {id:#x}");
        }
    }

    #[test]
    fn pes_pts_and_dts_are_decoded() {
        let es = [9u8, 8, 7];
        let pts = 0x1_2345_6789u64; // uses all 33 bits' worth of groups
        let dts = 0x0_0ABC_DEF0u64;
        let p = parse_pes(&pes(0xE0, Some(pts), Some(dts), &es)).unwrap();
        assert_eq!((p.pts, p.dts), (Some(pts), Some(dts)));
        assert_eq!(p.payload, es);

        let p = parse_pes(&pes(0xE0, Some(pts), None, &es)).unwrap();
        assert_eq!((p.pts, p.dts), (Some(pts), None));
        assert_eq!(p.payload, es);

        let p = parse_pes(&pes(0xE0, None, None, &es)).unwrap();
        assert_eq!((p.pts, p.dts), (None, None));
        assert_eq!(p.payload, es);
    }

    #[test]
    fn pes_timestamp_helper_round_trips_the_33_bit_range() {
        for t in [
            0u64,
            1,
            0x7F,
            0x80,
            0x7FFF,
            0x8000,
            0x3FFF_FFFF,
            0x4000_0000,
            (1 << 33) - 1,
        ] {
            assert_eq!(parse_timestamp(&timestamp(0x2, t)), t, "{t:#x}");
        }
    }

    #[test]
    fn pes_header_truncation_cases() {
        // video stream id but the 3 flag bytes are not all there
        let e = parse_pes(&[0, 0, 1, 0xE0, 0, 0, 0x80, 0x80])
            .unwrap_err()
            .to_string();
        assert!(e.contains("PES header too short"), "{e}");
        // header present, no payload
        let p = parse_pes(&[0, 0, 1, 0xE0, 0, 0, 0x80, 0x00, 0x00]).unwrap();
        assert!(p.payload.is_empty());
        // PTS announced but cut short: no PTS, and the payload offset lies past the end
        let p = parse_pes(&[0, 0, 1, 0xE0, 0, 0, 0x80, 0x80, 0x05, 0x21, 0, 1]).unwrap();
        assert_eq!(p.pts, None);
        assert!(p.payload.is_empty());
        // DTS announced but cut short: PTS survives
        let mut d = vec![0, 0, 1, 0xE0, 0, 0, 0x80, 0xC0, 0x0A];
        d.extend_from_slice(&timestamp(0x3, 5));
        d.extend_from_slice(&[0x11, 0, 1]);
        let p = parse_pes(&d).unwrap();
        assert_eq!((p.pts, p.dts), (Some(5), None));
    }

    #[test]
    fn pes_with_pts_and_no_payload_keeps_the_pts() {
        // exactly 5 timestamp bytes left in the buffer
        let p = parse_pes(&pes(0xE0, Some(77), None, &[])).unwrap();
        assert_eq!(p.pts, Some(77));
        assert!(p.payload.is_empty());
        let p = parse_pes(&pes(0xE0, Some(77), Some(66), &[])).unwrap();
        assert_eq!((p.pts, p.dts), (Some(77), Some(66)));
    }

    #[test]
    fn hevc_irap_range_is_16_to_23_inclusive() {
        for nal_type in 0u8..=40 {
            let nal = [nal_type << 1, 0x01];
            assert_eq!(
                is_hevc_keyframe(&nal),
                (16..=23).contains(&nal_type),
                "type {nal_type}"
            );
        }
        // layer id / temporal id bits in the 2nd byte and the top bit are not part of the type
        assert!(is_hevc_keyframe(&[(19 << 1) | 1, 0xFF]));
        assert!(!is_hevc_keyframe(&[0x26]));
    }

    // ---- split_annex_b_nal_units ----

    #[test]
    fn annex_b_edge_cases() {
        assert!(split_annex_b_nal_units(&[1, 2, 3]).is_empty());
        assert!(split_annex_b_nal_units(&[0, 0, 1]).is_empty());
        assert!(split_annex_b_nal_units(&[0, 0, 0, 1]).is_empty());
        // consecutive start codes: the empty unit between them is dropped
        assert_eq!(
            split_annex_b_nal_units(&[0, 0, 1, 0, 0, 1, 0xAA]),
            vec![vec![0xAA]]
        );
        // extra leading zero before a 4-byte start code is not part of the unit
        assert_eq!(
            split_annex_b_nal_units(&[0, 0, 0, 0, 1, 0xAA]),
            vec![vec![0xAA]]
        );
        // garbage before the first start code is ignored
        assert_eq!(
            split_annex_b_nal_units(&[7, 7, 0, 0, 1, 0xAA]),
            vec![vec![0xAA]]
        );
    }

    #[test]
    fn annex_b_trims_the_next_start_code_of_either_length() {
        let d = annex_b(&[
            (&[0, 0, 0, 1], &IDR),
            (&[0, 0, 1], &NON_IDR),
            (&[0, 0, 0, 1], &[0x67]),
        ]);
        assert_eq!(
            split_annex_b_nal_units(&d),
            vec![IDR.to_vec(), NON_IDR.to_vec(), vec![0x67]]
        );
    }

    // ---- extract_video_samples_for_pid ----

    fn avc_pes_stream() -> Vec<u8> {
        // PES 1: IDR (4-byte code) + non-IDR (3-byte code), cut mid-NAL across packets; PES 2: one non-IDR
        let es1 = annex_b(&[(&[0, 0, 0, 1], &IDR), (&[0, 0, 1], &NON_IDR)]);
        let es2 = annex_b(&[(&[0, 0, 0, 1], &NON_IDR)]);
        let mut d = Vec::new();
        d.extend(packet(0x55, true, &[1, 2, 3])); // other PID: ignored
        d.extend(pes_packets(
            VIDEO_PID,
            &pes(0xE0, Some(9000), Some(6000), &es1),
            24,
        ));
        d.extend(packet(0x55, false, &[4, 5, 6]));
        d.extend(pes_packets(
            VIDEO_PID,
            &pes(0xE0, Some(12000), None, &es2),
            184,
        ));
        d
    }

    #[test]
    fn video_samples_are_reassembled_across_packets() {
        let samples =
            extract_video_samples_for_pid(&avc_pes_stream(), VIDEO_PID, is_avc_keyframe).unwrap();
        type Row = (Vec<u8>, bool, Option<u64>, Option<u64>);
        let view: Vec<Row> = samples
            .iter()
            .map(|s| (s.data.clone(), s.key_frame, s.pts, s.dts))
            .collect();
        assert_eq!(
            view,
            vec![
                (IDR.to_vec(), true, Some(9000), Some(6000)),
                (NON_IDR.to_vec(), false, Some(9000), Some(6000)),
                (NON_IDR.to_vec(), false, Some(12000), None),
            ]
        );
        // offsets are cumulative NAL sizes, sizes match the data
        assert_eq!(
            samples.iter().map(|s| s.size).collect::<Vec<_>>(),
            vec![3, 2, 2]
        );
        assert_eq!(
            samples.iter().map(|s| s.offset).collect::<Vec<_>>(),
            vec![0, 3, 5]
        );
    }

    #[test]
    fn video_samples_use_the_given_keyframe_rule() {
        let s = extract_video_samples_for_pid(&avc_pes_stream(), VIDEO_PID, |_| true).unwrap();
        assert!(s.iter().all(|x| x.key_frame));
        let s = extract_video_samples_for_pid(&avc_pes_stream(), VIDEO_PID, |_| false).unwrap();
        assert!(s.iter().all(|x| !x.key_frame));
    }

    #[test]
    fn video_samples_ignore_a_trailing_partial_packet_and_other_pids() {
        let mut d = avc_pes_stream();
        d.extend_from_slice(&[0x47, 0x00, 0x00]); // < 188 bytes: not parsed
        assert_eq!(
            extract_video_samples_for_pid(&d, VIDEO_PID, is_avc_keyframe)
                .unwrap()
                .len(),
            3
        );
        assert!(extract_video_samples_for_pid(&d, 0x777, is_avc_keyframe)
            .unwrap()
            .is_empty());
        assert!(
            extract_video_samples_for_pid(&[], VIDEO_PID, is_avc_keyframe)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn video_samples_propagate_a_bad_packet() {
        let mut d = avc_pes_stream();
        let bad = d.len() / TS_PACKET_SIZE / 2 * TS_PACKET_SIZE;
        d[bad] = 0x00;
        assert!(extract_video_samples_for_pid(&d, VIDEO_PID, is_avc_keyframe).is_err());
    }

    #[test]
    fn video_continuation_without_a_start_is_dropped() {
        let es = annex_b(&[(&[0, 0, 0, 1], &IDR)]);
        let mut d = packet(VIDEO_PID, false, &[0, 0, 0, 1, 0x65, 0x01]); // no PES start yet
        d.extend(pes_packets(VIDEO_PID, &pes(0xE0, Some(1), None, &es), 184));
        let s = extract_video_samples_for_pid(&d, VIDEO_PID, is_avc_keyframe).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].data, IDR);
    }

    #[test]
    fn video_pes_with_a_bad_start_code_is_skipped_with_its_continuations() {
        let es = annex_b(&[(&[0, 0, 0, 1], &IDR)]);
        let mut d = packet(VIDEO_PID, true, &[9, 9, 9, 9, 9, 9, 9]); // PUSI, not a PES
        d.extend(packet(VIDEO_PID, false, &[0, 0, 0, 1, 0x41, 0x01]));
        d.extend(pes_packets(VIDEO_PID, &pes(0xE0, Some(1), None, &es), 184));
        let s = extract_video_samples_for_pid(&d, VIDEO_PID, is_avc_keyframe).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].data, IDR);
    }

    // ---- extract_pes_packets (the AV1 path) ----

    #[test]
    fn pes_packets_collect_payloads_and_pts() {
        let mut d = pes_packets(
            VIDEO_PID,
            &pes(0xE0, Some(3000), None, &[1, 2, 3, 4, 5, 6, 7, 8]),
            12,
        );
        d.extend(pes_packets(VIDEO_PID, &pes(0xE0, None, None, &[9, 9]), 184));
        d.extend(packet(0x55, true, &[0]));
        let (samples, ts) = extract_pes_packets(&d, VIDEO_PID).unwrap();
        assert_eq!(samples, vec![vec![1, 2, 3, 4, 5, 6, 7, 8], vec![9, 9]]);
        assert_eq!(ts, vec![3000, 0]); // a PES without PTS is stamped 0
    }

    #[test]
    fn pes_packets_drop_buffers_that_are_not_pes() {
        // a continuation packet before any PUSI creates a buffer that fails to parse when flushed
        let mut d = packet(VIDEO_PID, false, &[1, 2, 3]);
        d.extend(pes_packets(VIDEO_PID, &pes(0xE0, Some(1), None, &[5]), 184));
        // and a final PUSI packet that is not a PES at all
        d.extend(packet(VIDEO_PID, true, &[7, 7, 7, 7, 7, 7]));
        let (samples, ts) = extract_pes_packets(&d, VIDEO_PID).unwrap();
        assert_eq!(samples, vec![vec![5]]);
        assert_eq!(ts, vec![1]);
    }

    #[test]
    fn pes_packets_propagate_a_bad_packet() {
        let mut d = pes_packets(VIDEO_PID, &pes(0xE0, Some(1), None, &[5]), 184);
        d.extend(vec![0u8; TS_PACKET_SIZE]);
        assert!(extract_pes_packets(&d, VIDEO_PID).is_err());
    }

    // ---- extract_pat_pmt and the public extractors ----

    #[test]
    fn pat_pmt_scan_ignores_unrelated_and_early_packets() {
        let pat_pkt = packet(PAT_PID, true, &pat(&[(1, PMT_PID)]));
        let pmt_pkt = packet(
            PMT_PID,
            true,
            &pmt(&[], &[(STREAM_TYPE_AVC, VIDEO_PID, &[])]),
        );
        let other_pmt = packet(0x1FFE, true, &pmt(&[], &[(STREAM_TYPE_HEVC, 0x300, &[])]));
        let empty_pat = packet(PAT_PID, false, &[]);

        // PMT before any PAT is not recognised
        let d = [&pmt_pkt[..], &pat_pkt[..]].concat();
        let (pat, pmt) = extract_pat_pmt(&d).unwrap();
        assert_eq!((pat.len(), pmt.len()), (1, 0));

        // PMT after the PAT is read; a PMT on a PID the PAT does not name and an empty PAT packet are ignored
        let d = [&pat_pkt[..], &other_pmt[..], &empty_pat[..], &pmt_pkt[..]].concat();
        let (pat, pmt) = extract_pat_pmt(&d).unwrap();
        assert_eq!(pat.len(), 1);
        assert_eq!(pmt.len(), 1);
        assert_eq!(
            (pmt[0].stream_type, pmt[0].elementary_pid),
            (STREAM_TYPE_AVC, VIDEO_PID)
        );

        assert!(extract_pat_pmt(&[0xFF; TS_PACKET_SIZE]).is_err());
    }

    fn full_stream(stream_type: u8, es: &[u8]) -> Vec<u8> {
        let mut d = psi_packets(&[(stream_type, VIDEO_PID, &[])]);
        d.extend(pes_packets(
            VIDEO_PID,
            &pes(0xE0, Some(4500), Some(3000), es),
            20,
        ));
        d
    }

    #[test]
    fn avc_extractors_list_nal_units_and_group_them_into_access_units() {
        let d = full_stream(
            STREAM_TYPE_AVC,
            &annex_b(&[(&[0, 0, 0, 1], &IDR), (&[0, 0, 1], &NON_IDR)]),
        );
        // the plain extractor lists NAL units
        assert_eq!(
            extract_avc_samples(&d).unwrap(),
            vec![IDR.to_vec(), NON_IDR.to_vec()]
        );
        // the video-sample extractor returns one sample per picture: here one PES, no delimiter
        let v = extract_avc_video_samples(&d).unwrap();
        assert_eq!(v.len(), 1);
        let mut expected = vec![0, 0, 0, 1];
        expected.extend(IDR);
        expected.extend([0, 0, 0, 1]);
        expected.extend(NON_IDR);
        assert_eq!(v[0].data, expected);
        assert_eq!((v[0].offset, v[0].size), (0, expected.len()));
        assert!(v[0].key_frame);
        assert_eq!((v[0].pts, v[0].dts), (Some(4500), Some(3000)));
        // a stream of another type yields nothing rather than an error
        assert!(extract_hevc_samples(&d).unwrap().is_empty());
        assert!(extract_hevc_video_samples(&d).unwrap().is_empty());
    }

    /// Several pictures: one PES each, with the AUD, SPS/PPS and slices a real muxer writes.
    fn avc_pictures_stream() -> Vec<u8> {
        const AUD: [u8; 2] = [0x09, 0xF0];
        const SPS: [u8; 2] = [0x67, 0x42];
        const PPS: [u8; 2] = [0x68, 0xCE];
        let mut d = psi_packets(&[(STREAM_TYPE_AVC, VIDEO_PID, &[])]);
        for (i, nals) in [
            vec![&AUD[..], &SPS[..], &PPS[..], &IDR[..]],
            vec![&AUD[..], &NON_IDR[..]],
            vec![&AUD[..], &NON_IDR[..]],
        ]
        .iter()
        .enumerate()
        {
            let es: Vec<u8> = nals
                .iter()
                .flat_map(|n| [0u8, 0, 0, 1].into_iter().chain(n.iter().copied()))
                .collect();
            d.extend(pes_packets(
                VIDEO_PID,
                &pes(0xE0, Some(9000 + 3000 * i as u64), None, &es),
                184,
            ));
        }
        d
    }

    #[test]
    fn video_samples_are_one_per_picture_with_that_pictures_timing_and_offsets() {
        let d = avc_pictures_stream();
        // 8 NAL units (AUD+SPS+PPS+IDR, AUD+slice, AUD+slice), 3 pictures
        assert_eq!(extract_avc_samples(&d).unwrap().len(), 8);
        let v = extract_avc_video_samples(&d).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(
            v.iter().map(|s| s.key_frame).collect::<Vec<_>>(),
            vec![true, false, false]
        );
        assert_eq!(
            v.iter().map(|s| s.pts).collect::<Vec<_>>(),
            vec![Some(9000), Some(12000), Some(15000)]
        );
        // contiguous, and the samples concatenate back to every NAL unit
        let mut at = 0;
        for sample in &v {
            assert_eq!((sample.offset, sample.size), (at, sample.data.len()));
            at += sample.size;
        }
        let all: Vec<u8> = v.iter().flat_map(|s| s.data.iter().copied()).collect();
        let nals = split_annex_b_nal_units(&all);
        assert_eq!(nals, extract_avc_samples(&d).unwrap());
    }

    #[test]
    fn every_pes_packet_is_its_own_picture_even_without_delimiters() {
        let mut d = psi_packets(&[(STREAM_TYPE_AVC, VIDEO_PID, &[])]);
        for (i, nal) in [&IDR[..], &NON_IDR[..], &NON_IDR[..]].iter().enumerate() {
            let es = annex_b(&[(&[0, 0, 0, 1], nal)]);
            d.extend(pes_packets(
                VIDEO_PID,
                &pes(0xE0, Some(9000 + 3000 * i as u64), None, &es),
                184,
            ));
        }
        let v = extract_avc_video_samples(&d).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(
            v.iter().map(|s| s.pts).collect::<Vec<_>>(),
            vec![Some(9000), Some(12000), Some(15000)]
        );
        assert_eq!(
            v.iter().map(|s| s.key_frame).collect::<Vec<_>>(),
            vec![true, false, false]
        );
    }

    #[test]
    fn a_delimiter_inside_one_pes_starts_another_picture() {
        const AUD: [u8; 2] = [0x09, 0xF0];
        let es = annex_b(&[
            (&[0, 0, 0, 1], &AUD),
            (&[0, 0, 0, 1], &IDR),
            (&[0, 0, 0, 1], &AUD),
            (&[0, 0, 0, 1], &NON_IDR),
        ]);
        let v = extract_avc_video_samples(&full_stream(STREAM_TYPE_AVC, &es)).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(
            v.iter().map(|s| s.key_frame).collect::<Vec<_>>(),
            vec![true, false]
        );
        // a delimiter that is the very first NAL does not open an empty picture
        let v = extract_avc_video_samples(&full_stream(
            STREAM_TYPE_AVC,
            &annex_b(&[(&[0, 0, 0, 1], &AUD), (&[0, 0, 0, 1], &NON_IDR)]),
        ))
        .unwrap();
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn hevc_extractors_list_nal_units_and_group_them_into_access_units() {
        let aud = [0x46u8, 0x01, 0x50]; // AUD_NUT (35)
        let idr = [0x26u8, 0x01, 0xAF]; // IDR_W_RADL
        let trail = [0x02u8, 0x01, 0xBF]; // TRAIL_R
        let d = full_stream(
            STREAM_TYPE_HEVC,
            &annex_b(&[
                (&[0, 0, 0, 1], &aud),
                (&[0, 0, 0, 1], &idr),
                (&[0, 0, 1], &aud),
                (&[0, 0, 1], &trail),
            ]),
        );
        assert_eq!(extract_hevc_samples(&d).unwrap().len(), 4);
        let v = extract_hevc_video_samples(&d).unwrap();
        assert_eq!(
            v.iter().map(|s| s.key_frame).collect::<Vec<_>>(),
            vec![true, false]
        );
        assert!(extract_avc_samples(&d).unwrap().is_empty());
        assert!(extract_avc_video_samples(&d).unwrap().is_empty());
    }

    #[test]
    fn av1_extractor_returns_pes_payloads() {
        let d = full_stream(STREAM_TYPE_AV1, &[0x12, 0x00, 0x0A]);
        let info = parse_ts(&d).unwrap();
        assert_eq!(info.video_pid, Some(VIDEO_PID));
        assert_eq!(info.sample_count, 1);
        assert_eq!(info.samples, vec![vec![0x12, 0x00, 0x0A]]);
        assert_eq!(info.timestamps, vec![4500]);
        assert_eq!(extract_av1_samples(&d).unwrap(), info.samples);
    }

    #[test]
    fn parse_ts_without_an_av1_stream_reports_none() {
        let d = full_stream(STREAM_TYPE_AVC, &annex_b(&[(&[0, 0, 1], &IDR)]));
        let info = parse_ts(&d).unwrap();
        assert_eq!((info.video_pid, info.sample_count), (None, 0));
        assert!(info.samples.is_empty() && info.timestamps.is_empty());
    }

    #[test]
    fn pid_lookup_picks_the_first_match() {
        let s = [
            PmtStream {
                stream_type: 0x0F,
                elementary_pid: 1,
            },
            PmtStream {
                stream_type: STREAM_TYPE_AV1,
                elementary_pid: 2,
            },
            PmtStream {
                stream_type: STREAM_TYPE_AV1,
                elementary_pid: 3,
            },
        ];
        assert_eq!(find_video_pid(&s), Some(2));
        assert_eq!(find_pid_by_stream_type(&s, 0x0F), Some(1));
        assert_eq!(find_pid_by_stream_type(&s, STREAM_TYPE_HEVC), None);
        assert_eq!(find_video_pid(&[]), None);
    }

    // ---- spec layout and malformed section lengths (#261) ----

    /// The PAT and PMT sections ffmpeg writes for one H.264 stream (PCR_PID 0x100, PMT PID 0x1000),
    /// byte for byte, as quoted in #261. CRC_32 is not checked, so it is left as zeros.
    #[test]
    fn real_muxer_pat_and_pmt_bytes_are_parsed() {
        let pat = [
            0x00, 0x00, 0xB0, 0x0D, 0x00, 0x01, 0xC1, 0x00, 0x00, 0x00, 0x01, 0xF0, 0x00, 0, 0, 0,
            0,
        ];
        let entries = parse_pat(&pat, true).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pmt_pid, 0x1000);

        let pmt = [
            0x00, 0x02, 0xB0, 0x12, 0x00, 0x01, 0xC1, 0x00, 0x00, 0xE1, 0x00, 0xF0, 0x00, 0x1B,
            0xE1, 0x00, 0xF0, 0x00, 0, 0, 0, 0,
        ];
        let streams = parse_pmt(&pmt, true).unwrap();
        assert_eq!(streams.len(), 1);
        assert_eq!(
            (streams[0].stream_type, streams[0].elementary_pid),
            (STREAM_TYPE_AVC, 0x100)
        );
    }

    #[test]
    fn spec_layout_pmt_is_parsed() {
        let p = pmt(
            &[0x05, 0x04, 0x48, 0x45, 0x56, 0x43],
            &[(STREAM_TYPE_AVC, 0x100, &[0x28, 0x01])],
        );
        let s = parse_pmt(&p, true).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(
            (s[0].stream_type, s[0].elementary_pid),
            (STREAM_TYPE_AVC, 0x100)
        );
    }

    #[test]
    fn spec_layout_stream_yields_samples() {
        let mut d = psi_packets(&[(STREAM_TYPE_AVC, VIDEO_PID, &[])]);
        d.extend(pes_packets(
            VIDEO_PID,
            &pes(0xE0, Some(1), None, &annex_b(&[(&[0, 0, 1], &IDR)])),
            184,
        ));
        assert_eq!(extract_avc_samples(&d).unwrap(), vec![IDR.to_vec()]);
    }

    #[test]
    fn malformed_section_lengths_do_not_panic() {
        // section_length smaller than the fixed header, and larger than the payload
        assert!(parse_pat(
            &[0x00, 0x00, 0xB0, 0x04, 0, 1, 0xC1, 0, 0, 0, 1, 0xE1, 0x00],
            true
        )
        .is_ok());
        assert!(parse_pat(&[0x00, 0x00, 0x00, 0x00, 0, 0, 0, 0, 0, 0], false).is_ok());
        assert!(parse_pat(
            &[0x00, 0x00, 0xB0, 0xFF, 0, 1, 0xC1, 0, 0, 0, 1, 0xE1, 0x00],
            true
        )
        .is_ok());
        assert!(parse_pmt(
            &[0x00, 0x02, 0xB0, 0x00, 0, 1, 0xC1, 0, 0, 0xF0, 0x00],
            true
        )
        .is_ok());
    }
}
