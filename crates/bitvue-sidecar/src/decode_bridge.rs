//! Pixel-decode bridge backing the `get_decoded_frame_yuv` and `get_thumbnails` commands.
//!
//! Lives in `bitvue-sidecar`, not `bitvue-indexer` -- `bitvue-indexer`'s crate doc explicitly
//! scopes it to "no pixel decode" (metadata-indexing only). The sidecar is the orchestration
//! layer that's allowed to depend on both `bitvue-av1-codec` (IVF framing) and `bitvue-decode`
//! (dav1d) directly, the same way it already depends on `bitvue-indexer` for metadata.
//!
//! Correctness-first, not perf-first: re-decodes from the start of the stream up to the target
//! frame on every call (same approach `bitvue-cli`'s `decode_av1_yuv` already uses for its
//! `--yuv-dump`). No decoder-session caching, so repeatedly scrubbing a long stream is O(n) in
//! frame index per request -- acceptable for a first working version (the main preview pane
//! and filmstrip previously rendered nothing at all), flagged here for a later perf pass rather
//! than solved preemptively. `get_thumbnails` does at least batch this: one decode pass captures
//! every requested index up to the batch's max, rather than one pass per index.

use bitvue_av1_codec::ivf::parse_ivf_frames;
use bitvue_decode::decoder::ChromaFormat;
use bitvue_decode::{Av1Decoder, DecodedFrame};
use bitvue_engine::{CachedFrame, Thumbnail, ThumbnailCache};
use std::collections::HashSet;

/// Wire-ready decoded frame: metadata fields plus the concatenated Y+U+V byte buffer sent as
/// the `Data` frame that follows this command's `Control` response (see `get_hex_range` for the
/// established no-base64 two-frame pattern this mirrors). The TS side slices `bytes` back into
/// three planes using `y_len`/`u_len`/`v_len`.
pub struct DecodedYuvFrame {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub chroma_subsampling: &'static str,
    pub y_stride: usize,
    pub u_stride: usize,
    pub v_stride: usize,
    pub y_len: usize,
    pub u_len: usize,
    pub v_len: usize,
    pub bytes: Vec<u8>,
}

pub fn get_decoded_frame_yuv(data: &[u8], frame_index: usize) -> Result<DecodedYuvFrame, String> {
    let (_hdr, frames) = parse_ivf_frames(data).map_err(|e| format!("IVF parse error: {e}"))?;
    if frame_index >= frames.len() {
        return Err(format!(
            "frame_index {frame_index} out of range (stream has {} frames)",
            frames.len()
        ));
    }

    let mut dec = Av1Decoder::new().map_err(|e| format!("decoder init: {e}"))?;
    let mut decoded: Vec<DecodedFrame> = Vec::new();

    for f in &frames {
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| format!("decode send: {e}"))?;
        drain(&mut dec, &mut decoded);
        if decoded.len() > frame_index {
            break;
        }
    }
    if decoded.len() <= frame_index {
        dec.flush();
        drain(&mut dec, &mut decoded);
    }

    let frame = decoded.get(frame_index).ok_or_else(|| {
        format!(
            "decoder produced only {} frame(s), needed index {frame_index}",
            decoded.len()
        )
    })?;

    Ok(to_wire(frame))
}

/// Matches `bitvue-cli`'s `drain_frames_yuv`: any `get_frame()` error just means "no frame ready
/// yet", not a fatal condition -- stop draining, the caller will feed more data or flush.
fn drain(dec: &mut Av1Decoder, out: &mut Vec<DecodedFrame>) {
    while let Ok(f) = dec.get_frame() {
        out.push(f);
    }
}

