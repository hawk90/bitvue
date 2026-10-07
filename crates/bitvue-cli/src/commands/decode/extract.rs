//! Container/bitstream walking: turns raw file bytes into per-frame `FrameRecord`s
//! (type, size, pts, offset, optional md5) for each supported codec.

use super::{codec_name, ForceCodec};
use anyhow::Result;
use bitvue_av1_codec::{parse_ivf_frames, ObuIterator, ObuType};

// ─── Unified frame record ─────────────────────────────────────────────────────

#[derive(Debug)]
pub(super) struct FrameRecord {
    pub(super) index: usize,
    pub(super) frame_type: String,
    pub(super) size: usize,
    pub(super) pts: Option<u64>,
    pub(super) offset: u64,
    pub(super) key_frame: bool,
    /// Raw compressed data — populated only when --md5 is requested.
    pub(super) md5_hex: Option<String>,
}

// ─── Frame extraction ─────────────────────────────────────────────────────────

/// Returns (frames, parse_error_count).
pub(super) fn extract_frames(
    codec: ForceCodec,
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    match codec {
        ForceCodec::AV1 => extract_av1_frames(data, limit, want_md5),
        ForceCodec::HEVC => extract_hevc_frames(data, limit, want_md5),
        ForceCodec::AVC => extract_avc_frames(data, limit, want_md5),
        ForceCodec::VP9 => extract_vp9_frames(data, limit, want_md5),
        ForceCodec::AVS3 => extract_avs3_frames(data, limit, want_md5),
        ForceCodec::JpegXs => extract_jpegxs_frames(data, limit, want_md5),
        ForceCodec::Vc3 => extract_vc3_frames_cli(data, limit, want_md5),
        _ => {
            eprintln!(
                "Note: Frame extraction not yet implemented for {}.",
                codec_name(codec)
            );
            Ok((Vec::new(), 0))
        }
    }
}

pub(super) fn md5_hex(data: &[u8]) -> String {
    let digest = md5::compute(data);
    format!("{:x}", digest)
}

fn maybe_md5(data: &[u8], want: bool) -> Option<String> {
    if want {
        Some(md5_hex(data))
    } else {
        None
    }
}

// AV1 ──────────────────────────────────────────────────────────────────────────

fn extract_av1_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    let (_header, frames) =
        parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("IVF parse error: {}", e))?;

    let mut records = Vec::with_capacity(frames.len().min(limit));
    let mut errors = 0usize;
    let mut offset: u64 = 32; // IVF file header

    for (idx, frame) in frames.iter().enumerate() {
        if records.len() >= limit {
            break;
        }

        let frame_offset = offset + 12;
        let (frame_type, had_error) = av1_frame_type(&frame.data);
        if had_error {
            errors += 1;
        }

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size: frame.size as usize,
            pts: Some(frame.timestamp),
            offset: frame_offset,
            key_frame: false, // filled from frame_type
            md5_hex: maybe_md5(&frame.data, want_md5),
        });
        // Fix key_frame
        let last = records.last_mut().unwrap();
        last.key_frame = last.frame_type == "I" || last.frame_type == "KEY";

        offset += 12 + frame.size as u64;
    }

    Ok((records, errors))
}

fn av1_frame_type(data: &[u8]) -> (String, bool) {
    let mut error = false;
    for obu in ObuIterator::new(data) {
        match obu {
            Ok(obu) if matches!(obu.header.obu_type, ObuType::Frame | ObuType::FrameHeader) => {
                if let Some(ft) = obu.frame_type {
                    return (ft.as_str().to_string(), error);
                }
            }
            Err(_) => {
                error = true;
            }
            _ => {}
        }
    }
    ("?".to_string(), error)
}

// HEVC ─────────────────────────────────────────────────────────────────────────

fn extract_hevc_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_hevc::parse_hevc;

    let stream = parse_hevc(data).map_err(|e| anyhow::anyhow!("HEVC parse error: {}", e))?;
    let mut records = Vec::new();

    // Use slice records to associate NAL offset/size/type
    for (idx, slice) in stream.slices.iter().enumerate() {
        if records.len() >= limit {
            break;
        }

        let nal = &stream.nal_units[slice.nal_index];
        let nal_type = nal.header.nal_unit_type;
        let key_frame = nal_type.is_idr();
        let frame_type = if key_frame {
            "IDR".to_string()
        } else if nal_type.is_irap() {
            "CRA".to_string()
        } else {
            format!("{:?}", nal_type)
        };
        let offset = nal.offset;
        let size = nal.size as usize;
        let end = (offset as usize + size).min(data.len());
        let frame_data = &data[offset as usize..end];

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size,
            pts: Some(slice.poc as u64),
            offset,
            key_frame,
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, 0))
}

// AVC ──────────────────────────────────────────────────────────────────────────

