//! Services layer - Business logic between commands and core
//!
//! This layer contains:
//! - DecodeService: Manages frame decoding operations
//! - ThumbnailService: Generates and caches thumbnails
//! - FrameService: Aggregates frame information
//! - CodecUtils: Shared codec utility functions

pub mod codec_utils;
pub mod decode_service;
pub mod frame_service;
pub mod thumbnail_service;

pub use decode_service::DecodeService;
pub use thumbnail_service::ThumbnailService;
pub use thumbnail_service::create_svg_thumbnail;
