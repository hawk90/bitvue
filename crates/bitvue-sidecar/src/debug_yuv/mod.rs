//! Debug YUV (VQ Analyzer "Load Reference YUV") -- raw planar/semi-planar YUV file reading and
//! frame-level PSNR/SSIM/diff comparison against stream A's decoded output. Distinct from
//! `decode_bridge`: that module decodes *compressed* bitstream frames; this reads *uncompressed*
//! samples straight off disk, no decode -- the "ground truth" reference a user loads via
//! Debug -> Open debug YUV... to check a decoder/encoder against (see
//! `frontend/contexts/YuvDiffContext.tsx`'s module doc for the workflow this backs).
//!
//! Session state (path/dimensions/format/bitdepth/offset/crop) lives in a `Mutex<Option<Session>>`
//! held by `main.rs` (`DebugYuvSlot`), not `bitvue_engine::Core` -- it isn't per-`StreamId` (A/B)
//! state, and `Core` is a leaf crate that can't depend on filesystem/decode concerns anyway (same
//! reasoning as `decode_bridge` living here rather than in `bitvue-indexer`).
//!
//! **Scoping decision**: diff/amplified/metrics all operate at 8-bit precision regardless of the
//! reference file's declared bit depth -- samples above 8 bits are right-shifted down before
//! comparison. This matches `decode_bridge`'s existing wire format (always 8-bit-packed bytes) and
//! the frontend's `VideoCanvas`/`yuv_to_rgb` rendering pipeline, neither of which has a 10/12/16-bit
//! path today. Real high-bit-depth comparison would need a second wire format and renderer path --
//! out of scope here, flagged for later if a real 10-bit+ test asset needs it. "reference" mode
//! (no decoded-frame comparison) still reads the file at its declared bit depth, just downshifts
//! for the wire the same way, so what's displayed is consistent across all four modes.
//!
//! Split by concern: this file owns the session/format model (`Session`/`LoadParams`/`YuvFormat`);
//! [`planes`] owns raw file I/O and 8-bit plane normalization (`Planes8`, `read_reference_frame`);
//! [`compare`] owns the comparison algorithms (`get_frame`, `compute_frame_metrics`,
//! `find_first_diff`); [`commands`] owns the wire-protocol command handlers.

mod commands;
mod compare;
mod planes;

pub use commands::{
    find_first_diff_frame, get_debug_yuv_frame, get_yuv_diff_metrics, load_debug_yuv,
    set_debug_yuv_crop, set_debug_yuv_offset, unload_debug_yuv,
};
pub(crate) use planes::frame_byte_size;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YuvFormat {
    I420,
    Nv12,
    Nv21,
    I422,
    I444,
}

impl YuvFormat {
    /// (horizontal, vertical) chroma subsampling ratio -- how much smaller each chroma plane's
    /// dimension is than luma's. Same ratio for planar (I420/I422/I444) and semi-planar
    /// (NV12/NV21) layouts; only the byte arrangement in the file differs, not the sample grid.
    fn chroma_ratio(self) -> (u32, u32) {
        match self {
            YuvFormat::I420 | YuvFormat::Nv12 | YuvFormat::Nv21 => (2, 2),
            YuvFormat::I422 => (2, 1),
            YuvFormat::I444 => (1, 1),
        }
    }

    fn chroma_subsampling_str(self) -> &'static str {
        match self {
            YuvFormat::I420 | YuvFormat::Nv12 | YuvFormat::Nv21 => "420",
            YuvFormat::I422 => "422",
            YuvFormat::I444 => "444",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct Crop {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoadParams {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: YuvFormat,
    pub bitdepth: u8,
    #[serde(default)]
    pub picture_offset: i64,
    #[serde(default)]
    pub crop: Option<Crop>,
}

/// A loaded reference-YUV session. `picture_offset`/`crop` are mutated in place by
/// `set_debug_yuv_offset`/`set_debug_yuv_crop` (see `main.rs`) rather than replaced wholesale --
/// re-`load`ing would re-stat the file and re-validate frame_size for no reason.
#[derive(Debug, Clone)]
pub struct Session {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: YuvFormat,
    pub bitdepth: u8,
    pub picture_offset: i64,
    pub crop: Crop,
    pub frame_size: usize,
    pub frame_count: usize,
}

pub fn load(params: LoadParams) -> Result<Session, String> {
    if params.width == 0 || params.height == 0 {
        return Err("width/height must be non-zero".to_string());
    }
    if !matches!(params.bitdepth, 8 | 10 | 12 | 16) {
        return Err(format!("unsupported bit depth: {}", params.bitdepth));
    }
    let meta =
        std::fs::metadata(&params.path).map_err(|e| format!("cannot open {}: {e}", params.path))?;
    let frame_size = frame_byte_size(params.width, params.height, params.format, params.bitdepth);
    let frame_count = (meta.len() as usize) / frame_size;
    if frame_count == 0 {
        return Err(format!(
            "file is smaller than one frame ({} bytes < {frame_size} bytes/frame -- check resolution/format/bit depth)",
            meta.len()
        ));
    }
    Ok(Session {
        path: params.path,
        width: params.width,
        height: params.height,
        format: params.format,
        bitdepth: params.bitdepth,
        picture_offset: params.picture_offset,
        crop: params.crop.unwrap_or_default(),
        frame_size,
        frame_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_i420_fixture;

    #[test]
    fn load_computes_real_frame_count_from_file_size() {
        let file = write_i420_fixture(4, 4, &[0, 1, 2]); // 3 frames of 24 bytes each
        let session = load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        })
        .unwrap();
        assert_eq!(session.frame_count, 3);
        assert_eq!(session.frame_size, 24);
    }

    #[test]
    fn load_file_smaller_than_one_frame_is_a_real_error() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file, &[0u8; 5]).unwrap();
        let result = load(LoadParams {
            path: file.path().to_str().unwrap().to_string(),
            width: 4,
            height: 4,
            format: YuvFormat::I420,
            bitdepth: 8,
            picture_offset: 0,
            crop: None,
        });
        assert!(result.is_err());
    }
}
