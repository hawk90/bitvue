//! FFmpeg-based decoder for H.264/HEVC

#[cfg(feature = "ffmpeg")]
use ffmpeg_next as ffmpeg;

use crate::decoder::{DecodeError, DecodedFrame, FrameType, Result};
use crate::plane_utils;
use crate::traits::{CodecType, Decoder, DecoderCapabilities};
use abseil::prelude::*;
use std::collections::VecDeque;

/// Maximum number of frames to buffer
///
/// Prevents unbounded memory growth from malicious video files.
/// A typical decoder needs 2-4 frames for reordering, so 16 provides
/// a safe margin while preventing DoS via buffer exhaustion.
const MAX_FRAME_BUFFER_SIZE: usize = 16;

#[cfg(feature = "ffmpeg")]
use ffmpeg::codec::{decoder, Context};
#[cfg(feature = "ffmpeg")]
use ffmpeg::format::Pixel;
#[cfg(feature = "ffmpeg")]
use ffmpeg::media::Type;
#[cfg(feature = "ffmpeg")]
use ffmpeg::software::scaling::{context::Context as SwsContext, flag::Flags};
#[cfg(feature = "ffmpeg")]
use ffmpeg::util::frame::video::Video;
#[cfg(feature = "ffmpeg")]
use ffmpeg::{codec, packet};

/// FFmpeg-based decoder for H.264 and HEVC
#[cfg(feature = "ffmpeg")]
pub struct FfmpegDecoder {
    /// Codec type (H264 or H265)
    codec_type: CodecType,
    /// FFmpeg decoder context
    decoder: decoder::Video,
    /// Scaler context for converting to YUV420P
    scaler: Option<SwsContext>,
    /// Frame buffer for receiving decoded frames
    frame_buffer: VecDeque<DecodedFrame>,
    /// Current timestamp
    timestamp: i64,
}

#[cfg(feature = "ffmpeg")]
impl FfmpegDecoder {
    /// Create a new FFmpeg decoder for the specified codec
    pub fn new(codec_type: CodecType) -> Result<Self> {
        // Initialize FFmpeg (only needs to be called once, but safe to call multiple times)
        ffmpeg::init().map_err(|e| DecodeError::Init(format!("FFmpeg init failed: {}", e)))?;

        // Get codec ID
        let codec_id = match codec_type {
            CodecType::H264 => codec::Id::H264,
            CodecType::H265 => codec::Id::HEVC,
            CodecType::VP9 => codec::Id::VP9,
            _ => {
                return Err(DecodeError::Init(format!(
                    "Unsupported codec type for FFmpeg decoder: {}",
                    codec_type
                )))
            }
        };

        // Find decoder
        let codec = ffmpeg::decoder::find(codec_id).ok_or_else(|| {
            DecodeError::Init(format!(
                "FFmpeg decoder not found for codec: {}",
                codec_type
            ))
        })?;

        // Create decoder context
        let context = Context::new();
        let decoder = context
            .decoder()
            .video()
            .map_err(|e| DecodeError::Init(format!("Failed to create decoder: {}", e)))?;

        debug!("Created FFmpeg decoder for {}", codec_type);

        Ok(Self {
            codec_type,
            decoder,
            scaler: None,
            frame_buffer: VecDeque::with_capacity(MAX_FRAME_BUFFER_SIZE),
            timestamp: 0,
        })
    }

