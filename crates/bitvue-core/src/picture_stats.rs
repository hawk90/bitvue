//! Picture Stats Table - Feature Parity Phase D
//!
//! Per COMPETITOR_PARITY_STATUS.md §4.3:
//! - Picture stats table (aggregated frame statistics)
//! - Sortable columns
//! - Copy/export row
//!
//! Similar to Elecard StreamEye's "Picture Information" table.

use crate::timeline::{FrameMarker, TimelineFrame};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-frame statistics row for table display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PictureStatsRow {
    /// Display index (PTS order)
    pub display_idx: usize,

    /// Frame type (I, P, B, etc.)
    pub frame_type: String,

    /// Frame size in bytes
    pub size_bytes: u64,

    /// Frame size in bits
    pub size_bits: u64,

    /// Bits per pixel (requires frame dimensions)
    pub bpp: Option<f32>,

    /// Presentation timestamp
    pub pts: Option<u64>,

    /// Decode timestamp
    pub dts: Option<u64>,

    /// PTS - DTS delta (reorder depth)
    pub pts_dts_delta: Option<i64>,

    /// Is keyframe
    pub is_keyframe: bool,

    /// Has error
    pub has_error: bool,

    /// Has bookmark
    pub has_bookmark: bool,

    /// Is scene change
    pub is_scene_change: bool,

    /// QP average (if available from codec)
    pub qp_avg: Option<f32>,

    /// QP min (if available)
    pub qp_min: Option<i8>,

    /// QP max (if available)
    pub qp_max: Option<i8>,
}

impl PictureStatsRow {
    /// Create from TimelineFrame
    pub fn from_timeline_frame(frame: &TimelineFrame, frame_width: u32, frame_height: u32) -> Self {
        let pixel_count = (frame_width as u64) * (frame_height as u64);
        let bpp = if pixel_count > 0 {
            Some((frame.size_bytes * 8) as f32 / pixel_count as f32)
        } else {
            None
        };

        let pts_dts_delta = match (frame.pts, frame.dts) {
            (Some(p), Some(d)) => Some(p as i64 - d as i64),
            _ => None,
        };

        Self {
            display_idx: frame.display_idx,
            frame_type: frame.frame_type.clone(),
            size_bytes: frame.size_bytes,
            size_bits: frame.size_bytes * 8,
            bpp,
            pts: frame.pts,
            dts: frame.dts,
            pts_dts_delta,
            is_keyframe: matches!(frame.marker, FrameMarker::Key),
            has_error: matches!(frame.marker, FrameMarker::Error),
            has_bookmark: matches!(frame.marker, FrameMarker::Bookmark),
            is_scene_change: matches!(frame.marker, FrameMarker::SceneChange),
            qp_avg: None,
            qp_min: None,
            qp_max: None,
        }
    }

    /// Add QP data
    pub fn with_qp(mut self, qp_min: i8, qp_max: i8) -> Self {
        self.qp_min = Some(qp_min);
        self.qp_max = Some(qp_max);
        self.qp_avg = Some((qp_min as f32 + qp_max as f32) / 2.0);
        self
    }
}

/// Sort column for picture stats table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PictureStatsSortColumn {
    DisplayIdx,
    FrameType,
    SizeBytes,
    SizeBits,
    Bpp,
    Pts,
    Dts,
    PtsDtsDelta,
    QpAvg,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(&self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }
}

/// Filter for picture stats table
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PictureStatsFilter {
    /// Filter by frame type (empty = all)
    pub frame_types: Vec<String>,

    /// Only show keyframes
    pub keyframes_only: bool,

    /// Only show frames with errors
    pub errors_only: bool,

    /// Only show bookmarked frames
    pub bookmarks_only: bool,

    /// Only show scene changes
    pub scene_changes_only: bool,

    /// Min size filter (bytes)
    pub min_size_bytes: Option<u64>,

    /// Max size filter (bytes)
    pub max_size_bytes: Option<u64>,

    /// Min BPP filter
    pub min_bpp: Option<f32>,

    /// Max BPP filter
    pub max_bpp: Option<f32>,
}

impl PictureStatsFilter {
    /// Check if a row passes the filter
    pub fn matches(&self, row: &PictureStatsRow) -> bool {
        // Frame type filter
        if !self.frame_types.is_empty() && !self.frame_types.contains(&row.frame_type) {
            return false;
        }

        // Marker filters
        if self.keyframes_only && !row.is_keyframe {
            return false;
        }
        if self.errors_only && !row.has_error {
            return false;
        }
        if self.bookmarks_only && !row.has_bookmark {
            return false;
        }
        if self.scene_changes_only && !row.is_scene_change {
            return false;
        }

        // Size filters
        if let Some(min) = self.min_size_bytes {
            if row.size_bytes < min {
                return false;
            }
        }
        if let Some(max) = self.max_size_bytes {
            if row.size_bytes > max {
                return false;
            }
        }

        // BPP filters
        if let Some(min) = self.min_bpp {
            if row.bpp.is_none_or(|b| b < min) {
                return false;
            }
        }
        if let Some(max) = self.max_bpp {
            if row.bpp.is_none_or(|b| b > max) {
                return false;
            }
        }

        true
    }
}

