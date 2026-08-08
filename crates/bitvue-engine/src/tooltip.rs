//! Tooltip System - T8-1
//!
//! Per TOOLTIP_SPEC.md:
//! - 150ms hover delay (debounced)
//! - Never cover cursor target
//! - Stable while cursor remains in target
//! - Multi-line content with monospace blocks
//! - Copy actions for offsets/ranges
//! - Explicit units (bytes, bits, ms, frames)
//! - "N/A" for unavailable fields
//!
//! Tooltip types:
//! - Timeline: Frame bars, markers, selection lines
//! - Metrics: Plot points
//! - Tree: Track/frame/unit rows
//! - Syntax: Field nodes
//! - Hex/Bit: Byte/bit cells
//! - Player: Pixel/block hover
//! - Diagnostics: Error rows

use serde::{Deserialize, Serialize};

/// Tooltip configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipConfig {
    /// Hover delay before showing (ms)
    pub hover_delay_ms: u32,

    /// Offset from cursor (pixels)
    pub cursor_offset: (i32, i32),

    /// Maximum width (pixels)
    pub max_width: u32,

    /// Enable copy buttons
    pub enable_copy_actions: bool,
}

impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            hover_delay_ms: 150, // Per spec
            cursor_offset: (10, 10),
            max_width: 400,
            enable_copy_actions: true,
        }
    }
}

/// Tooltip content for different contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TooltipContent {
    /// Timeline frame bar tooltip
    Timeline(TimelineTooltip),

    /// Metrics plot point tooltip
    Metrics(MetricsTooltip),

    /// Tree row tooltip
    Tree(TreeTooltip),

    /// Syntax node tooltip
    Syntax(SyntaxTooltip),

    /// Hex/bit cell tooltip
    HexBit(HexBitTooltip),

    /// Player surface tooltip
    Player(PlayerTooltip),

    /// Diagnostics row tooltip
    Diagnostics(DiagnosticsTooltip),

    /// Custom tooltip with raw text
    Custom(String),
}

/// Timeline tooltip (frame bars, markers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineTooltip {
    /// Frame index
    pub frame_idx: usize,

    /// Frame type (Key/Inter/I/P/B)
    pub frame_type: String,

    /// PTS (ms or ticks)
    pub pts: Option<u64>,

    /// DTS (ms or ticks)
    pub dts: Option<u64>,

    /// Derived time (seconds)
    pub time_seconds: Option<f64>,

    /// Frame size (bytes)
    pub size_bytes: Option<usize>,

    /// Frame size (bits)
    pub size_bits: Option<usize>,

    /// Markers (Keyframe, Error, Bookmark)
    pub markers: Vec<String>,

    /// Decode status
    pub decoded: bool,

    /// Last decode error (short message)
    pub decode_error: Option<String>,

    /// Evidence chain: syntax path (e.g., `"OBU_FRAME.tile[0].superblock[5]"`)
    pub syntax_path: Option<String>,

    /// Evidence chain: bit offset in stream
    pub bit_offset: Option<u64>,

    /// Evidence chain: byte offset in stream
    pub byte_offset: Option<u64>,

    /// Copy actions
    pub copy_actions: Vec<CopyAction>,
}

impl TimelineTooltip {
    /// Format as multi-line text
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        // Frame info
        lines.push(format!("Frame: {} ({})", self.frame_idx, self.frame_type));

        // Time info
        if let Some(pts) = self.pts {
            lines.push(format!("PTS: {} ms", pts));
        } else {
            lines.push("PTS: N/A".to_string());
        }

        if let Some(dts) = self.dts {
            lines.push(format!("DTS: {} ms", dts));
        }

        if let Some(time_s) = self.time_seconds {
            lines.push(format!("Time: {:.3} s", time_s));
        }

        // Size info
        if let Some(bytes) = self.size_bytes {
            lines.push(format!(
                "Size: {} bytes ({} bits)",
                bytes,
                self.size_bits.unwrap_or(bytes * 8)
            ));
        } else {
            lines.push("Size: N/A".to_string());
        }

        // Markers
        if !self.markers.is_empty() {
            lines.push(format!("Markers: {}", self.markers.join(", ")));
        }

