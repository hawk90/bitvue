//! Metadata-indexing pipeline for `bitvue_engine::Core`.
//!
//! Populates `StreamState.container`/`.units` from a real file -- no pixel decode. This is
//! deliberately the *first* stage only (see docs/DEVELOPMENT_PHASES.md's decode-pipeline design
//! discussion, 2026-08-08), and only IVF/AV1 is supported so far. Other formats/codecs get an
//! honest `Event::DiagnosticAdded`, not a fabricated result.
//!
//! `.syntax` IS populated, but lazily: [`get_frame_syntax`] parses one unit's syntax tree on
//! demand (mirrors `Core::handle_command`'s `SelectBitRange`, which already does an on-demand
//! nearest-node lookup against whatever `.syntax` happens to hold) rather than eagerly building a
//! tree for every unit in [`index_stream`] -- that would be wasted work for units the UI never
//! looks at, and real cost on long streams.
//!
//! [`get_timeline`] builds a real `bitvue_engine::timeline::TimelineBase` from already-indexed
//! units via the existing (previously never-called) `frame_identity::TimelineMapper` -- see that
//! function's doc for why `StreamState.timeline: Option<TimelineModel>` is deliberately NOT
//! written to (that field's type is confirmed dead/vestigial code, not the real timeline design).
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
use bitvue_engine::frame_identity::{FrameMetadata, TimelineMapper};
use bitvue_engine::timeline::TimelineBase;
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

/// Same shape as [`diagnostic_event`], but for a real per-frame parse issue found inside
/// [`index_ivf_av1`] (Warn, not Error -- indexing still produces a usable, if partial, result for
/// this frame, unlike the whole-request-fatal cases `diagnostic_event` covers).
fn frame_parse_diagnostic(
    stream: StreamId,
    frame_index: usize,
    message: String,
) -> bitvue_engine::Event {
    bitvue_engine::Event::DiagnosticAdded {
        diagnostic: Diagnostic {
            id: NEXT_DIAGNOSTIC_ID.fetch_add(1, Ordering::Relaxed),
            severity: Severity::Warn,
            stream_id: stream,
            message,
            category: Category::Bitstream,
            offset_bytes: 0,
            timestamp_ms: now_ms(),
            frame_index: Some(frame_index),
            count: 1,
            impact_score: 30,
        },
    }
}

/// Index the given stream's already-open file: detect the container, parse its units, and write
/// `ContainerModel`/`UnitModel` into `StreamState`. Returns the resulting events (mirrors
/// `Core::handle_command`'s convention of reporting failures as `DiagnosticAdded` events rather
/// than a `Result`, since that's what every other stream-mutating operation in this codebase does).
///
/// Not cancellable -- convenience wrapper over [`index_stream_with_cancel`] with a flag that's
/// never set, for callers (all of this crate's own tests) that have no cancellation context.
pub fn index_stream(core: &Core, stream: StreamId) -> Vec<bitvue_engine::Event> {
    index_stream_with_cancel(core, stream, &std::sync::atomic::AtomicBool::new(false)).0
}

