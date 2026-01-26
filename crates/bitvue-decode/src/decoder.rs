//! AV1 decoder wrapper using dav1d

use dav1d::{Decoder, PlanarImageComponent};
use thiserror::Error;
use tracing::{debug, error, warn};

/// Decoder errors
#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Decoder initialization failed: {0}")]
    Init(String),

    #[error("Decode failed: {0}")]
    Decode(String),

    #[error("No frame available")]
    NoFrame,

    #[error("Unsupported pixel format")]
    UnsupportedFormat,
}

pub type Result<T> = std::result::Result<T, DecodeError>;

/// A decoded video frame
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Bit depth (8, 10, or 12)
    pub bit_depth: u8,
    /// Y plane data
    pub y_plane: Vec<u8>,
    /// Y plane stride
    pub y_stride: usize,
    /// U plane data (None for monochrome)
    pub u_plane: Option<Vec<u8>>,
    /// U plane stride
    pub u_stride: usize,
    /// V plane data (None for monochrome)
    pub v_plane: Option<Vec<u8>>,
    /// V plane stride
    pub v_stride: usize,
    /// Frame timestamp
    pub timestamp: i64,
    /// Frame type (key, inter, etc.)
    pub frame_type: FrameType,
    /// Average QP for the frame (0-255 for AV1, 0-51 for H.264/HEVC)
    /// None if QP information is not available
    pub qp_avg: Option<u8>,
}

/// Frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Key,
    Inter,
    Intra,
    Switch,
}

/// AV1 decoder using dav1d
pub struct Av1Decoder {
    decoder: Decoder,
}

// Implement the universal Decoder trait for Av1Decoder
impl crate::traits::Decoder for Av1Decoder {
    fn codec_type(&self) -> crate::traits::CodecType {
        crate::traits::CodecType::AV1
    }

    fn capabilities(&self) -> crate::traits::DecoderCapabilities {
        crate::traits::DecoderCapabilities {
            codec: crate::traits::CodecType::AV1,
            max_width: 8192,
            max_height: 8192,
            supported_bit_depths: vec![8, 10, 12],
            hw_accel: false, // dav1d doesn't expose HW accel through Rust bindings
        }
    }

    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()> {
        self.send_data(data, timestamp.unwrap_or(0))
    }

    fn get_frame(&mut self) -> Result<DecodedFrame> {
        self.get_frame()
    }

    fn decode_all(&mut self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
        self.decode_all(data)
    }

    fn flush(&mut self) {
        self.flush()
    }

    fn reset(&mut self) -> Result<()> {
        // Create a new decoder to reset state
        let new_decoder = Self::new()?;
        *self = new_decoder;
        Ok(())
    }
}

impl Av1Decoder {
    /// Creates a new AV1 decoder
    pub fn new() -> Result<Self> {
        let decoder = Decoder::new().map_err(|e| DecodeError::Init(e.to_string()))?;
        Ok(Self { decoder })
    }

    /// Sends data to the decoder
    pub fn send_data(&mut self, data: &[u8], timestamp: i64) -> Result<()> {
        self.decoder
            .send_data(data.to_vec(), None, Some(timestamp), None)
            .map_err(|e| DecodeError::Decode(e.to_string()))
    }

    /// Gets the next decoded frame
    pub fn get_frame(&mut self) -> Result<DecodedFrame> {
        let picture = self.decoder.get_picture().map_err(|e| {
            let err_str = e.to_string();
            // EAGAIN/"Try again" is expected when no frame is ready yet - not an error
            if err_str.contains("EAGAIN") || err_str.contains("Try again") {
                debug!("No frame available yet (EAGAIN) - this is normal during decoding");
                DecodeError::NoFrame
            } else {
                error!("Failed to get picture from decoder: {}", e);
                DecodeError::Decode(err_str)
            }
        })?;

        let frame = self.picture_to_frame(&picture)?;

        // Validate frame before returning
        validate_frame(&frame)?;

        debug!(
            "Decoded frame: {}x{} {}bit {:?}",
            frame.width, frame.height, frame.bit_depth, frame.frame_type
        );

        Ok(frame)
    }

