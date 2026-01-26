//! Panel tab types for dock area
//!
//! Defines all available panels in the UI

use serde::{Deserialize, Serialize};

/// Tab types for the dock area
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanelTab {
    // R2: Stream Tree
    StreamTree,

    // R3: Player / Charts (Timeline integrated into Filmstrip)
    // Timeline removed - now a view mode in Filmstrip panel
    Player,
    BitrateGraph,
    QualityMetrics,

    // R4: Inspectors
    SyntaxTree,
    HexView,
    BitView,
    BlockInfo,
    SelectionInfo,
    Diagnostics,
    YuvViewer,

    // Workspaces (Monster Pack v14)
    Metrics,
    Reference,
    Compare,

    // Codec Workspaces (VQAnalyzer parity - Coding Flow visualizations)
    Av1Coding,
    AvcCoding,
    HevcCoding,
    Mpeg2Coding,
    VvcCoding,
}

impl std::fmt::Display for PanelTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PanelTab::StreamTree => write!(f, "Stream Tree"),
            // PanelTab::Timeline removed - integrated into Filmstrip
            PanelTab::Player => write!(f, "Player"),
            PanelTab::BitrateGraph => write!(f, "Bitrate"),
            PanelTab::QualityMetrics => write!(f, "Quality"),
            PanelTab::SyntaxTree => write!(f, "Syntax"),
            PanelTab::HexView => write!(f, "Hex"),
            PanelTab::BitView => write!(f, "Bits"),
            PanelTab::BlockInfo => write!(f, "Block Info"),
            PanelTab::SelectionInfo => write!(f, "Selection Info"),
            PanelTab::Diagnostics => write!(f, "⚠ Diagnostics"),
            PanelTab::YuvViewer => write!(f, "YUV"),
            PanelTab::Metrics => write!(f, "Metrics"),
            PanelTab::Reference => write!(f, "Reference"),
            PanelTab::Compare => write!(f, "Compare"),
            PanelTab::Av1Coding => write!(f, "AV1 Coding Flow"),
            PanelTab::AvcCoding => write!(f, "AVC Coding Flow"),
            PanelTab::HevcCoding => write!(f, "HEVC Coding Flow"),
            PanelTab::Mpeg2Coding => write!(f, "MPEG-2 Coding Flow"),
            PanelTab::VvcCoding => write!(f, "VVC Coding Flow"),
        }
    }
}

/// Frame navigation request from toolbar buttons
#[derive(Debug, Clone, Copy)]
pub enum FrameNavRequest {
    First,
    Prev,
    Next,
    Last,
}
