//! Workspaces - Multi-layer visualization panels (Monster Pack v14)
//!
//! Complete workspace implementations per Monster Pack specs:
//! - WS_TIMELINE_TEMPORAL: Timeline with multi-lane overlays
//! - WS_PLAYER_SPATIAL: Player with HUD + overlays
//! - WS_DIAGNOSTICS_ERROR: Diagnostics with summary + burst detection
//! - WS_METRICS_QUALITY: Metrics series + histogram + summary
//! - WS_REFERENCE_DPB: Reference graph + DPB inspector
//! - WS_COMPARE_AB: A/B compare with delta lane + violations
//!
//! # Strategy Pattern
//!
//! The workspace_strategy module provides trait-based abstractions:
//! - ColorScheme: Codec-specific color palettes
//! - ViewRenderer: Pluggable view rendering strategies
//! - PartitionRenderer: Custom partition visualization
//! - CodecWorkspace: Common workspace interface

pub mod av1_workspace;
pub mod avc_workspace;
pub mod compare_workspace;
pub mod diagnostics_workspace;
pub mod hevc_workspace;
pub mod metrics_workspace;
pub mod mpeg2_workspace;
pub mod overlays;
pub mod player;
pub mod player_workspace;
pub mod reference_workspace;
pub mod timeline_workspace;
pub mod vvc_workspace;
pub mod workspace_strategy;

// Re-export strategy pattern traits and implementations
pub use workspace_strategy::{
    // Core traits
    ColorScheme, ViewRenderer, PartitionRenderer, CodecWorkspace,

    // Concrete types
    ViewContext, ViewRenderResult, PartitionData, StrategyBuilder, StrategySet,

    // Generic workspace
    GenericCodecWorkspace,

    // Default color schemes
    Av1ColorScheme, AvcColorScheme, HevcColorScheme, VvcColorScheme, Mpeg2ColorScheme,
};

pub use av1_workspace::Av1Workspace;
pub use avc_workspace::AvcWorkspace;
pub use compare_workspace::CompareWorkspace;
pub use diagnostics_workspace::DiagnosticsWorkspace;
pub use hevc_workspace::HevcWorkspace;
pub use metrics_workspace::MetricsWorkspace;
pub use mpeg2_workspace::Mpeg2Workspace;
pub use player_workspace::{OverlayType, PlayerWorkspace};
pub use reference_workspace::ReferenceWorkspace;
pub use timeline_workspace::TimelineWorkspace;
pub use vvc_workspace::VvcWorkspace;