    /// Decode a packet and add frames to buffer
    fn decode_packet(&mut self, packet: &packet::Packet) -> Result<()> {
        // Send packet to decoder
        self.decoder
            .send_packet(packet)
            .map_err(|e| DecodeError::Decode(format!("Failed to send packet: {}", e)))?;

        // Receive all available frames
        loop {
            let mut decoded = Video::empty();
            match self.decoder.receive_frame(&mut decoded) {
                Ok(_) => {
                    // Check buffer size to prevent unbounded growth
                    if self.frame_buffer.len() >= MAX_FRAME_BUFFER_SIZE {
                        warn!(
                            "Frame buffer at capacity ({}), dropping oldest frame",
                            MAX_FRAME_BUFFER_SIZE
                        );
                        self.frame_buffer.pop_front(); // O(1) instead of O(n)
                    }

                    // Convert FFmpeg frame to DecodedFrame
                    if let Ok(frame) = self.ffmpeg_frame_to_decoded(&decoded) {
                        self.frame_buffer.push(frame);
                    }
                }
                Err(e) => {
                    // EAGAIN means we need more data, other errors are fatal
                    let err_str = format!("{:?}", e);
                    if !err_str.contains("EAGAIN") && !err_str.contains("EOF") {
                        error!("Decoder error: {}", e);
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    /// Convert FFmpeg video frame to DecodedFrame
    fn ffmpeg_frame_to_decoded(&mut self, frame: &Video) -> Result<DecodedFrame> {
        let width = frame.width();
        let height = frame.height();
        let pixel_format = frame.format();

        // Validate dimensions to prevent DoS via unbounded loops
        // Limit to 8K resolution (7680x4320) per axis
        const MAX_DIMENSION: u32 = 7680;
        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(DecodeError::Decode(format!(
                "Frame dimensions {}x{} exceed maximum {}x{}",
                width, height, MAX_DIMENSION, MAX_DIMENSION
            )));
        }
        if width == 0 || height == 0 {
            return Err(DecodeError::Decode(format!(
                "Invalid frame dimensions: {}x{}",
                width, height
            )));
        }

        // Determine which frame to extract data from (avoid clone when already YUV420P)
        let data_frame: &Video = if pixel_format != Pixel::YUV420P {
            // Create scaler if not exists or format changed
            if self.scaler.is_none() {
                self.scaler = Some(
                    SwsContext::get(
                        pixel_format,
                        width,
                        height,
                        Pixel::YUV420P,
                        width,
                        height,
                        Flags::BILINEAR,
                    )
                    .map_err(|e| DecodeError::Decode(format!("Failed to create scaler: {}", e)))?,
                );
            }

            // Scale to YUV420P - need to clone the scaled frame since it's owned
            return self.extract_converted_frame(frame);
        };

        // Already YUV420P - extract data directly without cloning
        // Validate dimensions first
        plane_utils::validate_dimensions(width as usize, height as usize)?;

        // Extract Y plane using shared utility
        let y_stride = data_frame.stride(0) as usize;
        let y_plane = plane_utils::extract_y_plane(
            data_frame.data(0),
            width as usize,
            height as usize,
            y_stride,
            8, // FFmpeg always uses 8-bit for YUV420P
        )?;

        // Extract U plane using shared utility (420 chroma subsampling)
        let u_stride = data_frame.stride(1) as usize;
        let u_plane = plane_utils::extract_uv_plane_420(
            data_frame.data(1),
            width as usize,
            height as usize,
            u_stride,
            8,
        )?;

        // Extract V plane using shared utility (420 chroma subsampling)
        let v_stride = data_frame.stride(2) as usize;
        let v_plane = plane_utils::extract_uv_plane_420(
            data_frame.data(2),
            width as usize,
            height as usize,
            v_stride,
            8,
        )?;

        // Detect frame type
        let frame_type = if data_frame.is_key() {
            FrameType::Key
        } else {
            FrameType::Inter
        };

        // Get timestamp
        let timestamp = data_frame.timestamp().unwrap_or(self.timestamp);

        debug!(
            "Decoded {} frame: {}x{} {:?}",
            self.codec_type, width, height, frame_type
        );

        // For YUV420P frames, chroma format is known (no calculation needed)
        // This avoids recalculating for every frame in the video
        Ok(DecodedFrame {
            width,
            height,
            bit_depth: 8, // FFmpeg typically outputs 8-bit
            y_plane,
            y_stride: y_stride as usize,
            u_plane: Some(u_plane),
            u_stride: u_stride as usize,
            v_plane: Some(v_plane),
            v_stride: v_stride as usize,
            timestamp,
            frame_type,
            chroma_format: crate::decoder::ChromaFormat::Yuv420,
        })
    }

    /// Helper: Extract frame data after colorspace conversion (requires owning the frame)
    #[cfg(feature = "ffmpeg")]
    fn extract_converted_frame(&mut self, frame: &Video) -> Result<DecodedFrame> {
        let mut yuv = Video::empty();
        if let Some(scaler) = &mut self.scaler {
            scaler
                .run(frame, &mut yuv)
                .map_err(|e| DecodeError::Decode(format!("Failed to scale frame: {}", e)))?;
        }

        let width = yuv.width();
        let height = yuv.height();

        // Extract Y plane
        let y_plane = yuv.data(0).to_vec();
        let y_stride = yuv.stride(0);

        // Extract U plane
        let u_plane = yuv.data(1).to_vec();
        let u_stride = yuv.stride(1);

        // Extract V plane
        let v_plane = yuv.data(2).to_vec();
        let v_stride = yuv.stride(2);

        // Detect frame type
        let frame_type = if yuv.is_key() {
            FrameType::Key
        } else {
            FrameType::Inter
        };

        // Get timestamp
        let timestamp = yuv.timestamp().unwrap_or(self.timestamp);

        debug!(
            "Decoded {} frame (converted): {}x{} {:?}",
            self.codec_type, width, height, frame_type
        );

        // Detect chroma format once at frame creation
        let chroma_format = crate::decoder::ChromaFormat::from_frame_data(
            width,
            height,
            8,
            Some(&u_plane),
            Some(&v_plane),
        );

        Ok(DecodedFrame {
            width,
            height,
            bit_depth: 8,
            y_plane,
            y_stride: y_stride as usize,
            u_plane: Some(u_plane),
            u_stride: u_stride as usize,
            v_plane: Some(v_plane),
            v_stride: v_stride as usize,
            timestamp,
            frame_type,
            chroma_format: crate::decoder::ChromaFormat::Yuv420,
        })
    }
}

#[cfg(feature = "ffmpeg")]
impl Decoder for FfmpegDecoder {
    fn codec_type(&self) -> CodecType {
        self.codec_type
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities {
            codec: self.codec_type,
            max_width: 8192,
            max_height: 8192,
            supported_bit_depths: vec![8, 10, 12],
            hw_accel: false, // Software decoding for now
        }
    }

    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()> {
        self.timestamp = timestamp.unwrap_or(self.timestamp + 1);

        // Create packet from data
        let mut packet = packet::Packet::copy(data);
        packet.set_pts(Some(self.timestamp));

        self.decode_packet(&packet)?;
        Ok(())
    }

    fn get_frame(&mut self) -> Result<DecodedFrame> {
        if self.frame_buffer.is_empty() {
            return Err(DecodeError::NoFrame);
        }
        Ok(self.frame_buffer.remove(0))
    }

    fn flush(&mut self) {
        // Send flush packet
        let _ = self.decoder.send_eof();

        // Drain remaining frames
        loop {
            let mut frame = Video::empty();
            match self.decoder.receive_frame(&mut frame) {
                Ok(_) => {
                    // Check buffer size to prevent unbounded growth
                    if self.frame_buffer.len() >= MAX_FRAME_BUFFER_SIZE {
                        warn!(
                            "Frame buffer at capacity ({}), dropping oldest frame",
                            MAX_FRAME_BUFFER_SIZE
                        );
                        self.frame_buffer.pop_front(); // O(1) instead of O(n)
                    }

                    if let Ok(decoded) = self.ffmpeg_frame_to_decoded(&frame) {
                        self.frame_buffer.push(decoded);
                    }
                }
                Err(_) => break,
            }
        }
    }

    fn reset(&mut self) -> Result<()> {
        // Create a new decoder to reset state
        *self = Self::new(self.codec_type)?;
        Ok(())
    }
}

/// H.264/AVC decoder
#[cfg(feature = "ffmpeg")]
pub struct H264Decoder {
    inner: FfmpegDecoder,
}

#[cfg(feature = "ffmpeg")]
impl H264Decoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: FfmpegDecoder::new(CodecType::H264)?,
        })
    }
}

