//! AVS3 / IEEE 1857.10 bitstream parser for bitvue.
//!
//! Provides bitstream parsing for AVS3 (also known as GB/T 33475.3 and
//! IEEE 1857.10), the third-generation Audio Video Standard from China.
//!
//! # Supported capabilities
//!
//! - Start-code (NAL-unit) scanning (0x000001xx prefix)
//! - Sequence Header parsing (profile, level, dimensions, filter flags)
//! - I-picture and P/B-picture header parsing (QP, ESAO/CCSAO flags)
//! - Frame extraction with accurate offsets and sizes
//! - ESAO map heuristic (picture-level proxy; real AEC decoding pending)
//! - CCSAO map heuristic (picture-level proxy)
//!
//! # Note on AEC decoding
//!
//! Full per-CTU syntax element extraction would require decoding AVS3's
//! AEC (Adaptive Entropy Coding) — a CABAC variant. This is planned but
//! not yet implemented; current overlay data uses picture-level proxies.
//!
//! # Example
//!
//! ```no_run
//! use bitvue_avs3::extract_avs3_frames;
//!
//! let data: &[u8] = &[/* AVS3 bitstream */];
//! let result = extract_avs3_frames(data, 0).unwrap();
//!
//! println!("Found {} frames", result.frames.len());
//! if let Some(seq) = &result.sequence_header {
//!     println!("{}×{} {:?}", seq.width, seq.height, seq.profile);
//! }
//! ```

pub mod bitreader;
pub mod error;
pub mod frames;
pub mod nal;
pub mod overlay_extraction;
pub mod picture_header;
pub mod sequence_header;
pub mod syntax;

// ── Top-level re-exports ──────────────────────────────────────────────────────

pub use bitreader::BitReader;
pub use error::{Avs3Error, Result};
pub use frames::{extract_avs3_frames, Avs3Frame, ExtractResult};
pub use nal::{scan_nal_units, NalUnit, Sci};
pub use overlay_extraction::{extract_ccsao_map, extract_esao_map, extract_qp_grid};
pub use picture_header::{
    parse_i_picture_header, parse_pb_picture_header, PictureHeader, PictureType,
};
pub use sequence_header::{
    frame_rate_from_code, parse_sequence_header, Avs3Profile, ChromaFormat, SequenceHeader,
};
pub use syntax::{picture_header_syntax, sequence_header_syntax};

#[cfg(test)]
mod tests;