fn extract_avc_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_avc::{parse_avc, NalUnitType};

    let stream = parse_avc(data).map_err(|e| anyhow::anyhow!("AVC parse error: {}", e))?;
    let mut records = Vec::new();

    for (idx, slice) in stream.slices.iter().enumerate() {
        if records.len() >= limit {
            break;
        }

        let nal = &stream.nal_units[slice.nal_index];
        let is_idr = nal.header.nal_unit_type == NalUnitType::IdrSlice;
        let frame_type = format!("{:?}", slice.header.slice_type);
        let offset = nal.offset;
        let size = nal.size;
        let end = (offset + size).min(data.len());
        let frame_data = &data[offset..end];

        records.push(FrameRecord {
            index: idx,
            frame_type,
            size,
            pts: Some(slice.poc as u64),
            offset: offset as u64,
            key_frame: is_idr,
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, 0))
}

// VP9 ──────────────────────────────────────────────────────────────────────────

fn extract_vp9_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_vp9::{parse_frame_header, FrameType as Vp9FrameType2};

    // VP9 is commonly carried in IVF.  When the file starts with "DKIF" we strip
    // the IVF container and process each IVF packet as one VP9 frame payload.
    // For raw VP9 bitstream (no IVF wrapper) we pass the whole slice.
    let is_ivf = data.len() >= 4 && &data[0..4] == b"DKIF";

    let _owned: Vec<u8>; // keep allocations alive

    let payloads: Vec<(u64, u64, &[u8])> = if is_ivf {
        // (pts, file_offset, vp9_payload)
        let (hdr, ivf_frames) =
            parse_ivf_frames(data).map_err(|e| anyhow::anyhow!("VP9 IVF parse error: {}", e))?;
        let header_size = hdr.header_size as usize;
        ivf_frames
            .iter()
            .scan(header_size, |off, f| {
                // IVF frame header = 12 bytes (4 size + 8 pts)
                let payload_off = *off + 12;
                let payload_end = (payload_off + f.data.len()).min(data.len());
                let pts = f.timestamp;
                let file_off = payload_off as u64;
                *off = payload_end;
                Some((pts, file_off, &data[payload_off..payload_end]))
            })
            .collect()
    } else {
        // Raw VP9 — treat whole slice as one payload
        vec![(0, 0, data)]
    };

    let mut records: Vec<FrameRecord> = Vec::new();

    for (frame_idx, (pts, file_offset, payload)) in payloads.iter().enumerate() {
        if records.len() >= limit {
            break;
        }
        if payload.is_empty() {
            continue;
        }

        // Parse VP9 uncompressed frame header to get frame type
        let key_frame = match parse_frame_header(payload) {
            Ok(hdr) => matches!(hdr.frame_type, Vp9FrameType2::Key),
            // If header parse fails, assume INTER (non-key) — don't crash
            Err(_) => false,
        };
        let frame_type = if key_frame { "KEY" } else { "INTER" }.to_string();

        records.push(FrameRecord {
            index: frame_idx,
            frame_type,
            size: payload.len(),
            pts: Some(*pts),
            offset: *file_offset,
            key_frame,
            md5_hex: maybe_md5(payload, want_md5),
        });
    }

    Ok((records, 0))
}

// AVS3 ─────────────────────────────────────────────────────────────────────────

fn extract_avs3_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_avs3::extract_avs3_frames as avs3_extract;

    let result =
        avs3_extract(data, limit).map_err(|e| anyhow::anyhow!("AVS3 parse error: {}", e))?;

    let mut records = Vec::new();
    for frame in &result.frames {
        let end = (frame.offset + frame.size).min(data.len());
        let frame_data = &data[frame.offset..end];
        records.push(FrameRecord {
            index: frame.frame_index,
            frame_type: frame.frame_type_str().to_string(),
            size: frame.size,
            pts: None,
            offset: frame.offset as u64,
            key_frame: frame.is_key_frame(),
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, result.parse_errors))
}

fn extract_jpegxs_frames(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_jpegxs::extract_jpegxs_frames as jxs_extract;

    let result =
        jxs_extract(data, limit).map_err(|e| anyhow::anyhow!("JPEG XS parse error: {}", e))?;

    let mut records = Vec::new();
    for frame in &result.frames {
        let end = (frame.offset + frame.size).min(data.len());
        let frame_data = &data[frame.offset..end];
        records.push(FrameRecord {
            index: frame.frame_index,
            frame_type: "JXS".to_string(),
            size: frame.size,
            pts: None,
            offset: frame.offset as u64,
            key_frame: true, // JPEG XS is intra-only
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, result.parse_errors))
}

fn extract_vc3_frames_cli(
    data: &[u8],
    limit: usize,
    want_md5: bool,
) -> Result<(Vec<FrameRecord>, usize)> {
    use bitvue_vc3::extract_vc3_frames;

    let result =
        extract_vc3_frames(data, limit).map_err(|e| anyhow::anyhow!("VC-3 parse error: {}", e))?;

    let mut records = Vec::new();
    for frame in &result.frames {
        let end = (frame.offset + frame.frame_size as usize).min(data.len());
        let frame_data = &data[frame.offset..end];
        records.push(FrameRecord {
            index: frame.frame_index,
            frame_type: frame.frame_type_str().to_string(),
            size: frame.frame_size as usize,
            pts: None,
            offset: frame.offset as u64,
            key_frame: true, // VC-3 frames are all intra
            md5_hex: maybe_md5(frame_data, want_md5),
        });
    }

    Ok((records, result.parse_errors))
}
