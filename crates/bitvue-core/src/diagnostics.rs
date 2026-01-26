//! Error & Degrade UI - T8-2
//!
//! Per ERROR_MODEL.md:
//! - Severity levels: INFO, WARN, ERROR, FATAL
//! - Immutable diagnostic records
//! - Error surfacing in status bar and diagnostics panel
//! - Recovery rules for parsing/decode/fatal errors
//! - Jump/tri-sync integration
//! - Worker error handling
//!
//! Per EDGE_CASES_AND_DEGRADE_BEHAVIOR.md:
//! - Graceful degradation when features unavailable
//! - Clear messaging about why features disabled
//! - Fallback modes for PTS/resolution/alignment issues

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Diagnostic severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DiagnosticSeverity {
    /// Informational (non-actionable)
    Info,
    /// Spec anomalies, recoverable parsing issues
    Warn,
    /// Decoding/parsing failure for unit/frame, recovery possible
    Error,
    /// Cannot continue analysis for stream (app must not crash)
    Fatal,
}

impl DiagnosticSeverity {
    /// Get display text
    pub fn display_text(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Info => "INFO",
            DiagnosticSeverity::Warn => "WARN",
            DiagnosticSeverity::Error => "ERROR",
            DiagnosticSeverity::Fatal => "FATAL",
        }
    }

    /// Get short code
    pub fn short_code(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Info => "I",
            DiagnosticSeverity::Warn => "W",
            DiagnosticSeverity::Error => "E",
            DiagnosticSeverity::Fatal => "F",
        }
    }

    /// Check if actionable (WARN or worse)
    pub fn is_actionable(&self) -> bool {
        matches!(
            self,
            DiagnosticSeverity::Warn | DiagnosticSeverity::Error | DiagnosticSeverity::Fatal
        )
    }
}

/// Diagnostic category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    /// Container format issues
    Container,
    /// Bitstream syntax issues
    Bitstream,
    /// Decode errors
    Decode,
    /// Metric calculation issues
    Metric,
    /// I/O errors
    IO,
    /// Worker failures
    Worker,
}

impl DiagnosticCategory {
    /// Get display text
    pub fn display_text(&self) -> &'static str {
        match self {
            DiagnosticCategory::Container => "Container",
            DiagnosticCategory::Bitstream => "Bitstream",
            DiagnosticCategory::Decode => "Decode",
            DiagnosticCategory::Metric => "Metric",
            DiagnosticCategory::IO => "I/O",
            DiagnosticCategory::Worker => "Worker",
        }
    }
}