        // Decode status
        lines.push(format!(
            "Decoded: {}",
            if self.decoded { "yes" } else { "no" }
        ));
        if let Some(ref error) = self.decode_error {
            lines.push(format!("Error: {}", error));
        }

        // Evidence chain (per evidence_hover.001)
        if self.syntax_path.is_some() || self.bit_offset.is_some() {
            lines.push(String::new()); // Separator
            lines.push("Evidence Chain:".to_string());

            if let Some(ref path) = self.syntax_path {
                lines.push(format!("  Syntax: {}", path));
            }

            if let Some(byte_off) = self.byte_offset {
                lines.push(format!("  Byte offset: {}", byte_off));
            }

            if let Some(bit_off) = self.bit_offset {
                lines.push(format!("  Bit offset: {}", bit_off));
            }
        }

        lines.join("\n")
    }
}

/// Metrics plot tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTooltip {
    /// Frame index
    pub frame_idx: usize,

    /// Time (seconds)
    pub time_seconds: Option<f64>,

    /// Series name (PSNR_Y, SSIM_Y, etc.)
    pub series_name: String,

    /// Metric value
    pub value: f32,

    /// Unit
    pub unit: String,

    /// Delta vs previous frame
    pub delta: Option<f32>,

    /// Copy actions
    pub copy_actions: Vec<CopyAction>,
}

impl MetricsTooltip {
    /// Format as multi-line text
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Frame: {}", self.frame_idx));

        if let Some(time_s) = self.time_seconds {
            lines.push(format!("Time: {:.3} s", time_s));
        }

        lines.push(format!(
            "{}: {:.2} {}",
            self.series_name, self.value, self.unit
        ));

        if let Some(delta) = self.delta {
            lines.push(format!("Δ: {:+.2} {}", delta, self.unit));
        }

        lines.join("\n")
    }
}

/// Tree row tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeTooltip {
    /// Path breadcrumb (Container/Track/Frame/Unit)
    pub path: Vec<String>,

    /// Offset (hex)
    pub offset_hex: String,

    /// Size (bytes)
    pub size_bytes: usize,

    /// Unit type
    pub unit_type: String,

    /// Flags (key, error, warn)
    pub flags: Vec<String>,

    /// Parse status
    pub parsed: bool,

    /// Diagnostic summary (if errors)
    pub diagnostic_summary: Option<String>,

    /// Copy actions
    pub copy_actions: Vec<CopyAction>,
}

impl TreeTooltip {
    /// Format as multi-line text
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        // Path
        lines.push(format!("Path: {}", self.path.join(" > ")));

        // Offset and size
        lines.push(format!(
            "Offset: {} ({} bytes)",
            self.offset_hex, self.size_bytes
        ));

        // Unit type
        lines.push(format!("Type: {}", self.unit_type));

        // Flags
        if !self.flags.is_empty() {
            lines.push(format!("Flags: {}", self.flags.join(", ")));
        }

        // Parse status
        lines.push(format!(
            "Parsed: {}",
            if self.parsed { "yes" } else { "no" }
        ));

        // Diagnostics
        if let Some(ref summary) = self.diagnostic_summary {
            lines.push(format!("Diagnostics: {}", summary));
        }

        lines.join("\n")
    }
}

/// Syntax node tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxTooltip {
    /// Field name
    pub field_name: String,

    /// Field type
    pub field_type: String,

    /// Decoded value (formatted)
    pub decoded_value: String,

    /// Bit range (start..end)
    pub bit_range: (u64, u64),

    /// Raw bits preview (first N bits)
    pub raw_bits: String,

    /// Condition (if any)
    pub condition: Option<String>,

    /// Copy actions
    pub copy_actions: Vec<CopyAction>,
}

impl SyntaxTooltip {
    /// Format as multi-line text
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("{}: {}", self.field_name, self.field_type));
        lines.push(format!("Value: {}", self.decoded_value));
        lines.push(format!(
            "Bits: {}..{} ({} bits)",
            self.bit_range.0,
            self.bit_range.1,
            self.bit_range.1 - self.bit_range.0
        ));
        lines.push(format!("Raw: {}", self.raw_bits));

        if let Some(ref cond) = self.condition {
            lines.push(format!("Condition: {}", cond));
        }

        lines.join("\n")
    }
}