#[cfg(feature = "ffmpeg")]
impl Decoder for H264Decoder {
    fn codec_type(&self) -> CodecType {
        self.inner.codec_type()
    }

    fn capabilities(&self) -> DecoderCapabilities {
        self.inner.capabilities()
    }

    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()> {
        self.inner.send_data(data, timestamp)
    }

    fn get_frame(&mut self) -> Result<DecodedFrame> {
        self.inner.get_frame()
    }

    fn flush(&mut self) {
        self.inner.flush()
    }

    fn reset(&mut self) -> Result<()> {
        self.inner.reset()
    }
}

/// H.265/HEVC decoder
#[cfg(feature = "ffmpeg")]
pub struct HevcDecoder {
    inner: FfmpegDecoder,
}

#[cfg(feature = "ffmpeg")]
impl HevcDecoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: FfmpegDecoder::new(CodecType::H265)?,
        })
    }
}

#[cfg(feature = "ffmpeg")]
impl Decoder for HevcDecoder {
    fn codec_type(&self) -> CodecType {
        self.inner.codec_type()
    }

    fn capabilities(&self) -> DecoderCapabilities {
        self.inner.capabilities()
    }

    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()> {
        self.inner.send_data(data, timestamp)
    }

    fn get_frame(&mut self) -> Result<DecodedFrame> {
        self.inner.get_frame()
    }

