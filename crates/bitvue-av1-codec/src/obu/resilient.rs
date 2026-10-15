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

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_engine::event::Severity;

    const TD: [u8; 2] = [0x12, 0x00];
    /// forbidden bit set: always a parse error, consumed one byte at a time.
    const BAD: u8 = 0x80;

    fn run(data: &[u8]) -> (Vec<Obu>, Vec<bitvue_engine::event::Diagnostic>) {
        parse_all_obus_resilient(data, bitvue_engine::StreamId::A)
    }

    #[test]
    fn enough_valid_obus_between_errors_keep_the_scan_going_past_ten_diagnostics() {
        // A valid OBU then one bad byte, fifteen times: 15 diagnostics, 15 OBUs.
        let mut data = Vec::new();
        for _ in 0..15 {
            data.extend(TD);
            data.push(BAD);
        }
        let (obus, diags) = run(&data);
        assert_eq!(obus.len(), 15);
        assert_eq!(diags.len(), 15);
        assert!(diags.iter().all(|d| d.severity != Severity::Fatal));
        assert!(diags.iter().all(|d| !d.message.contains("Too many")));
    }

    #[test]
    fn ten_errors_in_a_row_stop_the_scan_with_one_fatal_diagnostic() {
        let mut data = TD.to_vec();
        data.extend([BAD; 40]);
        data.extend(TD); // never reached
        let (obus, diags) = run(&data);
        assert_eq!(obus.len(), 1, "the scan stopped before the trailing OBU");
        assert_eq!(diags.len(), 11, "ten errors and the fatal one");
        let last = diags.last().unwrap();
        assert_eq!(last.severity, Severity::Fatal);
        assert!(last.message.contains("Too many parse errors"));
        assert_eq!(last.count, 10);
        assert!(diags[..10]
            .iter()
            .all(|d| d.severity != Severity::Fatal || d.message.contains("end")));
    }

    #[test]
    fn the_limit_is_ten_diagnostics_that_also_outnumber_the_obus_found_so_far() {
        // Ten valid OBUs first: ten errors in a row are then not more than the OBUs, so the scan
        // goes on to the OBU after them.
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend(TD);
        }
        data.extend([BAD; 10]);
        data.extend(TD);
        let (obus, diags) = run(&data);
        assert_eq!(obus.len(), 11, "the trailing OBU is reached");
        assert_eq!(diags.len(), 10);
        assert!(diags.iter().all(|d| !d.message.contains("Too many")));

        // An eleventh error in a row is more than the ten OBUs: stop, before the trailing OBU.
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend(TD);
        }
        data.extend([BAD; 11]);
        data.extend(TD);
        let (obus, diags) = run(&data);
        assert_eq!(obus.len(), 10);
        assert_eq!(diags.len(), 12, "eleven errors and the fatal one");
        assert!(diags
            .last()
            .unwrap()
            .message
            .contains("Too many parse errors"));
        assert_eq!(diags.last().unwrap().count, 11);

        // Fewer OBUs mean the limit is reached at ten errors.
        let mut data = Vec::new();
        for _ in 0..3 {
            data.extend(TD);
        }
        data.extend([BAD; 30]);
        let (obus, diags) = run(&data);
        assert_eq!(obus.len(), 3);
        assert_eq!(diags.len(), 11);
        assert_eq!(diags.last().unwrap().count, 10);
    }

    #[test]
    fn diagnostics_are_numbered_in_order_and_carry_the_byte_offset() {
        let mut data = TD.to_vec();
        data.push(BAD);
        data.extend(TD);
        data.push(BAD);
        let (obus, diags) = run(&data);
        assert_eq!(obus.len(), 2);
        assert_eq!(diags.iter().map(|d| d.id).collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(
            diags.iter().map(|d| d.offset_bytes).collect::<Vec<_>>(),
            vec![2, 5]
        );
        assert_eq!(
            diags.iter().map(|d| d.frame_index).collect::<Vec<_>>(),
            vec![Some(1), Some(2)]
        );
    }
}