/// Diagnostic record
///
/// Immutable record of a diagnostic event.
/// Per ERROR_MODEL.md invariants:
/// - offset_bytes MUST always be present
/// - bit_range must be within unit range if unit_key exists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Unique diagnostic ID
    pub id: u64,

    /// Severity level
    pub severity: DiagnosticSeverity,

    /// Stream ID (A or B)
    pub stream_id: crate::StreamId,

    /// Diagnostic message
    pub message: String,

    /// Category
    pub category: DiagnosticCategory,

    /// Byte offset in stream
    pub offset_bytes: u64,

    /// Bit range (start, end) - exclusive end
    pub bit_range: Option<(u64, u64)>,

    /// Frame reference (if applicable)
    pub frame_key: Option<crate::FrameKey>,

    /// Unit reference (if applicable)
    pub unit_key: Option<crate::UnitKey>,

    /// Codec name (if applicable)
    pub codec: Option<String>,

    /// Timestamp (ms since epoch)
    pub timestamp_ms: u64,

    /// Extra details
    pub details: HashMap<String, String>,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(
        id: u64,
        severity: DiagnosticSeverity,
        stream_id: crate::StreamId,
        message: String,
        category: DiagnosticCategory,
        offset_bytes: u64,
    ) -> Self {
        Self {
            id,
            severity,
            stream_id,
            message,
            category,
            offset_bytes,
            bit_range: None,
            frame_key: None,
            unit_key: None,
            codec: None,
            timestamp_ms: 0, // Would be set from system time
            details: HashMap::new(),
        }
    }

    /// Set bit range
    pub fn with_bit_range(mut self, start: u64, end: u64) -> Self {
        self.bit_range = Some((start, end));
        self
    }

    /// Set frame reference
    pub fn with_frame(mut self, frame_key: crate::FrameKey) -> Self {
        self.frame_key = Some(frame_key);
        self
    }

    /// Set unit reference
    pub fn with_unit(mut self, unit_key: crate::UnitKey) -> Self {
        self.unit_key = Some(unit_key);
        self
    }

    /// Set codec
    pub fn with_codec(mut self, codec: String) -> Self {
        self.codec = Some(codec);
        self
    }

    /// Add detail
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }

    /// Get short summary for status bar
    pub fn short_summary(&self) -> String {
        format!(
            "[{}] {}: {}",
            self.severity.short_code(),
            self.category.display_text(),
            self.message.chars().take(50).collect::<String>()
        )
    }

    /// Get full formatted message
    pub fn full_message(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "[{}] {}: {}",
            self.severity.display_text(),
            self.category.display_text(),
            self.message
        ));

        lines.push(format!(
            "Offset: 0x{:X} ({} bytes)",
            self.offset_bytes, self.offset_bytes
        ));

        if let Some((start, end)) = self.bit_range {
            lines.push(format!(
                "Bit range: {}..{} ({} bits)",
                start,
                end,
                end - start
            ));
        }

        if let Some(ref frame) = self.frame_key {
            lines.push(format!("Frame: {}", frame.frame_index));
        }

        if let Some(ref codec) = self.codec {
            lines.push(format!("Codec: {}", codec));
        }

        if !self.details.is_empty() {
            lines.push("Details:".to_string());
            for (key, value) in &self.details {
                lines.push(format!("  {}: {}", key, value));
            }
        }

        lines.join("\n")
    }
}

/// Diagnostics manager
///
/// Manages diagnostic records and provides query/filtering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticsManager {
    /// All diagnostics
    pub diagnostics: Vec<Diagnostic>,

    /// Next diagnostic ID
    next_id: u64,
}

impl DiagnosticsManager {
    /// Create new diagnostics manager
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            next_id: 1,
        }
    }

    /// Add diagnostic
    pub fn add(&mut self, mut diagnostic: Diagnostic) -> u64 {
        diagnostic.id = self.next_id;
        self.next_id += 1;
        self.diagnostics.push(diagnostic.clone());
        diagnostic.id
    }

    /// Add diagnostic with builder
    pub fn add_diagnostic(
        &mut self,
        severity: DiagnosticSeverity,
        stream_id: crate::StreamId,
        message: String,
        category: DiagnosticCategory,
        offset_bytes: u64,
    ) -> u64 {
        let diagnostic = Diagnostic::new(
            0, // Will be set by add()
            severity,
            stream_id,
            message,
            category,
            offset_bytes,
        );
        self.add(diagnostic)
    }

    /// Get diagnostic by ID
    pub fn get(&self, id: u64) -> Option<&Diagnostic> {
        self.diagnostics.iter().find(|d| d.id == id)
    }

    /// Filter by severity
    pub fn filter_by_severity(&self, min_severity: DiagnosticSeverity) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity >= min_severity)
            .collect()
    }

    /// Filter by category
    pub fn filter_by_category(&self, category: DiagnosticCategory) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.category == category)
            .collect()
    }

    /// Filter by stream
    pub fn filter_by_stream(&self, stream_id: crate::StreamId) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.stream_id == stream_id)
            .collect()
    }

    /// Get diagnostics for frame
    pub fn get_for_frame(&self, frame_index: usize) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.frame_key.as_ref().map(|fk| fk.frame_index) == Some(frame_index))
            .collect()
    }

    /// Get diagnostics in byte range
    pub fn get_in_byte_range(&self, start: u64, end: u64) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.offset_bytes >= start && d.offset_bytes < end)
            .collect()
    }

    /// Count by severity
    pub fn count_by_severity(&self) -> SeverityCounts {
        let mut counts = SeverityCounts::default();

        for diagnostic in &self.diagnostics {
            match diagnostic.severity {
                DiagnosticSeverity::Info => counts.info += 1,
                DiagnosticSeverity::Warn => counts.warn += 1,
                DiagnosticSeverity::Error => counts.error += 1,
                DiagnosticSeverity::Fatal => counts.fatal += 1,
            }
        }

        counts
    }

    /// Get last error summary for status bar
    pub fn last_error_summary(&self) -> Option<String> {
        self.diagnostics
            .iter()
            .rev()
            .find(|d| d.severity.is_actionable())
            .map(|d| d.short_summary())
    }

    /// Clear all diagnostics
    pub fn clear(&mut self) {
        self.diagnostics.clear();
        self.next_id = 1;
    }

    /// Clear diagnostics for stream
    pub fn clear_stream(&mut self, stream_id: crate::StreamId) {
        self.diagnostics.retain(|d| d.stream_id != stream_id);
    }
}

