//! bitvue-container: Container format parsers for bitvue
//!
//! This crate provides parsers for container formats (MP4, MKV, TS) with zero external
//! dependencies beyond the Rust standard library.
//!
//! # Supported Formats
//!
//! - **MP4** (ISO Base Media File Format) - For extracting video samples (AV1, H.264, H.265)
//! - **MKV** (Matroska) - For extracting video samples (AV1, H.264, H.265)
//! - **TS** (MPEG-2 Transport Stream) - For extracting video samples (AV1, H.264, H.265)
//!
//! # Supported Codecs
//!
//! ## Phase 12A (Current)
//! - **AV1**: Fully supported in MP4, MKV, TS
//! - **H.264/AVC**: Sample extraction from MP4 (avc1/avc3) and MKV (V_MPEG4/ISO/AVC)
//! - **H.265/HEVC**: Sample extraction from MP4 (hev1/hvc1) and MKV (V_MPEGH/ISO/HEVC)
//!
//! ## Phase 12B/C (Planned)
//! - **H.264/AVC**: Full TS parsing with PMT stream type detection
//! - **H.265/HEVC**: Full TS parsing with PMT stream type detection
//!
//! # Examples
//!
//! ## Extract AV1 samples
//! ```no_run
//! use bitvue_container::mp4;
//! use std::fs;
//!
//! let data = fs::read("video.mp4").unwrap();
//! let samples = mp4::extract_av1_samples(&data).unwrap();
//! println!("Extracted {} AV1 samples", samples.len());
//! ```
//!
//! ## Extract H.264 samples
//! ```no_run
//! use bitvue_container::mp4;
//! use std::fs;
//!
//! let data = fs::read("video_h264.mp4").unwrap();
//! let samples = mp4::extract_avc_samples(&data).unwrap();
//! println!("Extracted {} H.264 samples", samples.len());
//! ```
//!
//! ## Extract H.265 samples
//! ```no_run
//! use bitvue_container::mkv;
//! use std::fs;
//!
//! let data = fs::read("video_hevc.mkv").unwrap();
//! let samples = mkv::extract_hevc_samples(&data).unwrap();
//! println!("Extracted {} H.265 samples", samples.len());
//! ```

pub mod container;
pub mod ivf_writer;
pub mod mkv;
pub mod mp4;
pub mod ts;

// Re-export main types and functions
pub use container::{detect_container_format, is_supported_format, ContainerFormat};
pub use ivf_writer::IvfWriter;
pub use mkv::MkvInfo;
pub use mp4::{BoxHeader, Mp4Info};
pub use ts::TsInfo;
