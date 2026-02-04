//! Bitvue Codecs - Unified codec parser interface
//!
//! This crate provides a unified interface to all video codec parsers.

// AV1 (AOMedia Video 1)
pub use bitvue_av1_codec as av1;

// AVC/H.264
pub use bitvue_avc as avc;

// HEVC/H.265
pub use bitvue_hevc as hevc;

// VP9
pub use bitvue_vp9 as vp9;

// VVC/H.266
pub use bitvue_vvc as vvc;

// AV3 (next-generation)
pub use bitvue_av3_codec as av3;

// MPEG-2
pub use bitvue_mpeg2_codec as mpeg2;