/// Severity counts
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub info: usize,
    pub warn: usize,
    pub error: usize,
    pub fatal: usize,
}

impl SeverityCounts {
    /// Get total count
    pub fn total(&self) -> usize {
        self.info + self.warn + self.error + self.fatal
    }

    /// Check if has actionable issues
    pub fn has_issues(&self) -> bool {
        self.warn > 0 || self.error > 0 || self.fatal > 0
    }

    /// Format for status bar (W/E/F)
    pub fn status_bar_text(&self) -> String {
        if self.has_issues() {
            format!("W:{} E:{} F:{}", self.warn, self.error, self.fatal)
        } else {
            "No issues".to_string()
        }
    }
}

/// Diagnostics filter for panel display
///
/// Per COMPETITOR_PARITY_STATUS.md §4.3:
/// Diagnostics list + jump + filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticsFilter {
    /// Filter by minimum severity (None = all)
    pub min_severity: Option<DiagnosticSeverity>,

    /// Filter by categories (empty = all)
    pub categories: Vec<DiagnosticCategory>,

    /// Filter by stream (None = all)
    pub stream_id: Option<crate::StreamId>,

    /// Filter by text search (case-insensitive substring)
    pub text_search: Option<String>,

    /// Filter by frame range (start, end inclusive)
    pub frame_range: Option<(usize, usize)>,

    /// Filter by byte offset range
    pub byte_range: Option<(u64, u64)>,
}

impl DiagnosticsFilter {
    /// Check if a diagnostic matches the filter
    pub fn matches(&self, diag: &Diagnostic) -> bool {
        // Severity filter
        if let Some(min_sev) = &self.min_severity {
            if diag.severity < *min_sev {
                return false;
            }
        }

        // Category filter
        if !self.categories.is_empty() && !self.categories.contains(&diag.category) {
            return false;
        }

        // Stream filter
        if let Some(stream_id) = &self.stream_id {
            if diag.stream_id != *stream_id {
                return false;
            }
        }

        // Text search filter
        if let Some(ref search) = self.text_search {
            let search_lower = search.to_lowercase();
            if !diag.message.to_lowercase().contains(&search_lower) {
                return false;
            }
        }

        // Frame range filter
        if let Some((start, end)) = self.frame_range {
            if let Some(ref frame_key) = diag.frame_key {
                if frame_key.frame_index < start || frame_key.frame_index > end {
                    return false;
                }
            } else {
                // No frame key and frame filter active = exclude
                return false;
            }
        }

        // Byte range filter
        if let Some((start, end)) = self.byte_range {
            if diag.offset_bytes < start || diag.offset_bytes > end {
                return false;
            }
        }

        true
    }

    /// Clear all filters
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set minimum severity filter
    pub fn with_min_severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.min_severity = Some(severity);
        self
    }

    /// Set categories filter
    pub fn with_categories(mut self, categories: Vec<DiagnosticCategory>) -> Self {
        self.categories = categories;
        self
    }

    /// Set text search filter
    pub fn with_text_search(mut self, search: impl Into<String>) -> Self {
        self.text_search = Some(search.into());
        self
    }
}

/// Sort column for diagnostics panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DiagnosticsSortColumn {
    #[default]
    Id,
    Severity,
    Category,
    OffsetBytes,
    Frame,
    Timestamp,
}

