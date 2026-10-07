//! Turns a decoded vvdec picture into the crate's [`DecodedFrame`].
//!
//! Pure safe Rust with no FFI types, so it is unit-tested without libvvdec. `handle.rs` translates
//! a vvdec frame into a [`Picture`]; this module validates it and copies the pixels out.
//!
//! Stride convention (shared with `dav1d.rs`; `bitvue-sidecar/src/decode_bridge.rs` warns about
//! it): `y_plane`/`u_plane`/`v_plane` hold *packed* rows (`width * bytes_per_sample` bytes each),
//! while `y_stride`/`u_stride`/`v_stride` report the decoder's *source* stride.

use crate::decoder::{ChromaFormat, DecodeError, DecodedFrame, FrameType, Result};
use crate::plane_utils::{self, PlaneConfig};

/// Maximum accepted frame dimension. Bounds memory use for hostile streams.
pub(super) const MAX_FRAME_DIMENSION: u32 = 8192;

/// What kind of picture this is, from the NAL and slice type vvdec reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PictureKind {
    /// IDR or CRA: a random access point.
    RandomAccess,
    /// Gradual decoding refresh.
    GradualRefresh,
    /// Intra-only but not a random access point.
    Intra,
    /// Predicted (P/B) or unknown.
    Inter,
}

/// One plane as vvdec hands it over. `data` ends at the last sample of the last row, so it is
/// `(height - 1) * stride + width * bytes_per_sample` bytes long.
#[derive(Debug, Clone, Copy)]
pub(super) struct PlaneView<'a> {
    pub data: &'a [u8],
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bytes_per_sample: usize,
}

/// A decoded picture in neutral types.
#[derive(Debug, Clone, Copy)]
pub(super) struct Picture<'a> {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u32,
    pub chroma: ChromaFormat,
    pub planes: [Option<PlaneView<'a>>; 3],
    pub cts: Option<i64>,
    pub kind: PictureKind,
}

fn decode_err(message: impl Into<String>) -> DecodeError {
    DecodeError::Decode(message.into())
}

/// Copies one plane out as packed rows, checking it against the picture's bit depth.
fn extract(plane: &PlaneView<'_>, bit_depth: u8, name: &str) -> Result<(Vec<u8>, usize)> {
    let expected_bps = if bit_depth > 8 { 2 } else { 1 };
    if plane.bytes_per_sample != expected_bps {
        return Err(decode_err(format!(
            "{name} plane has {} bytes/sample but the picture is {bit_depth}-bit",
            plane.bytes_per_sample
        )));
    }
    let config = PlaneConfig::new(plane.width, plane.height, plane.stride, bit_depth)?;
    let packed = plane_utils::extract_plane(plane.data, config)?;
    Ok((packed, plane.stride))
}