/// Aggregated statistics for the entire sequence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SequenceStats {
    /// Total frame count
    pub total_frames: usize,

    /// Frame type distribution
    pub frame_type_counts: HashMap<String, usize>,

    /// Frame type percentage
    pub frame_type_percentages: HashMap<String, f32>,

    /// Total size in bytes
    pub total_size_bytes: u64,

    /// Average frame size
    pub avg_size_bytes: f64,

    /// Min frame size
    pub min_size_bytes: u64,

    /// Max frame size
    pub max_size_bytes: u64,

    /// Size standard deviation
    pub size_std_dev: f64,

    /// Keyframe count
    pub keyframe_count: usize,

    /// Error count
    pub error_count: usize,

    /// Scene change count
    pub scene_change_count: usize,

    /// Average BPP (if dimensions known)
    pub avg_bpp: Option<f32>,

    /// I-frame average size
    pub i_frame_avg_size: Option<f64>,

    /// P-frame average size
    pub p_frame_avg_size: Option<f64>,

    /// B-frame average size
    pub b_frame_avg_size: Option<f64>,

    /// Reorder depth max (max |PTS - DTS|)
    pub max_reorder_depth: Option<i64>,

    /// GOP size average (frames between keyframes)
    pub avg_gop_size: Option<f32>,
}

impl SequenceStats {
    /// Calculate from rows
    pub fn from_rows(rows: &[PictureStatsRow]) -> Self {
        if rows.is_empty() {
            return Self::default();
        }

        let total_frames = rows.len();

        // Frame type distribution
        let mut frame_type_counts: HashMap<String, usize> = HashMap::new();
        for row in rows {
            *frame_type_counts.entry(row.frame_type.clone()).or_insert(0) += 1;
        }

        let frame_type_percentages: HashMap<String, f32> = frame_type_counts
            .iter()
            .map(|(k, v)| (k.clone(), (*v as f32 / total_frames as f32) * 100.0))
            .collect();

        // Size statistics
        let total_size_bytes: u64 = rows.iter().map(|r| r.size_bytes).sum();
        let avg_size_bytes = total_size_bytes as f64 / total_frames as f64;
        let min_size_bytes = rows.iter().map(|r| r.size_bytes).min().unwrap_or(0);
        let max_size_bytes = rows.iter().map(|r| r.size_bytes).max().unwrap_or(0);

        // Standard deviation
        let variance: f64 = rows
            .iter()
            .map(|r| {
                let diff = r.size_bytes as f64 - avg_size_bytes;
                diff * diff
            })
            .sum::<f64>()
            / total_frames as f64;
        let size_std_dev = variance.sqrt();

        // Marker counts
        let keyframe_count = rows.iter().filter(|r| r.is_keyframe).count();
        let error_count = rows.iter().filter(|r| r.has_error).count();
        let scene_change_count = rows.iter().filter(|r| r.is_scene_change).count();

        // Average BPP
        let bpp_values: Vec<f32> = rows.iter().filter_map(|r| r.bpp).collect();
        let avg_bpp = if !bpp_values.is_empty() {
            Some(bpp_values.iter().sum::<f32>() / bpp_values.len() as f32)
        } else {
            None
        };

        // Per-type average sizes
        let i_frame_avg_size = Self::avg_size_for_type(rows, "I");
        let p_frame_avg_size = Self::avg_size_for_type(rows, "P");
        let b_frame_avg_size = Self::avg_size_for_type(rows, "B");

        // Max reorder depth
        let max_reorder_depth = rows
            .iter()
            .filter_map(|r| r.pts_dts_delta.map(|d| d.abs()))
            .max();

        // Average GOP size
        let avg_gop_size = if keyframe_count > 1 {
            Some(total_frames as f32 / keyframe_count as f32)
        } else {
            None
        };

        Self {
            total_frames,
            frame_type_counts,
            frame_type_percentages,
            total_size_bytes,
            avg_size_bytes,
            min_size_bytes,
            max_size_bytes,
            size_std_dev,
            keyframe_count,
            error_count,
            scene_change_count,
            avg_bpp,
            i_frame_avg_size,
            p_frame_avg_size,
            b_frame_avg_size,
            max_reorder_depth,
            avg_gop_size,
        }
    }

    fn avg_size_for_type(rows: &[PictureStatsRow], frame_type: &str) -> Option<f64> {
        let matching: Vec<&PictureStatsRow> =
            rows.iter().filter(|r| r.frame_type == frame_type).collect();
        if matching.is_empty() {
            return None;
        }
        let sum: u64 = matching.iter().map(|r| r.size_bytes).sum();
        Some(sum as f64 / matching.len() as f64)
    }
}