/// Diagnostics panel view
///
/// Provides filtered, sorted view for UI display.
/// Per COMPETITOR_PARITY_STATUS.md §4.3: Filterable table with double-click jump
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsPanel {
    /// Current filter
    pub filter: DiagnosticsFilter,

    /// Sort column
    pub sort_column: DiagnosticsSortColumn,

    /// Sort direction (true = ascending)
    pub sort_ascending: bool,

    /// Currently selected diagnostic ID
    pub selected_id: Option<u64>,

    /// Page size for pagination
    pub page_size: usize,

    /// Current page (0-indexed)
    pub current_page: usize,
}

impl Default for DiagnosticsPanel {
    fn default() -> Self {
        Self {
            filter: DiagnosticsFilter::default(),
            sort_column: DiagnosticsSortColumn::default(),
            sort_ascending: true,
            selected_id: None,
            page_size: 50,
            current_page: 0,
        }
    }
}

impl DiagnosticsPanel {
    /// Create new diagnostics panel
    pub fn new() -> Self {
        Self::default()
    }

    /// Get filtered and sorted view of diagnostics
    pub fn get_view<'a>(&self, manager: &'a DiagnosticsManager) -> Vec<&'a Diagnostic> {
        let mut view: Vec<&Diagnostic> = manager
            .diagnostics
            .iter()
            .filter(|d| self.filter.matches(d))
            .collect();

        // Sort
        view.sort_by(|a, b| {
            let cmp = match self.sort_column {
                DiagnosticsSortColumn::Id => a.id.cmp(&b.id),
                DiagnosticsSortColumn::Severity => a.severity.cmp(&b.severity),
                DiagnosticsSortColumn::Category => {
                    a.category.display_text().cmp(b.category.display_text())
                }
                DiagnosticsSortColumn::OffsetBytes => a.offset_bytes.cmp(&b.offset_bytes),
                DiagnosticsSortColumn::Frame => {
                    let a_frame = a.frame_key.as_ref().map(|fk| fk.frame_index);
                    let b_frame = b.frame_key.as_ref().map(|fk| fk.frame_index);
                    a_frame.cmp(&b_frame)
                }
                DiagnosticsSortColumn::Timestamp => a.timestamp_ms.cmp(&b.timestamp_ms),
            };

            if self.sort_ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

        view
    }

    /// Get paginated view
    pub fn get_page<'a>(&self, manager: &'a DiagnosticsManager) -> Vec<&'a Diagnostic> {
        let view = self.get_view(manager);
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(view.len());

        if start >= view.len() {
            Vec::new()
        } else {
            view[start..end].to_vec()
        }
    }

    /// Get total page count
    pub fn total_pages(&self, manager: &DiagnosticsManager) -> usize {
        let count = self.filtered_count(manager);
        count.div_ceil(self.page_size)
    }

    /// Get filtered count
    pub fn filtered_count(&self, manager: &DiagnosticsManager) -> usize {
        manager
            .diagnostics
            .iter()
            .filter(|d| self.filter.matches(d))
            .count()
    }

    /// Toggle sort column (changes direction if same column)
    pub fn toggle_sort(&mut self, column: DiagnosticsSortColumn) {
        if self.sort_column == column {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
        self.current_page = 0; // Reset to first page
    }

    /// Select diagnostic and return frame key for jump (if available)
    pub fn select_diagnostic(
        &mut self,
        manager: &DiagnosticsManager,
        diag_id: u64,
    ) -> Option<crate::FrameKey> {
        self.selected_id = Some(diag_id);
        manager.get(diag_id).and_then(|d| d.frame_key.clone())
    }

    /// Get jump target for selected diagnostic
    pub fn get_jump_target(&self, manager: &DiagnosticsManager) -> Option<DiagnosticJumpTarget> {
        let diag_id = self.selected_id?;
        let diag = manager.get(diag_id)?;

        Some(DiagnosticJumpTarget {
            frame_key: diag.frame_key.clone(),
            unit_key: diag.unit_key.clone(),
            offset_bytes: diag.offset_bytes,
            bit_range: diag.bit_range,
        })
    }

    /// Navigate to next page
    pub fn next_page(&mut self, manager: &DiagnosticsManager) {
        let total = self.total_pages(manager);
        if self.current_page + 1 < total {
            self.current_page += 1;
        }
    }

    /// Navigate to previous page
    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
        }
    }

    /// Go to first page
    pub fn first_page(&mut self) {
        self.current_page = 0;
    }

    /// Go to last page
    pub fn last_page(&mut self, manager: &DiagnosticsManager) {
        let total = self.total_pages(manager);
        self.current_page = if total > 0 { total - 1 } else { 0 };
    }

    /// Get summary for status bar
    pub fn get_summary(&self, manager: &DiagnosticsManager) -> DiagnosticsSummary {
        let counts = manager.count_by_severity();
        let filtered = self.filtered_count(manager);
        let total = manager.diagnostics.len();

        DiagnosticsSummary {
            total,
            filtered,
            counts,
            current_page: self.current_page,
            total_pages: self.total_pages(manager),
        }
    }
}

