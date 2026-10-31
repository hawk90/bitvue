//! IVF loading for the MCP server: open the file in the engine and index it with
//! `bitvue-indexer`, the same pipeline the sidecar uses, so frame type, QP and reference frames
//! come from the real AV1 frame-header parser (with the sequence header and reference-slot state
//! threaded through the stream) and not from a per-frame approximation. See #60.

use bitvue_engine::{Command, Core, Event, StreamId, UnitNode};
use std::path::Path;

pub(crate) fn parse_ivf_file(
    core: &Core,
    path: &Path,
    stream_id: StreamId,
) -> Result<Vec<UnitNode>, String> {
    let mut messages = Vec::new();
    let mut collect = |events: Vec<Event>| {
        for event in events {
            if let Event::DiagnosticAdded { diagnostic } = event {
                messages.push(diagnostic.message);
            }
        }
    };

    collect(core.handle_command(Command::OpenFile {
        stream: stream_id,
        path: path.to_path_buf(),
    }));
    collect(bitvue_indexer::index_stream(core, stream_id));

    let units = core
        .get_stream(stream_id)
        .read()
        .units
        .as_ref()
        .map(|model| model.units.clone());
    match units {
        Some(units) if !units.is_empty() => {
            tracing::info!("Indexed {} units from {}", units.len(), path.display());
            Ok(units)
        }
        _ if messages.is_empty() => Err("No frames found in file".to_string()),
        _ => Err(messages.join("; ")),
    }
}
