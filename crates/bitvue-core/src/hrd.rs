//! HRD (Hypothetical Reference Decoder) Model - Feature Parity: Buffer/HRD Plot
//!
//! Per COMPETITOR_PARITY_STATUS.md §4.1:
//! - Buffer/HRD plot (CPB fullness) - for timeline visualization
//!
//! Implements:
//! - CPB (Coded Picture Buffer) fullness model
//! - HRD conformance checking
//! - Buffer occupancy visualization data

use serde::{Deserialize, Serialize};

/// Maximum CPB state history entries to prevent unbounded memory growth
///
/// At 2 states per frame (before/after removal), this supports:
/// - 5000 states = 2500 frames = ~1.4 minutes @ 30fps
/// - ~200KB memory (each CpbState is ~40 bytes)
///
/// Older states are removed in FIFO order to maintain a rolling window.
const MAX_STATE_HISTORY: usize = 5000;

// ═══════════════════════════════════════════════════════════════════════════
// HRD Parameters
// ═══════════════════════════════════════════════════════════════════════════

/// HRD parameters extracted from bitstream
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HrdParameters {
    /// CPB size in bits
    pub cpb_size_bits: u64,
    /// Bit rate in bits per second
    pub bit_rate_bps: u64,
    /// Initial CPB removal delay in 90kHz clock ticks
    pub initial_cpb_removal_delay: u64,
    /// CPB removal delay length (bits)
    pub cpb_removal_delay_length: u8,
    /// DPB output delay length (bits)
    pub dpb_output_delay_length: u8,
    /// Time scale (ticks per second)
    pub time_scale: u32,
    /// Number of units in tick
    pub num_units_in_tick: u32,
    /// Low delay HRD flag
    pub low_delay_hrd: bool,
    /// CBR (constant bit rate) flag
    pub cbr_flag: bool,
}

impl Default for HrdParameters {
    fn default() -> Self {
        Self {
            cpb_size_bits: 1_000_000, // 1 Mbit default
            bit_rate_bps: 5_000_000,  // 5 Mbps default
            initial_cpb_removal_delay: 0,
            cpb_removal_delay_length: 23,
            dpb_output_delay_length: 23,
            time_scale: 90000,
            num_units_in_tick: 1,
            low_delay_hrd: false,
            cbr_flag: false,
        }
    }
}

impl HrdParameters {
    /// Calculate frame duration in seconds
    pub fn frame_duration_sec(&self) -> f64 {
        if self.time_scale > 0 {
            self.num_units_in_tick as f64 / self.time_scale as f64
        } else {
            1.0 / 30.0 // Default to 30fps
        }
    }

    /// Calculate CPB size in bytes
    pub fn cpb_size_bytes(&self) -> u64 {
        self.cpb_size_bits / 8
    }

    /// Calculate bit rate in kbps
    pub fn bit_rate_kbps(&self) -> f64 {
        self.bit_rate_bps as f64 / 1000.0
    }

