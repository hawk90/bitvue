//! Metadata-indexing pipeline for `bitvue_engine::Core`.
//!
//! Populates `StreamState.container`/`.units` from a real file -- no pixel decode. This is
//! deliberately the *first* stage only (see docs/DEVELOPMENT_PHASES.md's decode-pipeline design
//! discussion, 2026-08-08): `.timeline` is NOT populated here, and only IVF/AV1 is supported so
//! far. Other formats/codecs get an honest `Event::DiagnosticAdded`, not a fabricated result.
//!
//! `.syntax` IS populated, but lazily: [`get_frame_syntax`] parses one unit's syntax tree on
//! demand (mirrors `Core::handle_command`'s `SelectBitRange`, which already does an on-demand
//! nearest-node lookup against whatever `.syntax` happens to hold) rather than eagerly building a
//! tree for every unit in [`index_stream`] -- that would be wasted work for units the UI never
//! looks at, and real cost on long streams.
//!
//! # Why this crate exists, and why it's not inside `bitvue-engine`
//!
//! `bitvue-engine` is a leaf crate (nothing in it depends on codec/format crates) -- `Core` can't
//! call into `bitvue-av1-codec` directly. But `Core::get_stream()`/`get_job_manager()` are already
//! public, so a crate on the *other* side of that dependency edge (this one, which depends on
//! `bitvue-engine` the same way `bitvue-decode` does) can read/write `StreamState` through the
//! existing `Arc<RwLock<StreamState>>` without any `bitvue-engine` changes. `bitvue-sidecar` calls
//! [`index_stream`] directly for its `index_stream` command -- there's no `Core::handle_command`
//! entry point for this (`Command::RunFullAnalysis` stays unused; see the sidecar's module doc).

use bitvue_av1_codec::frame_header::parse_frame_header_basic;
use bitvue_av1_codec::ivf::parse_ivf_frames;
use bitvue_av1_codec::parse_obu_syntax;
use bitvue_engine::event::{Category, Diagnostic, Severity};
use bitvue_engine::{ContainerFormat, ContainerModel, SyntaxModel, UnitModel, UnitNode};
use bitvue_engine::{Core, StreamId};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_DIAGNOSTIC_ID: AtomicU64 = AtomicU64::new(1);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn diagnostic_event(stream: StreamId, message: String) -> bitvue_engine::Event {
    bitvue_engine::Event::DiagnosticAdded {
        diagnostic: Diagnostic {
            id: NEXT_DIAGNOSTIC_ID.fetch_add(1, Ordering::Relaxed),
            severity: Severity::Error,
            stream_id: stream,
            message,
            category: Category::Container,
            offset_bytes: 0,
            timestamp_ms: now_ms(),
            frame_index: None,
            count: 1,
            impact_score: 60,
        },
    }
}

/// Index the given stream's already-open file: detect the container, parse its units, and write
/// `ContainerModel`/`UnitModel` into `StreamState`. Returns the resulting events (mirrors
/// `Core::handle_command`'s convention of reporting failures as `DiagnosticAdded` events rather
/// than a `Result`, since that's what every other stream-mutating operation in this codebase does).
pub fn index_stream(core: &Core, stream: StreamId) -> Vec<bitvue_engine::Event> {
    let byte_cache = {
        let stream_state = core.get_stream(stream);
        let state = stream_state.read();
        match state.byte_cache.clone() {
            Some(cache) => cache,
            None => {
                return vec![diagnostic_event(
                    stream,
                    "No file open for this stream".to_string(),
                )]
            }
        }
    };

    let len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, len) {
        Ok(d) => d,
        Err(e) => {
            return vec![diagnostic_event(
                stream,
                format!("Failed to read file: {e}"),
            )]
        }
    };

    if data.len() < 4 || &data[0..4] != b"DKIF" {
        // Honest scope limit: MP4/MKV/TS containers and non-AV1 codecs aren't indexed yet.
        return vec![diagnostic_event(
            stream,
            "Indexing is only implemented for IVF/AV1 streams so far".to_string(),
        )];
    }

    let (ivf_header, units) = match index_ivf_av1(data, stream) {
        Ok(result) => result,
        Err(e) => {
            return vec![diagnostic_event(
                stream,
                format!("Failed to parse IVF stream: {e}"),
            )]
        }
    };

    let unit_count = units.len();
    let container = ContainerModel {
        format: ContainerFormat::Ivf,
        codec: "av1".to_string(),
        track_count: 1,
        duration_ms: None,
        bitrate_bps: None,
        width: Some(ivf_header.width as u32),
        height: Some(ivf_header.height as u32),
        bit_depth: None,
    };
    let unit_model = UnitModel {
        units,
        unit_count,
        frame_count: unit_count,
    };

    {
        let stream_state = core.get_stream(stream);
        let mut state = stream_state.write();
        state.container = Some(container);
        state.units = Some(unit_model);
    }

    vec![
        bitvue_engine::Event::ModelUpdated {
            kind: bitvue_engine::event::ModelKind::Container,
            stream,
        },
        bitvue_engine::Event::ModelUpdated {
            kind: bitvue_engine::event::ModelKind::Units,
            stream,
        },
    ]
}