    /// Convert a dav1d Picture to DecodedFrame
    fn picture_to_frame(&self, picture: &dav1d::Picture) -> Result<DecodedFrame> {
        let width = picture.width();
        let height = picture.height();
        let bit_depth = picture.bit_depth() as u8;

        // Extract Y plane
        let y_plane_ref = picture.plane(PlanarImageComponent::Y);
        let y_stride = picture.stride(PlanarImageComponent::Y) as usize;
        let y_plane = extract_plane(
            &y_plane_ref,
            width as usize,
            height as usize,
            y_stride,
            bit_depth,
        );

        // Extract U and V planes (if not monochrome)
        let (u_plane, u_stride, v_plane, v_stride) =
            if picture.pixel_layout() != dav1d::PixelLayout::I400 {
                let chroma_width = match picture.pixel_layout() {
                    dav1d::PixelLayout::I420 | dav1d::PixelLayout::I422 => width as usize / 2,
                    dav1d::PixelLayout::I444 => width as usize,
                    dav1d::PixelLayout::I400 => 0,
                };
                let chroma_height = match picture.pixel_layout() {
                    dav1d::PixelLayout::I420 => height as usize / 2,
                    dav1d::PixelLayout::I422 | dav1d::PixelLayout::I444 => height as usize,
                    dav1d::PixelLayout::I400 => 0,
                };

                let u_plane_ref = picture.plane(PlanarImageComponent::U);
                let u_stride = picture.stride(PlanarImageComponent::U) as usize;
                let u_plane = extract_plane(
                    &u_plane_ref,
                    chroma_width,
                    chroma_height,
                    u_stride,
                    bit_depth,
                );

                let v_plane_ref = picture.plane(PlanarImageComponent::V);
                let v_stride = picture.stride(PlanarImageComponent::V) as usize;
                let v_plane = extract_plane(
                    &v_plane_ref,
                    chroma_width,
                    chroma_height,
                    v_stride,
                    bit_depth,
                );

                (Some(u_plane), u_stride, Some(v_plane), v_stride)
            } else {
                (None, 0, None, 0)
            };

        // Frame type detection based on picture properties
        let frame_type = FrameType::Key; // TODO: detect from picture metadata

        Ok(DecodedFrame {
            width,
            height,
            bit_depth,
            y_plane,
            y_stride,
            u_plane,
            u_stride,
            v_plane,
            v_stride,
            timestamp: picture.timestamp().unwrap_or(0),
            frame_type,
            qp_avg: None, // TODO: Extract QP from bitstream parser
        })
    }

    /// Decodes all frames from data (supports IVF container or raw OBU)
    pub fn decode_all(&mut self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
        // Check if it's IVF format and extract frame data
        if data.len() >= 4 && &data[0..4] == b"DKIF" {
            // IVF container - extract frames and decode each separately
            self.decode_ivf(data)
        } else {
            // Raw OBU data
            self.send_data(data, 0)?;
            self.collect_frames()
        }
    }

    /// Decode IVF container data
    fn decode_ivf(&mut self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
        // Parse IVF header
        if data.len() < 32 {
            return Err(DecodeError::Decode("IVF data too short".to_string()));
        }

        let header_size = u16::from_le_bytes([data[6], data[7]]) as usize;
        let mut offset = header_size;
        let mut frame_idx = 0i64;
        let mut decoded_frames = Vec::new();

        // Iterate through IVF frames
        while offset + 12 <= data.len() {
            let frame_size = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;

            offset += 12; // Skip size (4) + timestamp (8)

            if offset + frame_size > data.len() {
                break;
            }

            // Send frame data to decoder
            let frame_data = &data[offset..offset + frame_size];
            if let Err(e) = self.send_data(frame_data, frame_idx) {
                warn!(
                    "Failed to send IVF frame {} to decoder: {}. Skipping frame.",
                    frame_idx, e
                );
            } else {
                // Try to get frames after successful send
                while let Ok(frame) = self.get_frame() {
                    decoded_frames.push(frame);
                }
            }

            frame_idx += 1;
            offset += frame_size;
        }

        // Drain remaining frames from decoder
        for _ in 0..100 {
            match self.decoder.get_picture() {
                Ok(picture) => {
                    if let Ok(frame) = self.picture_to_frame(&picture) {
                        decoded_frames.push(frame);
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("EAGAIN") || err_str.contains("Try again") {
                        continue;
                    }
                    break;
                }
            }
        }

        // Ensure at least one frame was decoded
        if decoded_frames.is_empty() {
            return Err(DecodeError::Decode(
                "Failed to decode any frames from IVF file".to_string(),
            ));
        }

        Ok(decoded_frames)
    }

    /// Collect decoded frames from the decoder
    fn collect_frames(&mut self) -> Result<Vec<DecodedFrame>> {
        let mut frames = Vec::new();
        loop {
            match self.get_frame() {
                Ok(frame) => frames.push(frame),
                Err(DecodeError::NoFrame) => break, // No more frames available
                Err(DecodeError::Decode(_)) => break, // Decode error, stop trying
                Err(e) => return Err(e),
            }
        }
        Ok(frames)
    }

    /// Flushes the decoder
    pub fn flush(&mut self) {
        self.decoder.flush();
    }
}

impl Default for Av1Decoder {
    fn default() -> Self {
        Self::new().expect("Failed to create decoder")
    }
}

/// Extracts plane data from dav1d plane reference
fn extract_plane(
    plane: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bit_depth: u8,
) -> Vec<u8> {
    // For 10/12bit, dav1d returns 16-bit samples (2 bytes per sample)
    let bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };
    let row_bytes = width * bytes_per_sample;

