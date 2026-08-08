//! Diagnostics export (CSV and JSON)

use serde::{Deserialize, Serialize};
use std::io::Write;

use super::types::{ExportFormat, ExportResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity};

/// Diagnostics export row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsExportRow {
    pub id: u64,
    pub severity: String,
    pub category: String,
    pub message: String,
    pub byte_offset: u64,
    pub frame_idx: Option<u64>,
    pub stream_id: String,
}

impl DiagnosticsExportRow {
    pub fn from_diagnostic(diag: &Diagnostic) -> Self {
        Self {
            id: diag.id,
            severity: format!("{:?}", diag.severity),
            category: format!("{:?}", diag.category),
            message: diag.message.clone(),
            byte_offset: diag.offset_bytes,
            frame_idx: diag.frame_key.as_ref().map(|fk| fk.frame_index as u64),
            stream_id: format!("{:?}", diag.stream_id),
        }
    }
}

/// Export diagnostics to CSV
pub fn export_diagnostics_csv<W: Write>(
    diagnostics: &[Diagnostic],
    writer: &mut W,
    min_severity: Option<DiagnosticSeverity>,
) -> std::io::Result<ExportResult> {
    writeln!(
        writer,
        "id,severity,category,message,byte_offset,frame_idx,stream_id"
    )?;

    let mut row_count = 0;
    let mut bytes_written = 0;

    for record in diagnostics {
        // Filter by severity
        if let Some(min_sev) = &min_severity {
            if (record.severity as u8) < (*min_sev as u8) {
                continue;
            }
        }

        let row = DiagnosticsExportRow::from_diagnostic(record);

        // Escape message for CSV (handle commas and quotes)
        let escaped_message = if row.message.contains(',') || row.message.contains('"') {
            format!("\"{}\"", row.message.replace('"', "\"\""))
        } else {
            row.message.clone()
        };

        let line = format!(
            "{},{},{},{},{},{},{}\n",
            row.id,
            row.severity,
            row.category,
            escaped_message,
            row.byte_offset,
            row.frame_idx.map(|v| v.to_string()).unwrap_or_default(),
            row.stream_id,
        );

        bytes_written += writer.write(line.as_bytes())?;
        row_count += 1;
    }

    Ok(ExportResult {
        format: ExportFormat::Csv,
        bytes_written,
        row_count,
    })
}

/// Export diagnostics to JSON
pub fn export_diagnostics_json<W: Write>(
    diagnostics: &[Diagnostic],
    writer: &mut W,
    min_severity: Option<DiagnosticSeverity>,
    pretty: bool,
) -> std::io::Result<ExportResult> {
    let rows: Vec<DiagnosticsExportRow> = diagnostics
        .iter()
        .filter(|r| {
            if let Some(min_sev) = &min_severity {
                (r.severity as u8) >= (*min_sev as u8)
            } else {
                true
            }
        })
        .map(DiagnosticsExportRow::from_diagnostic)
        .collect();

    let row_count = rows.len();

    let json_str = if pretty {
        serde_json::to_string_pretty(&rows).map_err(std::io::Error::other)?
    } else {
        serde_json::to_string(&rows).map_err(std::io::Error::other)?
    };

    let bytes_written = writer.write(json_str.as_bytes())?;

    Ok(ExportResult {
        format: if pretty {
            ExportFormat::JsonPretty
        } else {
            ExportFormat::Json
        },
        bytes_written,
        row_count,
    })
}