pub(super) fn to_decoded_frame(picture: &Picture<'_>) -> Result<DecodedFrame> {
    let (width, height) = (picture.width, picture.height);
    if width == 0 || height == 0 || width > MAX_FRAME_DIMENSION || height > MAX_FRAME_DIMENSION {
        return Err(decode_err(format!(
            "frame dimensions {width}x{height} outside 1..={MAX_FRAME_DIMENSION}"
        )));
    }
    let bit_depth = u8::try_from(picture.bit_depth)
        .ok()
        .filter(|d| matches!(d, 8 | 10 | 12))
        .ok_or_else(|| decode_err(format!("unsupported bit depth {}", picture.bit_depth)))?;

    let luma = picture.planes[0]
        .as_ref()
        .ok_or_else(|| decode_err("picture has no luma plane"))?;
    if luma.width != width as usize || luma.height != height as usize {
        return Err(decode_err(format!(
            "luma plane is {}x{} but the picture is {width}x{height}",
            luma.width, luma.height
        )));
    }
    let (y_plane, y_stride) = extract(luma, bit_depth, "Y")?;

    let (u, v) = if picture.chroma == ChromaFormat::Monochrome {
        (None, None)
    } else {
        let cb = picture.planes[1]
            .as_ref()
            .ok_or_else(|| decode_err("picture has no Cb plane"))?;
        let cr = picture.planes[2]
            .as_ref()
            .ok_or_else(|| decode_err("picture has no Cr plane"))?;
        (
            Some(extract(cb, bit_depth, "Cb")?),
            Some(extract(cr, bit_depth, "Cr")?),
        )
    };
    let (u_plane, u_stride) = u.map_or((None, 0), |(d, s)| (Some(d), s));
    let (v_plane, v_stride) = v.map_or((None, 0), |(d, s)| (Some(d), s));

    Ok(DecodedFrame {
        width,
        height,
        bit_depth,
        y_plane: y_plane.into(),
        y_stride,
        u_plane: u_plane.map(Into::into),
        u_stride,
        v_plane: v_plane.map(Into::into),
        v_stride,
        timestamp: picture.cts.unwrap_or(0),
        frame_type: match picture.kind {
            PictureKind::RandomAccess => FrameType::Key,
            PictureKind::GradualRefresh | PictureKind::Intra => FrameType::Intra,
            PictureKind::Inter => FrameType::Inter,
        },
        qp_avg: None, // vvdec does not expose QP
        chroma_format: picture.chroma,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plane with `pad` bytes of padding after each row, filled so every sample is
    /// distinguishable: byte at (row, col) = row * 16 + col (mod 251), padding = 0xEE.
    fn padded(width: usize, height: usize, bps: usize, pad: usize) -> (Vec<u8>, usize) {
        let row_bytes = width * bps;
        let stride = row_bytes + pad;
        let mut data = Vec::new();
        for row in 0..height {
            for col in 0..row_bytes {
                data.push(((row * 16 + col) % 251) as u8);
            }
            if row + 1 < height {
                data.extend(std::iter::repeat_n(0xEE, pad)); // none after the last row
            }
        }
        (data, stride)
    }

    fn view(data: &[u8], width: usize, height: usize, stride: usize, bps: usize) -> PlaneView<'_> {
        PlaneView {
            data,
            width,
            height,
            stride,
            bytes_per_sample: bps,
        }
    }

    fn picture<'a>(
        w: u32,
        h: u32,
        depth: u32,
        chroma: ChromaFormat,
        planes: [Option<PlaneView<'a>>; 3],
    ) -> Picture<'a> {
        Picture {
            width: w,
            height: h,
            bit_depth: depth,
            chroma,
            planes,
            cts: Some(7),
            kind: PictureKind::Inter,
        }
    }

    #[test]
    fn strips_row_padding_and_reports_the_source_stride() {
        let (y, ys) = padded(8, 4, 1, 24);
        let (c, cs) = padded(4, 2, 1, 28);
        let pic = picture(
            8,
            4,
            8,
            ChromaFormat::Yuv420,
            [
                Some(view(&y, 8, 4, ys, 1)),
                Some(view(&c, 4, 2, cs, 1)),
                Some(view(&c, 4, 2, cs, 1)),
            ],
        );
        let f = to_decoded_frame(&pic).unwrap();
        assert_eq!(f.y_plane.len(), 8 * 4, "packed rows, padding removed");
        assert!(f.y_plane.iter().all(|b| *b != 0xEE), "padding leaked in");
        assert_eq!(f.y_plane[8], 16, "row 1 starts at the right place");
        assert_eq!((f.y_stride, f.u_stride, f.v_stride), (ys, cs, cs));
        assert_eq!(f.timestamp, 7);
        assert_eq!(f.chroma_format, ChromaFormat::Yuv420);
    }

    #[test]
    fn last_row_needs_no_padding() {
        // The view ends at the last sample: stride * height would overrun it by `pad` bytes.
        let (y, ys) = padded(8, 4, 1, 100);
        assert_eq!(y.len(), 3 * ys + 8);
        let (c, cs) = padded(4, 2, 1, 0);
        let pic = picture(
            8,
            4,
            8,
            ChromaFormat::Yuv420,
            [
                Some(view(&y, 8, 4, ys, 1)),
                Some(view(&c, 4, 2, cs, 1)),
                Some(view(&c, 4, 2, cs, 1)),
            ],
        );
        assert!(to_decoded_frame(&pic).is_ok());
    }

    #[test]
    fn ten_bit_samples_are_two_bytes() {
        let (y, ys) = padded(8, 4, 2, 8);
        let (c, cs) = padded(4, 2, 2, 0);
        let pic = picture(
            8,
            4,
            10,
            ChromaFormat::Yuv420,
            [
                Some(view(&y, 8, 4, ys, 2)),
                Some(view(&c, 4, 2, cs, 2)),
                Some(view(&c, 4, 2, cs, 2)),
            ],
        );
        let f = to_decoded_frame(&pic).unwrap();
        assert_eq!(f.bit_depth, 10);
        assert_eq!(f.y_plane.len(), 8 * 4 * 2);
    }

    #[test]
    fn monochrome_has_no_chroma_planes() {
        let (y, ys) = padded(8, 4, 1, 0);
        let pic = picture(
            8,
            4,
            8,
            ChromaFormat::Monochrome,
            [Some(view(&y, 8, 4, ys, 1)), None, None],
        );
        let f = to_decoded_frame(&pic).unwrap();
        assert!(f.u_plane.is_none() && f.v_plane.is_none());
        assert_eq!((f.u_stride, f.v_stride), (0, 0));
    }

    #[test]
    fn chroma_planes_use_their_own_dimensions() {
        // 4:2:2: chroma is half as wide, full height; the old code assumed height / 2 for 4:2:0
        // and the luma height for everything else, never the plane's own height.
        let (y, ys) = padded(8, 4, 1, 0);
        let (c, cs) = padded(4, 4, 1, 4);
        let pic = picture(
            8,
            4,
            8,
            ChromaFormat::Yuv422,
            [
                Some(view(&y, 8, 4, ys, 1)),
                Some(view(&c, 4, 4, cs, 1)),
                Some(view(&c, 4, 4, cs, 1)),
            ],
        );
        let f = to_decoded_frame(&pic).unwrap();
        assert_eq!(f.u_plane.as_ref().unwrap().len(), 4 * 4);
    }

    #[test]
    fn frame_kind_maps_to_frame_type() {
        let (y, ys) = padded(8, 4, 1, 0);
        let (c, cs) = padded(4, 2, 1, 0);
        let planes = [
            Some(view(&y, 8, 4, ys, 1)),
            Some(view(&c, 4, 2, cs, 1)),
            Some(view(&c, 4, 2, cs, 1)),
        ];
        for (kind, want) in [
            (PictureKind::RandomAccess, FrameType::Key),
            (PictureKind::GradualRefresh, FrameType::Intra),
            (PictureKind::Intra, FrameType::Intra),
            (PictureKind::Inter, FrameType::Inter),
        ] {
            let mut pic = picture(8, 4, 8, ChromaFormat::Yuv420, planes);
            pic.kind = kind;
            assert_eq!(to_decoded_frame(&pic).unwrap().frame_type, want);
        }
    }

    #[test]
    fn rejects_malformed_pictures() {
        let (y, ys) = padded(8, 4, 1, 0);
        let (c, cs) = padded(4, 2, 1, 0);
        let good = [
            Some(view(&y, 8, 4, ys, 1)),
            Some(view(&c, 4, 2, cs, 1)),
            Some(view(&c, 4, 2, cs, 1)),
        ];
        // oversized / zero dimensions
        assert!(to_decoded_frame(&picture(
            MAX_FRAME_DIMENSION + 1,
            4,
            8,
            ChromaFormat::Yuv420,
            good
        ))
        .is_err());
        assert!(to_decoded_frame(&picture(0, 4, 8, ChromaFormat::Yuv420, good)).is_err());
        // unsupported depth
        assert!(to_decoded_frame(&picture(8, 4, 16, ChromaFormat::Yuv420, good)).is_err());
        assert!(to_decoded_frame(&picture(8, 4, 9, ChromaFormat::Yuv420, good)).is_err());
        // 8-bit picture whose planes claim 2 bytes/sample
        let (y2, ys2) = padded(8, 4, 2, 0);
        let two = [Some(view(&y2, 8, 4, ys2, 2)), good[1], good[2]];
        assert!(to_decoded_frame(&picture(8, 4, 8, ChromaFormat::Yuv420, two)).is_err());
        // luma plane size disagrees with the picture
        assert!(to_decoded_frame(&picture(16, 4, 8, ChromaFormat::Yuv420, good)).is_err());
        // missing planes
        assert!(to_decoded_frame(&picture(
            8,
            4,
            8,
            ChromaFormat::Yuv420,
            [None, good[1], good[2]]
        ))
        .is_err());
        assert!(to_decoded_frame(&picture(
            8,
            4,
            8,
            ChromaFormat::Yuv420,
            [good[0], None, good[2]]
        ))
        .is_err());
        // data shorter than the geometry claims
        let short = &y[..y.len() - 1];
        let cut = [Some(view(short, 8, 4, ys, 1)), good[1], good[2]];
        assert!(to_decoded_frame(&picture(8, 4, 8, ChromaFormat::Yuv420, cut)).is_err());
    }
}
