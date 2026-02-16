//! bitvue-decode: Multi-codec frame decoding
//!
//! This crate provides frame decoding functionality for multiple video codecs.
//! Currently supports:
//! - AV1 (via dav1d)
//! - H.264/AVC (via FFmpeg)
//! - H.265/HEVC (via FFmpeg)
//! - H.266/VVC (via vvdec)
//! - VP9 (via FFmpeg)

// Allow clippy warnings common in decoder code
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::uninit_vec)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(unfulfilled_lint_expectations)]

pub mod decoder;
#[cfg(feature = "ffmpeg")]
pub mod ffmpeg;
pub mod plane_utils;
pub mod strategy;
pub mod traits;
#[cfg(feature = "vvdec")]
pub mod vvdec;
pub mod yuv;
pub mod yuv_loader;

pub use decoder::{Av1Decoder, DecodedFrame, FrameType};
#[cfg(feature = "ffmpeg")]
pub use ffmpeg::{H264Decoder, HevcDecoder, Vp9Decoder};
pub use traits::{CodecType, Decoder, DecoderCapabilities, DecoderFactory};
#[cfg(feature = "vvdec")]
pub use vvdec::VvcDecoder;
pub use yuv::yuv_to_rgb;
pub use yuv_loader::{BitDepth, ChromaSubsampling, YuvFileParams, YuvLoader, YuvLoaderError};
