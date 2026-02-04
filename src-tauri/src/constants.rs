//! Application-wide constants for Bitvue
//!
//! This module centralizes all magic numbers and provides named constants
//! for better maintainability and to avoid hardcoded values throughout the codebase.

/// Video-related constants
pub mod video {
    /// Default video dimensions
    pub const DEFAULT_WIDTH: u32 = 1920;
    pub const DEFAULT_HEIGHT: u32 = 1080;

    /// Default frame rate
    pub const DEFAULT_FRAME_RATE: u32 = 30;

    /// Maximum supported dimension
    pub const MAX_DIMENSION: u32 = 16384; // 16K

    /// Common video resolution names
    pub const RESOLUTION_480P: (u32, u32) = (854, 480);
    pub const RESOLUTION_720P: (u32, u32) = (1280, 720);
    pub const RESOLUTION_1080P: (u32, u32) = (1920, 1080);
    pub const RESOLUTION_4K: (u32, u32) = (3840, 2160);

    /// Bytes per pixel for different formats
    pub const BYTES_PER_PIXEL_RGB: usize = 3;
    pub const BYTES_PER_PIXEL_RGBA: usize = 4;
    pub const BYTES_PER_PIXEL_YUV420: usize = 1; // Average (4:2:0 subsampling)
}

/// File size limits
pub mod limits {
    /// Maximum file size: 2GB
    pub const MAX_FILE_SIZE_BYTES: u64 = 2_147_483_648;

    /// Maximum file size in megabytes
    pub const MAX_FILE_SIZE_MB: u64 = MAX_FILE_SIZE_BYTES / (1024 * 1024);

    /// Bytes per megabyte
    pub const BYTES_PER_MB: u64 = 1024 * 1024;

    /// Maximum hex data to return
    pub const MAX_HEX_BYTES: usize = 1_048_576; // 1MB

    /// Maximum image size for PNG encoding
    pub const MAX_IMAGE_SIZE_BYTES: usize = 100 * 1024 * 1024; // 100MB
}

/// Batch operation limits
pub mod batch {
    /// Maximum frames per batch request
    pub const MAX_BATCH_SIZE: usize = 1000;

    /// Maximum batch memory in megabytes
    pub const MAX_BATCH_MEMORY_MB: usize = 512;

    /// Maximum thumbnails per request
    pub const MAX_THUMBNAIL_REQUEST: usize = 500;

    /// Maximum quality samples
    pub const MAX_SAMPLES: usize = 100_000;
}

/// Grid and analysis limits
pub mod analysis {
    /// Maximum grid dimension
    pub const MAX_GRID_SIZE: u32 = 256;

    /// Minimum grid dimension
    pub const MIN_GRID_SIZE: u32 = 1;

    /// Block size for macroblock analysis
    pub const BLOCK_SIZE: u32 = 16;

    /// CTU (Coding Tree Unit) size for AV1/HEVC
    pub const CTU_SIZE: u32 = 64;
}

/// Cache limits
pub mod cache {
    /// Maximum decoded frame cache size in bytes
    pub const MAX_FRAME_CACHE_BYTES: usize = 512 * 1024 * 1024; // 512MB

    /// Maximum thumbnail cache entries
    pub const MAX_THUMBNAIL_CACHE_ENTRIES: usize = 200;

    /// Number of frames to prefetch during sequential access
    pub const PREFETCH_COUNT: usize = 3;
}

/// Format detection
pub mod format {
    /// File signatures (magic bytes)
    pub const IVF_SIGNATURE: &[u8; 4] = b"DKIF";
    pub const MP4_FTYP_SIGNATURE: &[u8; 4] = b"ftyp";
    pub const MKV_EBML_SIGNATURE: &[u8; 4] = b"\x1a\x45\xdf\xa3";

    /// Supported file extensions
    pub const EXT_IVF: &str = "ivf";
    pub const EXT_WEBM: &str = "webm";
    pub const EXT_MKV: &str = "mkv";
    pub const EXT_MP4: &str = "mp4";
    pub const EXT_MOV: &str = "mov";
    pub const EXT_H264: &str = "h264";
    pub const EXT_264: &str = "264";
    pub const EXT_H265: &str = "h265";
    pub const EXT_265: &str = "265";
    pub const EXT_AV1: &str = "av1";
}

/// Error message templates
///
/// Using static strings reduces allocations compared to `.to_string()`
pub mod error_msgs {
    pub const FILE_NOT_FOUND: &str = "File not found";
    pub const PATH_NOT_FILE: &str = "Path is not a file";
    pub const INVALID_PATH: &str = "Invalid path";
    pub const CANNOT_ACCESS_SYSTEM_DIR: &str = "Cannot access system directory";
    pub const FILE_TOO_LARGE: &str = "File too large";
    pub const INVALID_DIMENSIONS: &str = "Invalid dimensions";
    pub const FRAME_INDEX_OUT_OF_RANGE: &str = "Frame index out of range";
    pub const NO_FRAME_INDICES: &str = "No frame indices provided";
    pub const DECODING_FAILED: &str = "Decoding failed";
    pub const UNSUPPORTED_FORMAT: &str = "Unsupported format";
    pub const INVALID_PARAMETERS: &str = "Invalid parameters";
    pub const OPERATION_FAILED: &str = "Operation failed";
    pub const RESOURCE_EXHAUSTED: &str = "Resource limit reached";
}

/// Common string helpers to reduce allocations
pub mod strings {
    /// Efficiently format file size with unit
    pub fn format_file_size(bytes: u64) -> String {
        const MB: u64 = 1024 * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }

    /// Efficiently format dimensions
    pub fn format_dimensions(width: u32, height: u32) -> String {
        format!("{}x{}", width, height)
    }

    /// Efficiently format codec name
    pub const fn codec_name_av1() -> &'static str { "AV1" }
    pub const fn codec_name_h264() -> &'static str { "H.264/AVC" }
    pub const fn codec_name_h265() -> &'static str { "H.265/HEVC" }
    pub const fn codec_name_vp9() -> &'static str { "VP9" }
}