/// Same as [`index_stream`], but cooperatively cancellable: `cancel_flag` is checked once per
/// parsed IVF frame inside [`index_ivf_av1`], the only genuinely long-running loop in this crate
/// (a whole stream's worth of per-frame OBU header parses, no pixel decode but still real work on
/// a long stream). `bitvue-sidecar`'s `index_stream` command handler is the real caller -- see its
/// module doc's "Concurrency model" section.
///
/// Returns `(events, cancelled)`. When `cancelled` is `true`, `events` is empty and
/// `StreamState.container`/`.units` are deliberately left untouched -- a cancelled index run
/// writes no partial state, matching "an aborted request has no observable effect other than not
/// completing" for every other command in this codebase.
pub fn index_stream_with_cancel(
    core: &Core,
    stream: StreamId,
    cancel_flag: &std::sync::atomic::AtomicBool,
) -> (Vec<bitvue_engine::Event>, bool) {
    let byte_cache = {
        let stream_state = core.get_stream(stream);
        let state = stream_state.read();
        match state.byte_cache.clone() {
            Some(cache) => cache,
            None => {
                return (
                    vec![diagnostic_event(
                        stream,
                        "No file open for this stream".to_string(),
                    )],
                    false,
                )
            }
        }
    };

    let len = byte_cache.len() as usize;
    let data = match byte_cache.read_range(0, len) {
        Ok(d) => d,
        Err(e) => {
            return (
                vec![diagnostic_event(
                    stream,
                    format!("Failed to read file: {e}"),
                )],
                false,
            )
        }
    };

    if data.len() < 4 || &data[0..4] != b"DKIF" {
        // Honest scope limit: MP4/MKV/TS containers and non-AV1 codecs aren't indexed yet.
        return (
            vec![diagnostic_event(
                stream,
                "Indexing is only implemented for IVF/AV1 streams so far".to_string(),
            )],
            false,
        );
    }

    let (ivf_header, units, frame_diagnostics) = match index_ivf_av1(data, stream, cancel_flag) {
        Ok(Some(result)) => result,
        Ok(None) => return (Vec::new(), true),
        Err(e) => {
            return (
                vec![diagnostic_event(
                    stream,
                    format!("Failed to parse IVF stream: {e}"),
                )],
                false,
            )
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

    let mut events = vec![
        bitvue_engine::Event::ModelUpdated {
            kind: bitvue_engine::event::ModelKind::Container,
            stream,
        },
        bitvue_engine::Event::ModelUpdated {
            kind: bitvue_engine::event::ModelKind::Units,
            stream,
        },
    ];
    events.extend(frame_diagnostics);

    (events, false)
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

    // unit_offset/unit_size cover the full IVF chunk (12-byte chunk header + OBU-container
    // payload, see index_ivf_av1's UnitNode construction) -- skip the chunk header to get the
    // raw OBU-container bytes.
    if unit_size <= 12 {
        return Err(format!(
            "Unit at offset {unit_offset} is too small to contain OBU data"
        ));
    }
    let chunk_offset = unit_offset + 12;
    let chunk_len = unit_size - 12;
    let chunk_bytes = byte_cache
        .read_range(chunk_offset, chunk_len)
        .map_err(|e| format!("Failed to read OBU bytes: {e}"))?;

    // An IVF chunk isn't one OBU -- it's typically a Temporal Delimiter followed by the actual
    // Frame/FrameHeader OBU (an earlier version of this function assumed byte 0 was the frame
    // header directly, which parsed the TD's leftover bytes instead; see find_frame_obu's doc).
    let found = find_frame_obu(chunk_bytes)
        .ok_or_else(|| "No Frame/FrameHeader OBU found in this unit's chunk".to_string())?;
    let obu_bytes = &chunk_bytes[found.offset..found.offset + found.consumed];
    // parse_obu_syntax's global_offset is a BIT offset from file start (see
    // TrackedBitReader::new's doc), not a byte offset -- matches the CLI's analyze.rs convention
    // ((offset * 8) as u64), here applied to the found OBU's own absolute file offset (chunk
    // start + its offset within the chunk), not the chunk's own start.
    let obu_file_offset = chunk_offset + found.offset as u64;
    let model = parse_obu_syntax(obu_bytes, frame_index, obu_file_offset * 8)
        .map_err(|e| format!("Failed to parse OBU syntax: {e}"))?;

    {
        let stream_state = core.get_stream(stream);
        let mut state = stream_state.write();
        state.syntax = Some(model.clone());
    }

    Ok(model)
}

/// Builds a real display-order timeline from already-indexed units, via
/// `frame_identity::TimelineMapper`/`Av1TimelineExtractor` -- code that already existed in
/// `bitvue-engine`, is used by 9+ other modules (lanes, window, evidence, export, picture_stats),
/// and had zero production callers before this. No pixel decode needed: `TimelineMapper` only
/// needs per-frame `{pts, dts}`/size/type in *decode* order and sorts them into display order
/// itself (that's the whole point of the type -- see its own doc for the AV1 reordering case).
///
/// Deliberately does NOT write into `StreamState.timeline: Option<TimelineModel>` -- investigated
/// this before writing any code: `TimelineModel`/`stream_state.rs`'s own `TimelineFrame` have zero
/// non-definition references anywhere in the codebase, are never constructed by any production
/// code, and read as an early "Phase 1" placeholder superseded by the real "T4-1 deliverable"
/// (`timeline::TimelineBase`, this function's return type) that a real subsystem already depends
/// on. Retiring the dead `TimelineModel` type and giving `StreamState` a real timeline slot is a
/// `bitvue-engine`-internal cleanup, out of scope for this crate (which only ever adds data
/// through `Core`'s already-public accessors, never changes `bitvue-engine` itself).
pub fn get_timeline(core: &Core, stream: StreamId) -> Result<TimelineBase, String> {
    Ok(build_timeline_mapper(core, stream)?.build_timeline_av1())
}

/// Builds a real `bitvue_engine::frame_identity::FrameIndexMap` (PTS-based display/decode-order
/// mapping) from an already-indexed stream's units -- the one piece [`crate::compare`]-adjacent
/// callers (e.g. `bitvue-sidecar`'s `create_compare_workspace`) need but can't build themselves
/// (`bitvue-engine` is a leaf crate, see this module's doc). Shared with [`get_timeline`] so the
/// `units` → `Vec<FrameMetadata>` extraction (and its `frame_type` string-translation subtlety,
/// documented below) only exists in one place.
pub fn build_frame_index_map(
    core: &Core,
    stream: StreamId,
) -> Result<bitvue_engine::frame_identity::FrameIndexMap, String> {
    Ok(build_timeline_mapper(core, stream)?.index_map().clone())
}

fn build_timeline_mapper(core: &Core, stream: StreamId) -> Result<TimelineMapper, String> {
    let (units, codec) = {
        let stream_state = core.get_stream(stream);
        let state = stream_state.read();
        let codec = state
            .container
            .as_ref()
            .map(|c| c.codec.clone())
            .unwrap_or_default();
        let units = state
            .units
            .as_ref()
            .ok_or_else(|| "No units indexed for this stream -- has index_stream run?".to_string())?
            .units
            .clone();
        (units, codec)
    };

    if codec != "av1" {
        return Err(format!(
            "Timeline building is only implemented for AV1 so far (this stream's codec: {codec:?})"
        ));
    }

    // Units are already in decode order (index_ivf_av1 assigns frame_index sequentially as it
    // walks the IVF chunk list) -- exactly what TimelineMapper::new expects.
    let frame_units: Vec<&UnitNode> = units.iter().filter(|u| u.frame_index.is_some()).collect();
    if frame_units.is_empty() {
        return Err("No frame units found -- has index_stream run for this stream?".to_string());
    }

    let frames: Vec<FrameMetadata> = frame_units
        .iter()
        .map(|u| FrameMetadata {
            pts: u.pts,
            dts: None,
        })
        .collect();
    let sizes: Vec<u64> = frame_units.iter().map(|u| u.size as u64).collect();
    // Av1TimelineExtractor::determine_marker matches literal "KEY_FRAME"/"INTRA_ONLY_FRAME"
    // strings -- NOT the generic default extractor's "I" shortcut, which index_ivf_av1's short
    // codes ("I"/"P"/"B") would otherwise silently fail to match, marking zero keyframes in an
    // otherwise-correct timeline. Translate at this call boundary only -- UnitNode.frame_type
    // itself stays "I"/"P"/"B" everywhere else (frontend, syntax display, etc.).
    let types: Vec<String> = frame_units
        .iter()
        .map(|u| match u.frame_type.as_deref() {
            Some("I") => "KEY_FRAME",
            Some("P") | Some("B") => "INTER_FRAME",
            _ => "INTER_FRAME",
        })
        .map(String::from)
        .collect();

    Ok(TimelineMapper::new(
        format!("{stream:?}"),
        frames,
        sizes,
        types,
    ))
}

/// Finds the Frame/FrameHeader OBU within one IVF chunk's OBU-container bytes. An IVF chunk is
/// NOT one OBU -- it's typically a Temporal Delimiter (a tiny, mostly-empty OBU) followed by the
/// actual Frame/FrameHeader OBU. A real bug (found via a visual screenshot check, not caught by
/// any unit test until one was added afterward) came from an earlier version of this crate
/// treating the first OBU in each chunk as the frame header directly -- every single frame in a
/// 250-frame fixture silently came back as frame_type "I", because `parse_frame_header_basic` was
/// parsing the Temporal Delimiter's leftover bytes as if they were frame-header syntax. Returns
/// `None` if no Frame/FrameHeader OBU is found (malformed chunk, or every OBU fails to parse).
fn find_frame_obu(chunk_data: &[u8]) -> Option<bitvue_av1_codec::obu::ObuWithOffset> {
    let mut iter = bitvue_av1_codec::obu::ObuIterator::new(chunk_data);
    while let Some(result) = iter.next_obu_with_offset() {
        let Ok(found) = result else { continue };
        match found.obu.header.obu_type {
            bitvue_av1_codec::obu::ObuType::Frame | bitvue_av1_codec::obu::ObuType::FrameHeader => {
                return Some(found)
            }
            _ => continue,
        }
    }
    None
}

/// Real IVF/AV1 parsing: walks IVF chunks via `bitvue_av1_codec::ivf::parse_ivf_frames`, then
/// parses each frame's real Frame/FrameHeader OBU (found via [`find_frame_obu`], not assumed to
/// be the first OBU in the chunk) for frame-type/QP/ref-frame metadata. Modeled on
/// `bitvue-mcp`'s `parse_ivf_file` (proven working there) for the container-level walk, adapted
/// to operate on an in-memory slice (via `ByteCache`) instead of re-reading the file, and to
/// compute byte offsets manually since `parse_ivf_frames` doesn't expose them.
/// Returns `Ok(None)` if `cancel_flag` was set before the per-frame loop finished -- checked once
/// per frame, which is coarse enough to not matter perf-wise on the common case (a stream that
/// finishes) while still bailing out promptly on a long stream that gets cancelled.
/// `(header, units, per-frame diagnostics)`.
type IndexIvfAv1Result = (
    bitvue_av1_codec::ivf::IvfHeader,
    Vec<UnitNode>,
    Vec<bitvue_engine::Event>,
);

fn index_ivf_av1(
    data: &[u8],
    stream: StreamId,
    cancel_flag: &std::sync::atomic::AtomicBool,
) -> Result<Option<IndexIvfAv1Result>, bitvue_engine::error::BitvueError> {
    let (header, frames) = parse_ivf_frames(data)?;

    let mut diagnostics = Vec::new();
    // Real, previously-silent gap: the fourcc is parsed but was never checked against what this
    // function actually parses (AV1 OBUs) -- an IVF file with any other fourcc (VP9, etc.) would
    // silently mis-parse every frame as AV1 with no indication why. Doesn't abort (this function
    // still attempts the parse, same as before -- adding real non-AV1 codec support here is a
    // separate, much larger scope), just makes the resulting garbage/partial data explicable.
    if &header.fourcc != b"AV01" {
        let fourcc_str = String::from_utf8_lossy(&header.fourcc);
        diagnostics.push(diagnostic_event(
            stream,
            format!(
                "IVF fourcc is \"{fourcc_str}\", not \"AV01\" -- only AV1 is currently \
                 supported, frame parsing below is likely to fail or produce incorrect results"
            ),
        ));
    }

    let mut units = Vec::with_capacity(frames.len());
    let mut offset = header.header_size as u64;

    for (frame_index, frame) in frames.iter().enumerate() {
        if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(None);
        }
        let chunk_size = 12u64 + frame.size as u64;
        let frame_start = offset;

        let found_obu = find_frame_obu(&frame.data);
        let frame_header = found_obu
            .as_ref()
            .and_then(|found| parse_frame_header_basic(&found.obu.payload).ok());

        // Real, previously-silent per-frame signal: both steps already fail gracefully (frame
        // just gets frame_type "?", no ref_frames/qp_avg below) -- this makes that visible
        // instead of thrown away, without changing the existing fallback behavior at all.
        if found_obu.is_none() {
            diagnostics.push(frame_parse_diagnostic(
                stream,
                frame_index,
                "No frame OBU found in this IVF chunk".to_string(),
            ));
        } else if frame_header.is_none() {
            diagnostics.push(frame_parse_diagnostic(
                stream,
                frame_index,
                "Failed to parse this frame's AV1 frame header".to_string(),
            ));
        }

        let frame_type_str = match frame_header.as_ref().map(|fh| fh.frame_type) {
            Some(bitvue_engine::FrameType::Key) => "I",
            Some(bitvue_engine::FrameType::Inter) => "P",
            Some(bitvue_engine::FrameType::BFrame) => "B",
            Some(bitvue_engine::FrameType::IntraOnly) => "I",
            Some(bitvue_engine::FrameType::Switch) => "I",
            Some(bitvue_engine::FrameType::SI) => "I",
            Some(bitvue_engine::FrameType::SP) => "P",
            Some(bitvue_engine::FrameType::Unknown) | None => "?",
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

        if let Some(fh) = &frame_header {
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

    Ok(Some((header, units, diagnostics)))
}

#[cfg(test)]
mod tests;