    let mut data = Vec::with_capacity(row_bytes * height);
    for row in 0..height {
        let start = row * stride;
        let end = start + row_bytes;
        if end <= plane.len() {
            data.extend_from_slice(&plane[start..end]);
        } else {
            warn!(
                "Plane extraction: row {} exceeds bounds ({}..{} > {}), bit_depth={}",
                row,
                start,
                end,
                plane.len(),
                bit_depth
            );
        }
    }
    data
}

/// Validates a decoded frame for correctness
pub fn validate_frame(frame: &DecodedFrame) -> Result<()> {
    // Validate dimensions
    if frame.width == 0 || frame.height == 0 {
        error!("Invalid frame dimensions: {}x{}", frame.width, frame.height);
        return Err(DecodeError::UnsupportedFormat);
    }

    // Validate Y plane size
    let expected_y_size = (frame.width * frame.height) as usize;
    if frame.y_plane.len() != expected_y_size {
        error!(
            "Y plane size mismatch: expected {}, got {}",
            expected_y_size,
            frame.y_plane.len()
        );
        return Err(DecodeError::UnsupportedFormat);
    }

    // Validate U/V planes if present
    if let (Some(u_plane), Some(v_plane)) = (&frame.u_plane, &frame.v_plane) {
        // Assume 4:2:0 chroma subsampling
        let expected_uv_size = ((frame.width / 2) * (frame.height / 2)) as usize;

        if u_plane.len() != expected_uv_size {
            warn!(
                "U plane size mismatch: expected {}, got {} (may be 4:2:2 or 4:4:4)",
                expected_uv_size,
                u_plane.len()
            );
        }

        if v_plane.len() != expected_uv_size {
            warn!(
                "V plane size mismatch: expected {}, got {} (may be 4:2:2 or 4:4:4)",
                expected_uv_size,
                v_plane.len()
            );
        }
    }

    // Validate bit depth
    if frame.bit_depth != 8 && frame.bit_depth != 10 && frame.bit_depth != 12 {
        warn!("Unusual bit depth: {}", frame.bit_depth);
    }

    debug!(
        "Frame validation passed: {}x{} {}bit",
        frame.width, frame.height, frame.bit_depth
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_creation() {
        let decoder = Av1Decoder::new();
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_decode_ivf_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test_data/av1_test.ivf");

        if !path.exists() {
            // Skip test if file doesn't exist
            return;
        }

        let data = std::fs::read(&path).unwrap();
        let mut decoder = Av1Decoder::new().unwrap();
        let frames = decoder.decode_all(&data);

        assert!(frames.is_ok(), "Decode failed: {:?}", frames.err());
        let frames = frames.unwrap();
        assert_eq!(frames.len(), 2, "Expected 2 frames");

        // Check first frame dimensions
        let first = &frames[0];
        assert_eq!(first.width, 352);
        assert_eq!(first.height, 288);
    }
}
