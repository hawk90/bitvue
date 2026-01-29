//! bitvue-core: Core types and traits for bitstream analysis
//!
//! Monster Pack v9 Architecture:
//! - SelectionState: Single source of truth
//! - Command/Event bus: No direct panel-to-panel calls
//! - Worker runtime: Last-wins cancellation
//! - WorkspaceState: Single/Dual stream management

#![allow(ambiguous_glob_reexports)]

pub mod bitreader;
pub mod codec_error;
pub mod error;
pub mod frame;
pub mod limits;
pub mod qp_extraction;
pub mod types;

// Monster Pack v9: Core state management
pub mod command;
pub mod event;
pub mod selection;
pub mod worker;
pub mod workspace;

// Monster Pack v9: File I/O
pub mod byte_cache;

// Monster Pack v9: State model
pub mod stream_state;

// Monster Pack v9: Core coordinator
pub mod core;

// Monster Pack v14: Feature Parity - Export
pub mod export; // CSV/JSON export for timeline, metrics, diagnostics

// Monster Pack v14: Phase 0 - Foundations
pub mod cache_provenance; // T0-2: Cache Provenance Tracking
pub mod coordinate_transform; // T0-2: Coordinate System Contract
pub mod evidence; // T0-2: Evidence Chain (bit_offset layer)
pub mod frame_identity; // T0-1: Frame Identity Contract
pub mod future_plugin; // T0-2: Future Plugin System
pub mod player_evidence; // T0-2: PlayerOverlay Evidence Chain Integration

// Monster Pack v14: Extended Evidence Chain (Perfect Visualization)
pub mod semantic_evidence; // Layer 3: Codec-specific semantic evidence
pub mod spatial_hierarchy; // Layer 6: Spatial hierarchy (Frame→Tile→CTU→Block)
pub mod temporal_state; // Layer 5: DPB state and temporal evolution

// Monster Pack v14: Phase 1 - Indexing & I/O
pub mod index_dev_hud; // T1-1: Index Development HUD
pub mod index_dev_hud_window;
pub mod index_extractor; // T1-1: Index Extractor API + Adapters
pub mod index_extractor_evidence; // T1-1: Index Extractor Evidence Chain Integration
pub mod index_session; // T1-1: Index Session Management
pub mod index_session_evidence; // T1-1: Index Session Evidence Chain Integration
pub mod index_session_window; // T1-1: Index Session Out-of-Core Windowing
pub mod indexing; // T1-1: Two-Phase Index Builder // T1-1: Index DevHUD Out-of-Core Windowing

// Monster Pack v14: Phase 2 - Player Core
pub mod player; // T2-1: Player Frame Pipeline

// Monster Pack v14: Phase 3 - Visual Overlays
pub mod block_metrics; // Feature Parity: Per-block metric map (PSNR/SSIM)
pub mod diff_heatmap;
pub mod mv_overlay; // T3-2: MV Vector Overlay
pub mod partition_grid; // T3-3: Partition / Block Grid Overlay
pub mod qp_heatmap; // T3-1: QP Heatmap Overlay // T3-4: Diff Heatmap Overlay (Compare)

// Monster Pack v14: Phase 4 - Timeline
pub mod diagnostics_bands;
pub mod hrd; // Feature Parity: HRD/Buffer Plot (CPB fullness)
pub mod timeline; // T4-1: Timeline Base Track
pub mod timeline_cache; // T4-1: Timeline Cache Provenance
pub mod timeline_evidence; // T4-1: Timeline Evidence Chain Integration
pub mod timeline_lane_clustering; // T4-2: Marker clustering for LOD
pub mod timeline_lane_population; // T4-2: Lane population helpers
pub mod timeline_lane_types; // T4-2: Lane types and statistics
pub mod timeline_lanes; // T4-2: Multi-Lane Timeline Overlays
pub mod timeline_window; // T0-2: Timeline Out-of-Core Windowing // T4-3: Timeline Diagnostics Bands

