//! YUV to RGB conversion utilities

use crate::decoder::DecodedFrame;
use tracing::debug;

/// Chroma subsampling format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChromaFormat {
    /// 4:2:0 - U and V are 1/2 resolution in both dimensions
    Yuv420,
    /// 4:2:2 - U and V are 1/2 resolution horizontally only
    Yuv422,
    /// 4:4:4 - U and V are full resolution
    Yuv444,
    /// Monochrome - Y only
    Monochrome,
}

/// Detect chroma format from plane sizes
fn detect_chroma_format(frame: &DecodedFrame) -> ChromaFormat {
    match (&frame.u_plane, &frame.v_plane) {
        (Some(u_plane), Some(_)) => {
            let width = frame.width as usize;
            let height = frame.height as usize;
            let bytes_per_sample = if frame.bit_depth > 8 { 2 } else { 1 };

            let y_size = width * height * bytes_per_sample;
            let uv_size = u_plane.len();

            if uv_size == y_size {
                ChromaFormat::Yuv444
            } else if uv_size == (width / 2) * height * bytes_per_sample {
                ChromaFormat::Yuv422
            } else if uv_size == (width / 2) * (height / 2) * bytes_per_sample {
                ChromaFormat::Yuv420
            } else {
                debug!(
                    "Unknown chroma format: Y={}, UV={}, {}bit, assuming 4:2:0",
                    y_size, uv_size, frame.bit_depth
                );
                ChromaFormat::Yuv420
            }
        }
        _ => ChromaFormat::Monochrome,
    }
}

/// Read a sample from plane data, handling 8/10/12bit
#[inline]
fn read_sample(plane: &[u8], idx: usize, bit_depth: u8) -> u8 {
    if bit_depth > 8 {
        // 10/12bit: read 16-bit sample and normalize to 8-bit
        let byte_idx = idx * 2;
        if byte_idx + 1 < plane.len() {
            let sample16 = u16::from_le_bytes([plane[byte_idx], plane[byte_idx + 1]]);
            // Normalize to 8-bit by right-shifting
            (sample16 >> (bit_depth - 8)) as u8
        } else {
            0
        }
    } else {
        // 8bit: direct read
        plane.get(idx).copied().unwrap_or(0)
    }
}

/// Converts a decoded YUV frame to RGB
pub fn yuv_to_rgb(frame: &DecodedFrame) -> Vec<u8> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let mut rgb = vec![0u8; width * height * 3];

    let chroma_format = detect_chroma_format(frame);
    let bit_depth = frame.bit_depth;
    debug!(
        "Converting {:?} frame to RGB ({}x{}, {}bit)",
        chroma_format, width, height, bit_depth
    );

    match chroma_format {
        ChromaFormat::Monochrome => {
            // Y only - grayscale
            for i in 0..(width * height) {
                let y_val = read_sample(&frame.y_plane, i, bit_depth);
                let rgb_idx = i * 3;
                rgb[rgb_idx] = y_val;
                rgb[rgb_idx + 1] = y_val;
                rgb[rgb_idx + 2] = y_val;
            }
        }
        ChromaFormat::Yuv420 => {
            let u_plane = match frame.u_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv420 frame missing U plane, falling back to grayscale");
                    // Fallback to grayscale
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };
            let v_plane = match frame.v_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv420 frame missing V plane, falling back to grayscale");
                    // Fallback to grayscale
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };

            for y in 0..height {
                for x in 0..width {
                    let y_idx = y * width + x;
                    let uv_idx = (y / 2) * (width / 2) + (x / 2);

                    let y_val = read_sample(&frame.y_plane, y_idx, bit_depth) as f32;
                    let u_val = read_sample(u_plane, uv_idx, bit_depth) as f32 - 128.0;
                    let v_val = read_sample(v_plane, uv_idx, bit_depth) as f32 - 128.0;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);

                    let rgb_idx = y_idx * 3;
                    rgb[rgb_idx] = r;
                    rgb[rgb_idx + 1] = g;
                    rgb[rgb_idx + 2] = b;
                }
            }
        }
        ChromaFormat::Yuv422 => {
            let u_plane = match frame.u_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv422 frame missing U plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };
            let v_plane = match frame.v_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv422 frame missing V plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };

            for y in 0..height {
                for x in 0..width {
                    let y_idx = y * width + x;
                    let uv_idx = y * (width / 2) + (x / 2);

                    let y_val = read_sample(&frame.y_plane, y_idx, bit_depth) as f32;
                    let u_val = read_sample(u_plane, uv_idx, bit_depth) as f32 - 128.0;
                    let v_val = read_sample(v_plane, uv_idx, bit_depth) as f32 - 128.0;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);

                    let rgb_idx = y_idx * 3;
                    rgb[rgb_idx] = r;
                    rgb[rgb_idx + 1] = g;
                    rgb[rgb_idx + 2] = b;
                }
            }
        }
        ChromaFormat::Yuv444 => {
            let u_plane = match frame.u_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv444 frame missing U plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };
            let v_plane = match frame.v_plane.as_ref() {
                Some(plane) => plane,
                None => {
                    tracing::error!("Yuv444 frame missing V plane, falling back to grayscale");
                    for i in 0..(width * height) {
                        let y_val = read_sample(&frame.y_plane, i, bit_depth);
                        let rgb_idx = i * 3;
                        rgb[rgb_idx] = y_val;
                        rgb[rgb_idx + 1] = y_val;
                        rgb[rgb_idx + 2] = y_val;
                    }
                    return rgb;
                }
            };

            for y in 0..height {
                for x in 0..width {
                    let idx = y * width + x;

                    let y_val = read_sample(&frame.y_plane, idx, bit_depth) as f32;
                    let u_val = read_sample(u_plane, idx, bit_depth) as f32 - 128.0;
                    let v_val = read_sample(v_plane, idx, bit_depth) as f32 - 128.0;

                    let (r, g, b) = yuv_to_rgb_pixel(y_val, u_val, v_val);

                    let rgb_idx = idx * 3;
                    rgb[rgb_idx] = r;
                    rgb[rgb_idx + 1] = g;
                    rgb[rgb_idx + 2] = b;
                }
            }
        }
    }

    rgb
}

/// Convert a single YUV pixel to RGB using BT.601 color space
#[inline]
fn yuv_to_rgb_pixel(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    // BT.601 conversion matrix
    let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
    (r, g, b)
}

/// Converts RGB data to an image::RgbImage
pub fn rgb_to_image(rgb: &[u8], width: u32, height: u32) -> image::RgbImage {
    image::RgbImage::from_raw(width, height, rgb.to_vec())
        .expect("Failed to create image from RGB data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monochrome_conversion() {
        let frame = DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0, 128, 255, 64],
            y_stride: 2,
            u_plane: None,
            u_stride: 0,
            v_plane: None,
            v_stride: 0,
            timestamp: 0,
            frame_type: crate::decoder::FrameType::Key,
            qp_avg: None,
        };

        let rgb = yuv_to_rgb(&frame);
        assert_eq!(rgb.len(), 12); // 2x2x3

        // First pixel (Y=0) -> RGB(0,0,0)
        assert_eq!(rgb[0], 0);
        assert_eq!(rgb[1], 0);
        assert_eq!(rgb[2], 0);

        // Second pixel (Y=128)
        assert_eq!(rgb[3], 128);
    }
}
