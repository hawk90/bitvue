//! UI Panels for Monster Pack v9 architecture
//!
//! Pure UI components that:
//! - Emit Commands only (no direct state mutation)
//! - Read SelectionState (immutable)
//! - No business logic (parsing, decoding, etc.)
//!
//! Note: Timeline, Player, and Diagnostics have been moved to workspaces/

pub mod bit_view;
pub mod bitrate_graph;
pub mod block_info;
pub mod filmstrip;
pub mod hex_view;
pub mod quality_metrics;
pub mod selection_info;
pub mod stream_tree;
pub mod syntax_detail;
pub mod yuv_viewer;

pub use bit_view::BitViewPanel;
pub use bitrate_graph::BitrateGraphPanel;
pub use block_info::{BlockData, BlockInfoPanel};
pub use filmstrip::{FilmstripPanel, FilmstripViewMode};
pub use hex_view::HexViewPanel;
pub use quality_metrics::QualityMetricsPanel;
pub use selection_info::{BlockInfo, SelectionInfoPanel};
pub use stream_tree::StreamTreePanel;
pub use syntax_detail::SyntaxDetailPanel;
pub use yuv_viewer::YuvViewerPanel;