/// Hex/bit cell tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexBitTooltip {
    /// Byte offset (hex)
    pub offset_hex: String,

    /// Byte value (hex)
    pub byte_hex: String,

    /// Byte value (decimal)
    pub byte_decimal: u8,

    /// ASCII character (if printable)
    pub ascii_char: Option<char>,

    /// Bit index within byte
    pub bit_in_byte: Option<u8>,

    /// Global bit offset
    pub global_bit_offset: Option<u64>,

    /// Mapped syntax node (best-effort)
    pub mapped_field: Option<String>,

    /// Copy actions
    pub copy_actions: Vec<CopyAction>,
}

impl HexBitTooltip {
    /// Format as multi-line text
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Offset: {}", self.offset_hex));
        lines.push(format!("Byte: {} ({})", self.byte_hex, self.byte_decimal));

        if let Some(ch) = self.ascii_char {
            if ch.is_ascii_graphic() {
                lines.push(format!("ASCII: '{}'", ch));
            }
        }

        if let Some(bit_idx) = self.bit_in_byte {
            lines.push(format!("Bit: {} in byte", bit_idx));
        }

        if let Some(global_bit) = self.global_bit_offset {
            lines.push(format!("Global bit: {}", global_bit));
        }

        if let Some(ref field) = self.mapped_field {
            lines.push(format!("Field: {}", field));
        }

        lines.join("\n")
    }
}

/// Player surface tooltip
///
/// Per WS_PLAYER_SPATIAL:
/// - Pixel hover: x,y + luma/chroma (if available)
/// - Block hover: block id, qp, mv, partition type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerTooltip {
    /// Frame index
    pub frame_idx: usize,

    /// Pixel coordinates
    pub pixel_xy: (u32, u32),

    /// Luma value (if available)
    pub luma: Option<u8>,

    /// Chroma values (if available)
    pub chroma: Option<(u8, u8)>,

    /// Block ID (if available)
    pub block_id: Option<String>,

    /// Quantization parameter (if available)
    pub qp: Option<f32>,

    /// Motion vector (dx, dy) in pixels (if available)
    pub mv: Option<(f32, f32)>,

    /// Partition info (if available)
    pub partition_info: Option<String>,

    /// Active overlays
    pub active_overlays: Vec<String>,

    /// Evidence chain: syntax path (e.g., `"OBU_FRAME.tile[0].sb[5]"`)
    pub syntax_path: Option<String>,

    /// Evidence chain: bit offset in stream
    pub bit_offset: Option<u64>,

    /// Evidence chain: byte offset in stream
    pub byte_offset: Option<u64>,

    /// Copy actions
    pub copy_actions: Vec<CopyAction>,
}

impl PlayerTooltip {
    /// Format as multi-line text
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Frame: {}", self.frame_idx));
        lines.push(format!("Pixel: ({}, {})", self.pixel_xy.0, self.pixel_xy.1));

        if let Some(y) = self.luma {
            lines.push(format!("Y: {}", y));
        }

        if let Some((u, v)) = self.chroma {
            lines.push(format!("UV: ({}, {})", u, v));
        }

        if let Some(ref block) = self.block_id {
            lines.push(format!("Block: {}", block));
        }

        if let Some(qp) = self.qp {
            lines.push(format!("QP: {:.1}", qp));
        }

        if let Some((dx, dy)) = self.mv {
            let magnitude = (dx * dx + dy * dy).sqrt();
            lines.push(format!("MV: ({:.1}, {:.1}) [{:.1}px]", dx, dy, magnitude));
        }

        if let Some(ref part) = self.partition_info {
            lines.push(format!("Partition: {}", part));
        }

        if !self.active_overlays.is_empty() {
            lines.push(format!("Overlays: {}", self.active_overlays.join(", ")));
        }

        // Evidence chain (per evidence_hover.001)
        if self.syntax_path.is_some() || self.bit_offset.is_some() {
            lines.push(String::new()); // Separator
            lines.push("Evidence Chain:".to_string());

            if let Some(ref path) = self.syntax_path {
                lines.push(format!("  Syntax: {}", path));
            }

            if let Some(byte_off) = self.byte_offset {
                lines.push(format!("  Byte offset: {}", byte_off));
            }

            if let Some(bit_off) = self.bit_offset {
                lines.push(format!("  Bit offset: {}", bit_off));
            }
        }

        lines.join("\n")
    }
}

