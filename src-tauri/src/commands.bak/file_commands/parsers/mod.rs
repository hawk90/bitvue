//! File format parsers

mod ivf_parser;
mod mp4_parser;
mod mkv_parser;
mod ts_parser;

use bitvue_core::{StreamId, UnitNode};
use std::path::PathBuf;

pub use ivf_parser::parse_ivf_file;
pub use mp4_parser::parse_mp4_file;
pub use mkv_parser::parse_mkv_file;
pub use ts_parser::parse_ts_file;

/// Parse file based on extension
pub fn parse_by_extension(path: &PathBuf, ext: &str, stream_id: StreamId) -> Result<Vec<UnitNode>, String> {
    match ext {
        "ivf" | "av1" => {
            tracing::info!("Parsing AV1/IVF file...");
            parse_ivf_file(path, stream_id)
        }
        "mp4" | "mov" | "m4v" => {
            tracing::info!("Parsing MP4 file...");
            parse_mp4_file(path, stream_id)
        }
        "mkv" | "webm" => {
            tracing::info!("Parsing MKV/WebM file...");
            parse_mkv_file(path, stream_id)
        }
        "ts" | "m2ts" => {
            tracing::info!("Parsing MPEG-TS file...");
            parse_ts_file(path, stream_id)
        }
        _ => {
            tracing::warn!("Unsupported format for parsing: {}, attempting IVF", ext);
            parse_ivf_file(path, stream_id)
        }
    }
}

/// Detect codec from file extension
pub fn detect_codec_from_extension(ext: &str) -> String {
    match ext {
        "ivf" | "av1" => "AV1",
        "webm" => "AV1",
        "hevc" | "265" => "HEVC",
        "h264" | "264" => "AVC",
        "vvc" | "266" => "VVC",
        "mp4" | "mov" | "m4v" => "MP4",
        "mkv" => "MKV",
        "ts" | "m2ts" => "TS",
        _ => {
            tracing::warn!("Unknown codec extension: {}", ext);
            ext
        }
    }.to_string()
}
