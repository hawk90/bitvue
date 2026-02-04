//! # Bitvue - Video Bitstream Analyzer
//!
//! Bitvue is a professional video bitstream analyzer supporting multiple codecs and formats.
//!
//! ## Overview
//!
//! This library provides a unified interface to all Bitvue functionality:
//!
//! - **Core**: Selection state, command/event bus, worker runtime, caches
//! - **Formats**: Container parsers (IVF, MP4, MKV, TS)
//! - **Codecs**: Video codec parsers (AV1, AVC, HEVC, VP9, VVC, AV3, MPEG-2)
//! - **Decode**: Frame decoders (dav1d for AV1, etc.)
//! - **Metrics**: Quality metrics (PSNR, SSIM, VMAF)
//!
//! ## Module Organization
//!
//! - [`core`] - Core functionality and state management
//! - [`formats`] - Container format parsers
//! - [`codecs`] - Video codec parsers
//! - [`decode`] - Frame decoders
//! - [`metrics`] - Quality metrics
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use bitvue::core;
//! use bitvue::formats;
//! use bitvue::codecs;
//!
//! // Access core functionality
//! let state = core::SelectionState::new();
//!
//! // Parse a video file
//! let parser = formats::ivf::IvfParser::new();
//! ```

// Re-export core functionality
pub use bitvue_core as core;

// Re-export format parsers
pub use bitvue_formats as formats;

// Re-export unified codec interface
pub use bitvue_codecs as codecs;

// Re-export decoders
pub use bitvue_decode as decode;

// Re-export metrics
pub use bitvue_metrics as metrics;

// Re-export integration layer
pub use bitvue_codecs_parser as integration;

/// Prelude module with common imports
///
/// This module re-exports the primary modules for convenient access.
/// For specific types, import directly from the submodules.
pub mod prelude {
    pub use super::core;
    pub use super::formats;
    pub use super::codecs;
    pub use super::decode;
    pub use super::metrics;
    pub use super::integration;
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