/// Parses one unit's syntax tree on demand (AV1 only, matching [`index_stream`]'s scope) and
/// writes it into `StreamState.syntax` -- `Core::handle_command`'s `SelectBitRange` handler
/// already does an on-demand nearest-node lookup against whatever `.syntax` holds, so this is
/// what actually makes that lookup meaningful instead of always empty. Returns the parsed model
/// directly (not events) since the caller (the sidecar's `get_frame_syntax` command) needs the
/// tree data itself, not just a "something changed" notification -- matches `get_hex_range`'s
/// data-returning shape rather than `index_stream`'s event-emitting one.
///
/// Requires [`index_stream`] to have already run for this stream (needs `.units` to look up the
/// unit's byte range) -- returns a plain error string, not a panic, if it hasn't.
pub fn get_frame_syntax(
    core: &Core,
    stream: StreamId,
    frame_index: usize,
) -> Result<SyntaxModel, String> {
    let (byte_cache, unit_offset, unit_size, codec) = {
        let stream_state = core.get_stream(stream);
        let state = stream_state.read();
        let codec = state
            .container
            .as_ref()
            .map(|c| c.codec.clone())
            .unwrap_or_default();
        let unit = state
            .units
            .as_ref()
            .and_then(|m| m.units.iter().find(|u| u.frame_index == Some(frame_index)))
            .ok_or_else(|| {
                format!(
                    "No unit found for frame_index {frame_index} -- has index_stream run for this stream?"
                )
            })?;
        let byte_cache = state
            .byte_cache
            .clone()
            .ok_or_else(|| "No file open for this stream".to_string())?;
        (byte_cache, unit.offset, unit.size, codec)
    };

    if codec != "av1" {
        return Err(format!(
            "Syntax parsing is only implemented for AV1 so far (this stream's codec: {codec:?})"
        ));
    }

    // unit_offset/unit_size cover the full IVF chunk (12-byte chunk header + OBU payload, see
    // index_ivf_av1's UnitNode construction) -- skip the chunk header to get the raw OBU bytes
    // parse_obu_syntax expects.
    if unit_size <= 12 {
        return Err(format!(
            "Unit at offset {unit_offset} is too small to contain OBU data"
        ));
    }
    let obu_offset = unit_offset + 12;
    let obu_len = unit_size - 12;
    let obu_bytes = byte_cache
        .read_range(obu_offset, obu_len)
        .map_err(|e| format!("Failed to read OBU bytes: {e}"))?;

    // parse_obu_syntax's global_offset is a BIT offset from file start (see
    // TrackedBitReader::new's doc), not a byte offset -- matches the CLI's analyze.rs convention
    // ((offset * 8) as u64). Passing the raw byte offset here would silently make every
    // SyntaxNode's bit_range wrong (off by a factor of 8), which the "jump to hex" frontend
    // feature would then jump to the wrong byte for.
    let model = parse_obu_syntax(obu_bytes, frame_index, obu_offset * 8)
        .map_err(|e| format!("Failed to parse OBU syntax: {e}"))?;

    {
        let stream_state = core.get_stream(stream);
        let mut state = stream_state.write();
        state.syntax = Some(model.clone());
    }

    Ok(model)
}

/// Real IVF/AV1 parsing: walks IVF chunks via `bitvue_av1_codec::ivf::parse_ivf_frames`, then
/// parses each frame's OBU frame header for frame-type/QP/ref-frame metadata. Modeled on
/// `bitvue-mcp`'s `parse_ivf_file` (proven working there), adapted to operate on an in-memory
/// slice (via `ByteCache`) instead of re-reading the file, and to compute byte offsets manually
/// since `parse_ivf_frames` doesn't expose them.
fn index_ivf_av1(
    data: &[u8],
    stream: StreamId,
) -> Result<(bitvue_av1_codec::ivf::IvfHeader, Vec<UnitNode>), bitvue_engine::error::BitvueError> {
    let (header, frames) = parse_ivf_frames(data)?;

    let mut units = Vec::with_capacity(frames.len());
    let mut offset = header.header_size as u64;

    for (frame_index, frame) in frames.iter().enumerate() {
        let chunk_size = 12u64 + frame.size as u64;
        let frame_start = offset;

        let obu_header_payload = if !frame.data.is_empty() {
            let obu_header = frame.data[0];
            let obu_header_size = 1 + usize::from((obu_header & 0x04) != 0);
            frame.data.get(obu_header_size..).unwrap_or(&[])
        } else {
            &[][..]
        };
        let frame_header = parse_frame_header_basic(obu_header_payload);

        let frame_type_str = match &frame_header {
            Ok(fh) => match fh.frame_type {
                bitvue_engine::FrameType::Key => "I",
                bitvue_engine::FrameType::Inter => "P",
                bitvue_engine::FrameType::BFrame => "B",
                bitvue_engine::FrameType::IntraOnly => "I",
                bitvue_engine::FrameType::Switch => "I",
                bitvue_engine::FrameType::SI => "I",
                bitvue_engine::FrameType::SP => "P",
                bitvue_engine::FrameType::Unknown => "?",
            },
            Err(_) => "?",
        };

        let mut unit = UnitNode::new(
            stream,
            "FRAME".to_string(),
            frame_start,
            chunk_size as usize,
        );
        unit.frame_index = Some(frame_index);
        unit.frame_type = Some(Arc::from(frame_type_str));
        unit.pts = Some(frame.timestamp);
        unit.temporal_id = Some(frame.temporal_id);
        unit.display_name = Arc::from(format!(
            "Frame {frame_index} @ 0x{frame_start:08X} ({} bytes)",
            frame.size
        ));

        if let Ok(fh) = &frame_header {
            if let Some(ref_idx) = fh.ref_frame_idx {
                unit.ref_frames = Some(ref_idx.iter().map(|&x| x as usize).collect());
            }
            if let Some(qp) = fh.base_q_idx {
                unit.qp_avg = Some(qp);
            }
        }

        units.push(unit);
        offset += chunk_size;
    }

    Ok((header, units))
}

#[cfg(test)]
mod tests;
