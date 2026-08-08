//! Pixel-decode bridge backing the `get_decoded_frame_yuv` command.
//!
//! Lives in `bitvue-sidecar`, not `bitvue-indexer` -- `bitvue-indexer`'s crate doc explicitly
//! scopes it to "no pixel decode" (metadata-indexing only). The sidecar is the orchestration
//! layer that's allowed to depend on both `bitvue-av1-codec` (IVF framing) and `bitvue-decode`
//! (dav1d) directly, the same way it already depends on `bitvue-indexer` for metadata.
//!
//! Correctness-first, not perf-first: re-decodes from the start of the stream up to
//! `frame_index` on every call (same approach `bitvue-cli`'s `decode_av1_yuv` already uses for
//! its `--yuv-dump`). No decoder-session caching, so repeatedly scrubbing a long stream is
//! O(n) in frame index per request -- acceptable for a first working version (the main preview
//! pane currently renders nothing at all), flagged here for a later perf pass rather than
//! solved preemptively.

use bitvue_av1_codec::ivf::parse_ivf_frames;
use bitvue_decode::decoder::ChromaFormat;
use bitvue_decode::{Av1Decoder, DecodedFrame};

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

fn to_wire(frame: &DecodedFrame) -> DecodedYuvFrame {
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
}