/// Diagnostics row tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsTooltip {
    /// Severity (info, warn, error, critical)
    pub severity: String,

    /// Category
    pub category: String,

    /// Offset (hex)
    pub offset_hex: Option<String>,

    /// Frame/unit references
    pub frame_unit_refs: Vec<String>,

    /// Full message (wrapped)
    pub full_message: String,

    /// Root cause chain
    pub root_cause_chain: Vec<String>,

    /// Copy actions
    pub copy_actions: Vec<CopyAction>,
}

impl DiagnosticsTooltip {
    /// Format as multi-line text
    pub fn format(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Severity: {}", self.severity));
        lines.push(format!("Category: {}", self.category));

        if let Some(ref offset) = self.offset_hex {
            lines.push(format!("Offset: {}", offset));
        }

        if !self.frame_unit_refs.is_empty() {
            lines.push(format!("Refs: {}", self.frame_unit_refs.join(", ")));
        }

        lines.push(format!("\n{}", self.full_message));

        if !self.root_cause_chain.is_empty() {
            lines.push("\nRoot cause:".to_string());
            for cause in &self.root_cause_chain {
                lines.push(format!("  → {}", cause));
            }
        }

        lines.join("\n")
    }
}

/// Copy action for tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyAction {
    /// Action label
    pub label: String,

    /// Content to copy
    pub content: String,
}

impl CopyAction {
    /// Create a new copy action
    pub fn new(label: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            content: content.into(),
        }
    }
}

/// Tooltip manager
#[derive(Debug, Clone)]
pub struct TooltipManager {
    /// Configuration
    pub config: TooltipConfig,

    /// Current tooltip (if any)
    pub current_tooltip: Option<TooltipState>,
}

impl TooltipManager {
    /// Create new tooltip manager
    pub fn new(config: TooltipConfig) -> Self {
        Self {
            config,
            current_tooltip: None,
        }
    }

    /// Show tooltip
    pub fn show(&mut self, content: TooltipContent, position: (i32, i32)) {
        self.current_tooltip = Some(TooltipState {
            content,
            position,
            visible: true,
        });
    }

    /// Hide tooltip
    pub fn hide(&mut self) {
        self.current_tooltip = None;
    }

    /// Update tooltip position
    pub fn update_position(&mut self, position: (i32, i32)) {
        if let Some(ref mut tooltip) = self.current_tooltip {
            tooltip.position = position;
        }
    }

    /// Get current tooltip
    pub fn current(&self) -> Option<&TooltipState> {
        self.current_tooltip.as_ref()
    }
}

impl Default for TooltipManager {
    fn default() -> Self {
        Self::new(TooltipConfig::default())
    }
}

/// Tooltip state
#[derive(Debug, Clone)]
pub struct TooltipState {
    /// Tooltip content
    pub content: TooltipContent,

    /// Position (screen coordinates)
    pub position: (i32, i32),

    /// Visible flag
    pub visible: bool,
}

impl TooltipState {
    /// Get formatted text
    pub fn format_text(&self) -> String {
        match &self.content {
            TooltipContent::Timeline(t) => t.format(),
            TooltipContent::Metrics(t) => t.format(),
            TooltipContent::Tree(t) => t.format(),
            TooltipContent::Syntax(t) => t.format(),
            TooltipContent::HexBit(t) => t.format(),
            TooltipContent::Player(t) => t.format(),
            TooltipContent::Diagnostics(t) => t.format(),
            TooltipContent::Custom(text) => text.clone(),
        }
    }

    /// Get copy actions
    pub fn copy_actions(&self) -> Vec<&CopyAction> {
        match &self.content {
            TooltipContent::Timeline(t) => t.copy_actions.iter().collect(),
            TooltipContent::Metrics(t) => t.copy_actions.iter().collect(),
            TooltipContent::Tree(t) => t.copy_actions.iter().collect(),
            TooltipContent::Syntax(t) => t.copy_actions.iter().collect(),
            TooltipContent::HexBit(t) => t.copy_actions.iter().collect(),
            TooltipContent::Player(t) => t.copy_actions.iter().collect(),
            TooltipContent::Diagnostics(t) => t.copy_actions.iter().collect(),
            TooltipContent::Custom(_) => Vec::new(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("tooltip_test.rs");