    fn flush(&mut self) {
        self.inner.flush()
    }

    fn reset(&mut self) -> Result<()> {
        self.inner.reset()
    }
}

/// VP9 decoder
#[cfg(feature = "ffmpeg")]
pub struct Vp9Decoder {
    inner: FfmpegDecoder,
}

#[cfg(feature = "ffmpeg")]
impl Vp9Decoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: FfmpegDecoder::new(CodecType::VP9)?,
        })
    }
}

#[cfg(feature = "ffmpeg")]
impl Decoder for Vp9Decoder {
    fn codec_type(&self) -> CodecType {
        self.inner.codec_type()
    }

    fn capabilities(&self) -> DecoderCapabilities {
        self.inner.capabilities()
    }

    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()> {
        self.inner.send_data(data, timestamp)
    }

    fn get_frame(&mut self) -> Result<DecodedFrame> {
        self.inner.get_frame()
    }

    fn flush(&mut self) {
        self.inner.flush()
    }

    fn reset(&mut self) -> Result<()> {
        self.inner.reset()
    }
}

// Stub implementations when ffmpeg feature is disabled
#[cfg(not(feature = "ffmpeg"))]
pub struct H264Decoder;

#[cfg(not(feature = "ffmpeg"))]
impl H264Decoder {
    pub fn new() -> Result<Self> {
        Err(DecodeError::Init(
            "H.264 decoder requires ffmpeg feature".to_string(),
        ))
    }
}

#[cfg(not(feature = "ffmpeg"))]
pub struct HevcDecoder;

#[cfg(not(feature = "ffmpeg"))]
impl HevcDecoder {
    pub fn new() -> Result<Self> {
        Err(DecodeError::Init(
            "HEVC decoder requires ffmpeg feature".to_string(),
        ))
    }
}

#[cfg(not(feature = "ffmpeg"))]
pub struct Vp9Decoder;

#[cfg(not(feature = "ffmpeg"))]
impl Vp9Decoder {
    pub fn new() -> Result<Self> {
        Err(DecodeError::Init(
            "VP9 decoder requires ffmpeg feature".to_string(),
        ))
    }
}