    /// Calculate maximum buffer delay in seconds
    pub fn max_buffer_delay_sec(&self) -> f64 {
        if self.bit_rate_bps > 0 {
            self.cpb_size_bits as f64 / self.bit_rate_bps as f64
        } else {
            0.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Frame Timing Info
// ═══════════════════════════════════════════════════════════════════════════

/// Per-frame timing information for HRD model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameHrdTiming {
    /// Frame index (display order)
    pub display_idx: usize,
    /// Frame size in bits
    pub frame_size_bits: u64,
    /// CPB removal delay (90kHz ticks)
    pub cpb_removal_delay: Option<u64>,
    /// DPB output delay (90kHz ticks)
    pub dpb_output_delay: Option<u64>,
    /// Presentation timestamp (seconds)
    pub pts_sec: f64,
    /// Decode timestamp (seconds)
    pub dts_sec: Option<f64>,
}

impl FrameHrdTiming {
    pub fn new(display_idx: usize, frame_size_bits: u64, pts_sec: f64) -> Self {
        Self {
            display_idx,
            frame_size_bits,
            cpb_removal_delay: None,
            dpb_output_delay: None,
            pts_sec,
            dts_sec: None,
        }
    }

    /// Frame size in bytes
    pub fn frame_size_bytes(&self) -> u64 {
        self.frame_size_bits / 8
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CPB State
// ═══════════════════════════════════════════════════════════════════════════

/// CPB buffer state at a point in time
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CpbState {
    /// Current CPB fullness in bits
    pub fullness_bits: u64,
    /// Time in seconds
    pub time_sec: f64,
    /// Frame index associated with this state
    pub frame_idx: usize,
    /// Is this a removal point (frame was removed)?
    pub is_removal: bool,
    /// Buffer overflow flag
    pub overflow: bool,
    /// Buffer underflow flag
    pub underflow: bool,
}

impl CpbState {
    /// CPB fullness as percentage (0.0 - 1.0)
    pub fn fullness_percent(&self, cpb_size_bits: u64) -> f32 {
        if cpb_size_bits > 0 {
            (self.fullness_bits as f64 / cpb_size_bits as f64) as f32
        } else {
            0.0
        }
    }

    /// CPB fullness in bytes
    pub fn fullness_bytes(&self) -> u64 {
        self.fullness_bits / 8
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HRD Model
// ═══════════════════════════════════════════════════════════════════════════

/// HRD conformance model for simulating buffer occupancy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrdModel {
    /// HRD parameters
    pub params: HrdParameters,
    /// Current CPB fullness in bits
    cpb_fullness_bits: u64,
    /// Current time in seconds
    current_time_sec: f64,
    /// CPB state history (for visualization)
    state_history: Vec<CpbState>,
    /// Overflow count
    overflow_count: u32,
    /// Underflow count
    underflow_count: u32,
    /// Frame index
    frame_count: usize,
}

impl Default for HrdModel {
    fn default() -> Self {
        Self::new(HrdParameters::default())
    }
}

impl HrdModel {
    pub fn new(params: HrdParameters) -> Self {
        Self {
            params,
            cpb_fullness_bits: 0,
            current_time_sec: 0.0,
            state_history: Vec::new(),
            overflow_count: 0,
            underflow_count: 0,
            frame_count: 0,
        }
    }

    /// Add state to history with bounded growth (rolling window)
    ///
    /// Maintains a maximum of MAX_STATE_HISTORY entries to prevent
    /// unbounded memory growth on long videos. Removes oldest states
    /// when limit is exceeded.
    fn push_state(&mut self, state: CpbState) {
        self.state_history.push(state);

        // Enforce maximum history size to prevent memory exhaustion
        if self.state_history.len() > MAX_STATE_HISTORY {
            // Remove oldest 10% of entries to avoid frequent removals
            let remove_count = MAX_STATE_HISTORY / 10;
            self.state_history.drain(0..remove_count);
        }
    }

    /// Reset the model to initial state
    pub fn reset(&mut self) {
        self.cpb_fullness_bits = 0;
        self.current_time_sec = 0.0;
        self.state_history.clear();
        self.overflow_count = 0;
        self.underflow_count = 0;
        self.frame_count = 0;
    }

    /// Initialize buffer with initial delay
    pub fn initialize_buffer(&mut self) {
        // Fill buffer based on initial CPB removal delay
        let initial_bits = (self.params.initial_cpb_removal_delay as f64 / 90000.0)
            * self.params.bit_rate_bps as f64;
        self.cpb_fullness_bits = initial_bits.min(self.params.cpb_size_bits as f64) as u64;

        self.push_state(CpbState {
            fullness_bits: self.cpb_fullness_bits,
            time_sec: 0.0,
            frame_idx: 0,
            is_removal: false,
            overflow: false,
            underflow: false,
        });
    }

    /// Process a frame and update CPB state
    pub fn process_frame(&mut self, timing: &FrameHrdTiming) -> CpbState {
        // Step 1: Fill buffer (bits arrive at constant rate)
        let time_delta = if self.frame_count == 0 {
            self.params.frame_duration_sec()
        } else {
            timing.pts_sec - self.current_time_sec
        };

        if time_delta > 0.0 {
            let bits_arrived = (time_delta * self.params.bit_rate_bps as f64) as u64;
            self.cpb_fullness_bits = self.cpb_fullness_bits.saturating_add(bits_arrived);
        }

        // Check overflow (before removal)
        let overflow = self.cpb_fullness_bits > self.params.cpb_size_bits;
        if overflow {
            self.overflow_count += 1;
            self.cpb_fullness_bits = self.params.cpb_size_bits;
        }

        // Record state before removal
        self.push_state(CpbState {
            fullness_bits: self.cpb_fullness_bits,
            time_sec: timing.pts_sec,
            frame_idx: timing.display_idx,
            is_removal: false,
            overflow,
            underflow: false,
        });

        // Step 2: Remove frame from buffer
        let underflow = timing.frame_size_bits > self.cpb_fullness_bits;
        if underflow {
            self.underflow_count += 1;
            self.cpb_fullness_bits = 0;
        } else {
            self.cpb_fullness_bits = self
                .cpb_fullness_bits
                .saturating_sub(timing.frame_size_bits);
        }

        // Record state after removal
        let state = CpbState {
            fullness_bits: self.cpb_fullness_bits,
            time_sec: timing.pts_sec,
            frame_idx: timing.display_idx,
            is_removal: true,
            overflow: false,
            underflow,
        };
        self.push_state(state);

        self.current_time_sec = timing.pts_sec;
        self.frame_count += 1;

        state
    }

    /// Get current CPB fullness in bits
    pub fn current_fullness_bits(&self) -> u64 {
        self.cpb_fullness_bits
    }

    /// Get current CPB fullness as percentage
    pub fn current_fullness_percent(&self) -> f32 {
        if self.params.cpb_size_bits > 0 {
            (self.cpb_fullness_bits as f64 / self.params.cpb_size_bits as f64) as f32
        } else {
            0.0
        }
    }

    /// Get state history for visualization
    pub fn state_history(&self) -> &[CpbState] {
        &self.state_history
    }

    /// Get overflow count
    pub fn overflow_count(&self) -> u32 {
        self.overflow_count
    }

    /// Get underflow count
    pub fn underflow_count(&self) -> u32 {
        self.underflow_count
    }

    /// Check if model is HRD conformant
    pub fn is_conformant(&self) -> bool {
        self.overflow_count == 0 && self.underflow_count == 0
    }

    /// Get number of processed frames
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HRD Statistics
// ═══════════════════════════════════════════════════════════════════════════

/// HRD statistics summary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HrdStatistics {
    /// Total frames processed
    pub total_frames: usize,
    /// Overflow events
    pub overflow_count: u32,
    /// Underflow events
    pub underflow_count: u32,
    /// Min CPB fullness (bits)
    pub min_fullness_bits: u64,
    /// Max CPB fullness (bits)
    pub max_fullness_bits: u64,
    /// Average CPB fullness (bits)
    pub avg_fullness_bits: f64,
    /// CPB size (bits)
    pub cpb_size_bits: u64,
    /// Is HRD conformant
    pub is_conformant: bool,
}

impl HrdStatistics {
    pub fn from_model(model: &HrdModel) -> Self {
        let history = model.state_history();
        let removal_states: Vec<_> = history.iter().filter(|s| s.is_removal).collect();

        let (min, max, sum) = if removal_states.is_empty() {
            (0, 0, 0.0)
        } else {
            let min = removal_states
                .iter()
                .map(|s| s.fullness_bits)
                .min()
                .unwrap_or(0);
            let max = removal_states
                .iter()
                .map(|s| s.fullness_bits)
                .max()
                .unwrap_or(0);
            let sum: u64 = removal_states.iter().map(|s| s.fullness_bits).sum();
            let avg = if removal_states.is_empty() {
                0.0
            } else {
                sum as f64 / removal_states.len() as f64
            };
            (min, max, avg)
        };

        Self {
            total_frames: model.frame_count(),
            overflow_count: model.overflow_count(),
            underflow_count: model.underflow_count(),
            min_fullness_bits: min,
            max_fullness_bits: max,
            avg_fullness_bits: sum,
            cpb_size_bits: model.params.cpb_size_bits,
            is_conformant: model.is_conformant(),
        }
    }

    /// Min fullness as percentage
    pub fn min_fullness_percent(&self) -> f32 {
        if self.cpb_size_bits > 0 {
            (self.min_fullness_bits as f64 / self.cpb_size_bits as f64) as f32
        } else {
            0.0
        }
    }

    /// Max fullness as percentage
    pub fn max_fullness_percent(&self) -> f32 {
        if self.cpb_size_bits > 0 {
            (self.max_fullness_bits as f64 / self.cpb_size_bits as f64) as f32
        } else {
            0.0
        }
    }

    /// Avg fullness as percentage
    pub fn avg_fullness_percent(&self) -> f32 {
        if self.cpb_size_bits > 0 {
            (self.avg_fullness_bits / self.cpb_size_bits as f64) as f32
        } else {
            0.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HRD Plot Data
// ═══════════════════════════════════════════════════════════════════════════

/// Data point for HRD plot visualization
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HrdPlotPoint {
    /// Time in seconds
    pub time_sec: f64,
    /// CPB fullness percentage (0.0 - 1.0)
    pub fullness_percent: f32,
    /// Frame index
    pub frame_idx: usize,
    /// Is this a removal event
    pub is_removal: bool,
    /// Violation flag (overflow or underflow)
    pub violation: bool,
}

/// HRD plot data for timeline visualization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HrdPlotData {
    /// Plot points
    pub points: Vec<HrdPlotPoint>,
    /// CPB size in bits
    pub cpb_size_bits: u64,
    /// Max time
    pub max_time_sec: f64,
    /// Statistics
    pub statistics: Option<HrdStatistics>,
}

impl HrdPlotData {
    pub fn from_model(model: &HrdModel) -> Self {
        let cpb_size = model.params.cpb_size_bits;
        let points: Vec<_> = model
            .state_history()
            .iter()
            .map(|s| HrdPlotPoint {
                time_sec: s.time_sec,
                fullness_percent: s.fullness_percent(cpb_size),
                frame_idx: s.frame_idx,
                is_removal: s.is_removal,
                violation: s.overflow || s.underflow,
            })
            .collect();

        let max_time = points.iter().map(|p| p.time_sec).fold(0.0, f64::max);

        Self {
            points,
            cpb_size_bits: cpb_size,
            max_time_sec: max_time,
            statistics: Some(HrdStatistics::from_model(model)),
        }
    }

    /// Get points for a time range
    pub fn points_in_range(&self, start_time: f64, end_time: f64) -> Vec<&HrdPlotPoint> {
        self.points
            .iter()
            .filter(|p| p.time_sec >= start_time && p.time_sec <= end_time)
            .collect()
    }

    /// Get violation points
    pub fn violation_points(&self) -> Vec<&HrdPlotPoint> {
        self.points.iter().filter(|p| p.violation).collect()
    }

    /// Check if there are any violations
    pub fn has_violations(&self) -> bool {
        self.points.iter().any(|p| p.violation)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HRD Lane Data (for Timeline integration)
// ═══════════════════════════════════════════════════════════════════════════

/// HRD lane data for timeline lane system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrdLaneData {
    /// Frame index
    pub display_idx: usize,
    /// CPB fullness before removal (percentage)
    pub pre_removal_percent: f32,
    /// CPB fullness after removal (percentage)
    pub post_removal_percent: f32,
    /// Frame size in bits
    pub frame_size_bits: u64,
    /// Overflow flag
    pub overflow: bool,
    /// Underflow flag
    pub underflow: bool,
}

/// Convert HRD model to lane data
pub fn hrd_to_lane_data(model: &HrdModel) -> Vec<HrdLaneData> {
    let cpb_size = model.params.cpb_size_bits;
    let history = model.state_history();
    let mut result = Vec::new();

    // Group by frame_idx (each frame has pre and post removal states)
    let mut i = 0;
    while i + 1 < history.len() {
        let pre = &history[i];
        let post = &history[i + 1];

        if !pre.is_removal && post.is_removal && pre.frame_idx == post.frame_idx {
            let frame_size = pre.fullness_bits.saturating_sub(post.fullness_bits);
            result.push(HrdLaneData {
                display_idx: pre.frame_idx,
                pre_removal_percent: pre.fullness_percent(cpb_size),
                post_removal_percent: post.fullness_percent(cpb_size),
                frame_size_bits: frame_size,
                overflow: pre.overflow,
                underflow: post.underflow,
            });
            i += 2;
        } else {
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    include!("hrd_test.rs");
}