pub(crate) fn to_wire(frame: &DecodedFrame) -> DecodedYuvFrame {
    let chroma_subsampling = match frame.chroma_format {
        ChromaFormat::Yuv420 | ChromaFormat::Monochrome => "420",
        ChromaFormat::Yuv422 => "422",
        ChromaFormat::Yuv444 => "444",
    };
    // NOT frame.y_stride/u_stride/v_stride -- those are dav1d's *source* picture strides
    // (before extraction), left on `DecodedFrame` as leftover/informational fields.
    // `bitvue_decode::plane_utils::extract_plane` always copies planes out tightly packed
    // (row length == plane width, no padding), so the *real* stride of `y_plane`/`u_plane`/
    // `v_plane`'s bytes is just the plane width -- using the stale source stride here produced
    // a badly garbled image (each row read from the wrong offset). Matches the same
    // width-halving `bitvue-decode::decoder::get_frame` itself uses when extracting chroma.
    let chroma_width = match frame.chroma_format {
        ChromaFormat::Yuv420 | ChromaFormat::Yuv422 => frame.width as usize / 2,
        ChromaFormat::Yuv444 => frame.width as usize,
        ChromaFormat::Monochrome => 0,
    };
    let u_bytes: &[u8] = frame.u_plane.as_deref().unwrap_or(&[]);
    let v_bytes: &[u8] = frame.v_plane.as_deref().unwrap_or(&[]);

    let mut bytes = Vec::with_capacity(frame.y_plane.len() + u_bytes.len() + v_bytes.len());
    bytes.extend_from_slice(&frame.y_plane);
    bytes.extend_from_slice(u_bytes);
    bytes.extend_from_slice(v_bytes);

    DecodedYuvFrame {
        width: frame.width,
        height: frame.height,
        bit_depth: frame.bit_depth,
        chroma_subsampling,
        y_stride: frame.width as usize,
        u_stride: chroma_width,
        v_stride: chroma_width,
        y_len: frame.y_plane.len(),
        u_len: u_bytes.len(),
        v_len: v_bytes.len(),
        bytes,
    }
}

/// One filmstrip thumbnail: a small PNG, base64-encoded as a `data:` URL -- the exact shape
/// `frontend/components/Filmstrip/views/*ThumbnailsView.tsx` already feeds straight into an
/// `<img src=...>`, so no frontend-side format translation is needed (unlike the raw-bytes
/// approach used for full-resolution `get_decoded_frame_yuv` -- thumbnails are small enough that
/// base64 JSON is a reasonable, not wasteful, choice here).
pub struct ThumbnailResult {
    pub frame_index: usize,
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

/// Decodes once, capturing a thumbnail at every requested index along the way, rather than one
/// full decode pass per index (`frame_indices` is typically a batch of up to
/// `THUMBNAIL_BATCH_SIZE` -- see `frontend/constants/ui.ts`). `target_width` matches
/// `ThumbnailCache::default()`'s 120px (`THUMBNAIL_SIZE.WIDTH` on the frontend) unless overridden.
pub fn get_thumbnails(
    data: &[u8],
    frame_indices: &[usize],
    target_width: u32,
) -> Result<Vec<ThumbnailResult>, String> {
    let (_hdr, frames) = parse_ivf_frames(data).map_err(|e| format!("IVF parse error: {e}"))?;
    let wanted: HashSet<usize> = frame_indices.iter().copied().collect();
    let max_wanted = match wanted.iter().max() {
        Some(&m) => m,
        None => return Ok(Vec::new()),
    };
    if max_wanted >= frames.len() {
        return Err(format!(
            "frame_index {max_wanted} out of range (stream has {} frames)",
            frames.len()
        ));
    }

    let mut dec = Av1Decoder::new().map_err(|e| format!("decoder init: {e}"))?;
    let mut results = Vec::with_capacity(wanted.len());
    let mut decoded_count = 0usize;

    let capture = |decoded: &DecodedFrame, index: usize, out: &mut Vec<ThumbnailResult>| {
        if !wanted.contains(&index) {
            return;
        }
        let rgb_data = bitvue_decode::yuv_to_rgb(decoded);
        let cached = CachedFrame {
            index,
            rgb_data,
            width: decoded.width,
            height: decoded.height,
            decoded: true,
            error: None,
            y_plane: None,
            u_plane: None,
            v_plane: None,
            chroma_width: None,
            chroma_height: None,
        };
        let thumb = ThumbnailCache::generate_thumbnail(&cached, target_width);
        out.push(ThumbnailResult {
            frame_index: index,
            data_url: thumbnail_to_png_data_url(&thumb),
            width: thumb.width,
            height: thumb.height,
        });
    };

    for f in &frames {
        dec.send_data_owned(f.data.clone(), f.timestamp as i64)
            .map_err(|e| format!("decode send: {e}"))?;
        while let Ok(frame) = dec.get_frame() {
            capture(&frame, decoded_count, &mut results);
            decoded_count += 1;
        }
        if decoded_count > max_wanted {
            break;
        }
    }
    if decoded_count <= max_wanted {
        dec.flush();
        while let Ok(frame) = dec.get_frame() {
            capture(&frame, decoded_count, &mut results);
            decoded_count += 1;
        }
    }

    Ok(results)
}

fn thumbnail_to_png_data_url(thumb: &Thumbnail) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use image::{DynamicImage, ImageFormat, RgbImage};
    use std::io::Cursor;

