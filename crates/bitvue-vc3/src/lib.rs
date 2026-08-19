//! VC-3 / DNxHD bitstream parser for bitvue.

pub mod error;
pub mod frames;
pub mod overlay_extraction;
pub mod segment;

pub use error::{Result, Vc3Error};
pub use frames::{extract_vc3_frames, ExtractResult, Vc3Frame};
pub use overlay_extraction::{extract_mb_grid, MbGrid};
pub use segment::{parse_frame_header, scan_segments, CompId, FrameHeader, DNXHD_MAGIC};
