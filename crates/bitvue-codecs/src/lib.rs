//! codecs - Codec-specific parsers (AV1, H.264, HEVC)
//!
//! Monster Pack v3 Architecture:
//! - Pure data output (no egui types)
//! - Produces UnitModel + SyntaxModel + bit ranges
//! - Implements CodecParser, CodecIndexBuilder, CodecStatsBuilder traits

// Re-export bitvue-av1 for now (will integrate directly in Phase 0)
pub use bitvue_av1::*;

// Placeholder for BOSS_03
// Full codec parser traits will be implemented in BOSS_03