/// Jump target from diagnostic selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticJumpTarget {
    /// Frame to jump to
    pub frame_key: Option<crate::FrameKey>,

    /// Unit to select
    pub unit_key: Option<crate::UnitKey>,

    /// Byte offset to scroll to
    pub offset_bytes: u64,

    /// Bit range to highlight
    pub bit_range: Option<(u64, u64)>,
}

/// Panel summary for status bar display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsSummary {
    /// Total diagnostic count
    pub total: usize,

    /// Filtered count
    pub filtered: usize,

    /// Severity counts
    pub counts: SeverityCounts,

    /// Current page
    pub current_page: usize,

    /// Total pages
    pub total_pages: usize,
}

impl DiagnosticsSummary {
    /// Format for panel header
    pub fn header_text(&self) -> String {
        format!(
            "Diagnostics: {} / {} | {}",
            self.filtered,
            self.total,
            self.counts.status_bar_text()
        )
    }

    /// Format pagination text
    pub fn pagination_text(&self) -> String {
        format!(
            "Page {} / {}",
            self.current_page + 1,
            self.total_pages.max(1)
        )
    }
}

/// Degrade mode for feature degradation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradeMode {
    /// Feature fully available
    Available,

    /// Feature degraded but usable
    Degraded,

    /// Feature unavailable
    Unavailable,
}

/// Feature degrade state
///
/// Tracks availability and degradation reasons for features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDegradeState {
    /// Degrade mode
    pub mode: DegradeMode,

    /// Feature name
    pub feature_name: String,

    /// Reason for degradation/unavailability
    pub reason: Option<String>,

    /// Suggested action (if any)
    pub suggested_action: Option<String>,
}

impl FeatureDegradeState {
    /// Create available feature
    pub fn available(feature_name: impl Into<String>) -> Self {
        Self {
            mode: DegradeMode::Available,
            feature_name: feature_name.into(),
            reason: None,
            suggested_action: None,
        }
    }

    /// Create degraded feature
    pub fn degraded(feature_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            mode: DegradeMode::Degraded,
            feature_name: feature_name.into(),
            reason: Some(reason.into()),
            suggested_action: None,
        }
    }

    /// Create unavailable feature
    pub fn unavailable(feature_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            mode: DegradeMode::Unavailable,
            feature_name: feature_name.into(),
            reason: Some(reason.into()),
            suggested_action: None,
        }
    }

    /// Set suggested action
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.suggested_action = Some(action.into());
        self
    }

    /// Get user-facing message
    pub fn message(&self) -> String {
        let mut msg = format!("{}: ", self.feature_name);

        msg.push_str(match self.mode {
            DegradeMode::Available => "Available",
            DegradeMode::Degraded => "Degraded",
            DegradeMode::Unavailable => "Unavailable",
        });

        if let Some(ref reason) = self.reason {
            msg.push_str(&format!(" - {}", reason));
        }

        if let Some(ref action) = self.suggested_action {
            msg.push_str(&format!("\nSuggested: {}", action));
        }

        msg
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("diagnostics_test.rs");