    let img = RgbImage::from_raw(thumb.width, thumb.height, thumb.rgb_data.clone())
        .expect("Thumbnail's rgb_data is always width*height*3 bytes, see generate_thumbnail");
    let mut png_bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut png_bytes, ImageFormat::Png)
        .expect("in-memory PNG encode does not fail");
    format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png_bytes.into_inner())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

    /// Ground-truth pin: byte values independently verified against `bitvue-cli`'s
    /// `decode --dump` output for the same frame (`0f 10 10 10 d5 d5 d6 d6 ...`). Also pins the
    /// real bug this test would have caught: reporting dav1d's *source* stride (384, with
    /// padding) instead of the tightly-packed extraction stride (320 == width) made every row
    /// after the first read from the wrong offset, producing a badly garbled image even though
    /// the underlying decoded bytes were already correct.
    #[test]
    fn frame_zero_matches_cli_ground_truth_and_uses_a_tightly_packed_stride() {
        let frame = get_decoded_frame_yuv(AV1_IVF_FIXTURE, 0).unwrap();

        assert_eq!(frame.width, 320);
        assert_eq!(frame.height, 240);
        assert_eq!(
            frame.y_stride, frame.width as usize,
            "y_plane is tightly packed (no dav1d row padding survives extraction) -- \
             y_stride must equal width, not dav1d's source picture stride"
        );
        assert_eq!(frame.y_len, frame.width as usize * frame.height as usize);

        let y = &frame.bytes[..frame.y_len];
        assert_eq!(
            &y[..16],
            &[15, 16, 16, 16, 213, 213, 214, 214, 17, 17, 17, 17, 212, 212, 212, 212],
        );
    }

    #[test]
    fn out_of_range_frame_index_is_a_real_error() {
        let result = get_decoded_frame_yuv(AV1_IVF_FIXTURE, 999_999);
        assert!(result.is_err());
    }

    // -- get_thumbnails ----------------------------------------------------------------------

    #[test]
    fn get_thumbnails_returns_one_real_decodable_png_per_requested_index() {
        let results = get_thumbnails(AV1_IVF_FIXTURE, &[0, 5, 10], 120).unwrap();
        assert_eq!(results.len(), 3);

        let mut by_index: Vec<&ThumbnailResult> = results.iter().collect();
        by_index.sort_by_key(|t| t.frame_index);
        assert_eq!(
            by_index.iter().map(|t| t.frame_index).collect::<Vec<_>>(),
            vec![0, 5, 10]
        );

        for thumb in &results {
            assert_eq!(thumb.width, 120, "target_width should be honored exactly");
            assert!(thumb.height > 0);
            let prefix = "data:image/png;base64,";
            assert!(
                thumb.data_url.starts_with(prefix),
                "expected a PNG data URL, got: {}...",
                &thumb.data_url[..prefix.len().min(thumb.data_url.len())]
            );

            // Round-trip through a real PNG decoder to prove this isn't just a string that
            // happens to start with the right prefix -- it must actually decode.
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            let png_bytes = STANDARD.decode(&thumb.data_url[prefix.len()..]).unwrap();
            let decoded = image::load_from_memory(&png_bytes).unwrap();
            assert_eq!(decoded.width(), thumb.width);
            assert_eq!(decoded.height(), thumb.height);
        }
    }

    #[test]
    fn get_thumbnails_empty_request_returns_empty_not_an_error() {
        let results = get_thumbnails(AV1_IVF_FIXTURE, &[], 120).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn get_thumbnails_out_of_range_index_is_a_real_error() {
        let result = get_thumbnails(AV1_IVF_FIXTURE, &[0, 999_999], 120);
        assert!(result.is_err());
    }
}