/// Picture stats table with sorting and filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PictureStatsTable {
    /// All rows (unsorted, unfiltered)
    rows: Vec<PictureStatsRow>,

    /// Current sort column
    pub sort_column: PictureStatsSortColumn,

    /// Current sort direction
    pub sort_direction: SortDirection,

    /// Current filter
    pub filter: PictureStatsFilter,

    /// Cached sequence statistics
    pub sequence_stats: SequenceStats,

    /// Frame dimensions (for BPP calculation)
    frame_width: u32,
    frame_height: u32,
}

impl PictureStatsTable {
    /// Create new table from timeline frames
    pub fn new(frames: &[TimelineFrame], frame_width: u32, frame_height: u32) -> Self {
        let rows: Vec<PictureStatsRow> = frames
            .iter()
            .map(|f| PictureStatsRow::from_timeline_frame(f, frame_width, frame_height))
            .collect();

        let sequence_stats = SequenceStats::from_rows(&rows);

        Self {
            rows,
            sort_column: PictureStatsSortColumn::DisplayIdx,
            sort_direction: SortDirection::Ascending,
            filter: PictureStatsFilter::default(),
            sequence_stats,
            frame_width,
            frame_height,
        }
    }

    /// Create empty table
    pub fn empty() -> Self {
        Self {
            rows: Vec::new(),
            sort_column: PictureStatsSortColumn::DisplayIdx,
            sort_direction: SortDirection::Ascending,
            filter: PictureStatsFilter::default(),
            sequence_stats: SequenceStats::default(),
            frame_width: 0,
            frame_height: 0,
        }
    }

    /// Get filtered and sorted rows
    pub fn get_view(&self) -> Vec<&PictureStatsRow> {
        let mut view: Vec<&PictureStatsRow> = self
            .rows
            .iter()
            .filter(|r| self.filter.matches(r))
            .collect();

        // Sort
        view.sort_by(|a, b| {
            let cmp = match self.sort_column {
                PictureStatsSortColumn::DisplayIdx => a.display_idx.cmp(&b.display_idx),
                PictureStatsSortColumn::FrameType => a.frame_type.cmp(&b.frame_type),
                PictureStatsSortColumn::SizeBytes => a.size_bytes.cmp(&b.size_bytes),
                PictureStatsSortColumn::SizeBits => a.size_bits.cmp(&b.size_bits),
                PictureStatsSortColumn::Bpp => a
                    .bpp
                    .partial_cmp(&b.bpp)
                    .unwrap_or(std::cmp::Ordering::Equal),
                PictureStatsSortColumn::Pts => a.pts.cmp(&b.pts),
                PictureStatsSortColumn::Dts => a.dts.cmp(&b.dts),
                PictureStatsSortColumn::PtsDtsDelta => a.pts_dts_delta.cmp(&b.pts_dts_delta),
                PictureStatsSortColumn::QpAvg => a
                    .qp_avg
                    .partial_cmp(&b.qp_avg)
                    .unwrap_or(std::cmp::Ordering::Equal),
            };

            match self.sort_direction {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });

        view
    }

    /// Get row count after filtering
    pub fn filtered_count(&self) -> usize {
        self.rows.iter().filter(|r| self.filter.matches(r)).count()
    }

    /// Get total row count
    pub fn total_count(&self) -> usize {
        self.rows.len()
    }

    /// Set sort column (toggles direction if same column)
    pub fn set_sort(&mut self, column: PictureStatsSortColumn) {
        if self.sort_column == column {
            self.sort_direction = self.sort_direction.toggle();
        } else {
            self.sort_column = column;
            self.sort_direction = SortDirection::Ascending;
        }
    }

    /// Get row by display index
    pub fn get_row(&self, display_idx: usize) -> Option<&PictureStatsRow> {
        self.rows.iter().find(|r| r.display_idx == display_idx)
    }

    /// Update with new frames
    pub fn update(&mut self, frames: &[TimelineFrame]) {
        self.rows = frames
            .iter()
            .map(|f| PictureStatsRow::from_timeline_frame(f, self.frame_width, self.frame_height))
            .collect();
        self.sequence_stats = SequenceStats::from_rows(&self.rows);
    }

    /// Export row to CSV string
    pub fn row_to_csv(row: &PictureStatsRow) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            row.display_idx,
            row.frame_type,
            row.size_bytes,
            row.size_bits,
            row.bpp.map_or("".to_string(), |b| format!("{:.4}", b)),
            row.pts.map_or("".to_string(), |p| p.to_string()),
            row.dts.map_or("".to_string(), |d| d.to_string()),
            row.pts_dts_delta.map_or("".to_string(), |d| d.to_string()),
            row.is_keyframe,
            row.has_error,
            row.has_bookmark,
            row.is_scene_change,
            row.qp_avg.map_or("".to_string(), |q| format!("{:.2}", q)),
        )
    }

    /// CSV header
    pub fn csv_header() -> &'static str {
        "display_idx,frame_type,size_bytes,size_bits,bpp,pts,dts,pts_dts_delta,is_keyframe,has_error,has_bookmark,is_scene_change,qp_avg"
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("picture_stats_test.rs");
