//! Services layer - Business logic between commands and core
//!
//! This layer contains:
//! - DecodeService: Manages frame decoding operations
//! - ThumbnailService: Generates and caches thumbnails
//! - FrameService: Aggregates frame information
//! - RateLimiter: Prevents DoS through rate limiting
//! - CodecUtils: Shared codec utility functions
//! - Utils: Shared utility macros and functions

pub mod codec_utils;
pub mod decode_service;
pub mod frame_service;
pub mod rate_limiter;
pub mod thumbnail_service;
pub mod utils;

pub use decode_service::DecodeService;
pub use rate_limiter::RateLimiter;
pub use thumbnail_service::create_svg_thumbnail;
pub use thumbnail_service::ThumbnailService;
