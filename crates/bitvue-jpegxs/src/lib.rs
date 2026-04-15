//! JPEG XS (ISO 21122) bitstream parser for bitvue.
//!
//! Supports marker scanning, picture header parsing, NLT/MCT parameter
//! extraction, and precinct/sub-band overlay data generation.

pub mod error;
pub mod frames;
pub mod marker;
pub mod mct;
pub mod nlt;
pub mod overlay_extraction;
pub mod picture_header;
pub mod syntax;

pub use error::{JpegXsError, Result};
pub use frames::{extract_jpegxs_frames, ExtractResult, JpegXsFrame};
pub use marker::{markers, scan_markers, MarkerSegment};
pub use mct::{MctParams, MctType};
pub use nlt::{NltParams, NltType};
pub use overlay_extraction::{
    extract_dequant_map, extract_mct_info, extract_nlt_info, extract_precinct_map,
    extract_transform_map, DequantMap, MctInfo, NltInfo, PrecinctMap, TransformMap,
};
pub use picture_header::{parse_pih, DecompLevels, PictureHeader, SliceGeometry};