// Monster Pack v14: Phase 5 - Graph & Diagnostics
pub mod metadata; // Feature Parity: Metadata Inspector (HDR/SEI)
pub mod metrics_distribution;
pub mod picture_stats; // Feature Parity: Picture Stats Table (aggregated frame statistics)
pub mod reference_graph; // T5-1: Reference Graph View
pub mod reference_graph_evidence; // T5-1: Reference Graph Evidence Chain Integration // T5-2: Metrics Distribution Panel

// Monster Pack v14: Phase 6 - Compare & Regression
pub mod alignment; // T6-1: Compare Alignment Engine
pub mod compare; // T6-2: A/B Compare View
pub mod compare_cache;
pub mod compare_evidence; // T6-2: Compare Evidence Chain Integration // T0-2: Compare Cache Provenance

// Monster Pack v14: Phase 7 - Insight & MCP
pub mod insight_feed; // T7-1: Insight Feed Generator
pub mod mcp; // T7-2: MCP Integration Layer

// Monster Pack v14: Phase 8 - UX Polish
pub mod cache_debug_overlay; // T8-1: Cache Debug Overlay
pub mod diagnostics;
pub mod disable_reason; // T8-1: Disable Reason Matrix
pub mod discoverability; // T8-1: Discoverability System
pub mod filmstrip; // VQAnalyzer Parity: Filmstrip thumbnail strip
pub mod occlusion_budget; // T8-1: Occlusion Budget System
pub mod tooltip; // T8-1: Tooltip System // T8-2: Error & Degrade UI

// Monster Pack v14: Phase 9 - Performance & Validation
pub mod cache_validation; // T9-2: Cache Validation & HUD
pub mod lockcheck;
pub mod performance; // T9-1: Performance Instrumentation // T9-3: Product Lock Check (v14)

// Monster Pack v14: Phase 10 - Parity Harness
pub mod parity_harness; // T10-1: Competitor Parity Harness (schema validation, probes, gates)

pub use self::bitreader::*;
pub use self::codec_error::*;
pub use self::core::*;
pub use self::frame::*;
pub use alignment::*;
pub use block_metrics::*;
pub use byte_cache::*;
pub use cache_debug_overlay::*;
pub use cache_provenance::*;
pub use cache_validation::*;
pub use command::*;
pub use compare::*;
pub use compare_cache::*;
pub use compare_evidence::*;
pub use coordinate_transform::*;
pub use diagnostics::*;
pub use diagnostics_bands::*;
pub use diff_heatmap::*;
pub use disable_reason::*;
pub use discoverability::*;
pub use error::*;
pub use event::*;
pub use evidence::*;
pub use export::*;
pub use filmstrip::*;
pub use frame_identity::*;
pub use future_plugin::*;
pub use hrd::*;
pub use index_dev_hud::*;
pub use index_dev_hud_window::*;
pub use index_extractor::*;
pub use index_extractor_evidence::*;
pub use index_session::*;
pub use index_session_evidence::*;
pub use index_session_window::*;
pub use indexing::*;
pub use insight_feed::*;
pub use lockcheck::*;
pub use mcp::*;
pub use metadata::*;
pub use metrics_distribution::*;
pub use mv_overlay::*;
pub use occlusion_budget::*;
pub use parity_harness::*;
pub use partition_grid::*;
pub use performance::*;
pub use picture_stats::*;
pub use player::*;
pub use player_evidence::*;
pub use qp_heatmap::*;
pub use reference_graph::*;
pub use reference_graph_evidence::*;
pub use selection::*;
pub use semantic_evidence::*;
pub use spatial_hierarchy::*;
pub use stream_state::*;
pub use temporal_state::*;
pub use timeline::*;
pub use timeline_cache::*;
pub use timeline_evidence::*;
pub use timeline_lanes::*;
pub use timeline_window::*;
pub use tooltip::*;
pub use types::*;
// Export commonly used types at crate root for convenience
pub use types::FrameType;
pub use worker::*;
pub use workspace::*;
