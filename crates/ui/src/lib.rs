//! ui - Pure UI panels and workspaces (no business logic)
//!
//! Monster Pack v9 Architecture:
//! - Panels emit Commands only
//! - Panels read SelectionState (immutable)
//! - No direct panel-to-panel communication
//! - No file parsing, decoding, or model mutation
//! - Workspaces: Multi-layer visualization panels

pub mod panels;
pub mod workspaces;

pub use panels::*;
pub use workspaces::*;
