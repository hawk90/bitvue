//! Parsing that survives errors: every failure becomes a diagnostic and the scan goes on.

use bitvue_engine::BitvueError;

use super::parse::parse_obu;
use super::types::Obu;

/// Parses all OBUs from a byte slice with error resilience
///
/// Unlike `parse_all_obus()`, this function continues parsing even when errors occur,
/// collecting diagnostics for each error encountered. This is useful for analyzing
/// corrupt or malformed bitstreams.
///
/// Returns:
/// - Vector of successfully parsed OBUs
/// - Vector of diagnostics for any errors encountered
pub fn parse_all_obus_resilient(
    data: &[u8],
    stream_id: bitvue_engine::StreamId,
) -> (Vec<Obu>, Vec<bitvue_engine::event::Diagnostic>) {
    use bitvue_engine::event::{Category, Diagnostic, Severity};

    let mut obus = Vec::new();
    let mut diagnostics = Vec::new();
    let mut offset = 0usize;
    let mut diagnostic_id = 0u64;

    while offset < data.len() {
        match parse_obu(data, offset) {
            Ok((obu, consumed)) => {
                obus.push(obu);
                offset += consumed;
            }
            Err(e) => {
                // Convert parse error to diagnostic
                let (message, severity, impact_score) = match &e {
                    BitvueError::Parse { message, .. } => (message.clone(), Severity::Error, 85),
                    BitvueError::UnexpectedEof(_) => {
                        ("Unexpected end of file".to_string(), Severity::Fatal, 100)
                    }
                    BitvueError::InvalidObuType(t) => {
                        (format!("Invalid OBU type: {}", t), Severity::Error, 90)
                    }
                    _ => (format!("{:?}", e), Severity::Error, 80),
                };

                let diagnostic = Diagnostic {
                    id: diagnostic_id,
                    severity,
                    stream_id,
                    message,
                    category: Category::Bitstream,
                    offset_bytes: offset as u64,
                    timestamp_ms: 0, // Will be calculated from frame index later
                    frame_index: Some(obus.len()), // Approximate frame index
                    count: 1,
                    impact_score,
                };

                diagnostics.push(diagnostic);
                diagnostic_id += 1;

                // Try to skip past the error and continue parsing
                // Skip 1 byte at a time looking for next valid OBU header
                offset += 1;

                // After 10 consecutive errors, give up
                if diagnostics.len() >= 10 && diagnostics.len() > obus.len() {
                    let fatal_diagnostic = Diagnostic {
                        id: diagnostic_id,
                        severity: Severity::Fatal,
                        stream_id,
                        message: "Too many parse errors, stopping".to_string(),
                        category: Category::Bitstream,
                        offset_bytes: offset as u64,
                        timestamp_ms: 0,
                        frame_index: None,
                        count: diagnostics.len() as u32,
                        impact_score: 100,
                    };
                    diagnostics.push(fatal_diagnostic);
                    break;
                }
            }
        }
    }

    (obus, diagnostics)
}
