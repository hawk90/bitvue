//! Raw YUV file loader
//!
//! Loads raw .yuv and .y4m files for reference comparison

use crate::decoder::DecodedFrame;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum YuvLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid YUV file format")]
    InvalidFormat,
    #[error("Invalid Y4M header: {0}")]
    InvalidY4mHeader(String),
    #[error("Frame size mismatch: expected {expected}, got {actual}")]
    FrameSizeMismatch { expected: usize, actual: usize },
    #[error("Unsupported chroma subsampling: {0}")]
    UnsupportedChromaSubsampling(String),
}

pub type Result<T> = std::result::Result<T, YuvLoaderError>;

/// Chroma subsampling format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaSubsampling {
    /// 4:2:0 - U and V are 1/2 resolution in both dimensions
    Yuv420,
    /// 4:2:2 - U and V are 1/2 resolution horizontally only
    Yuv422,
    /// 4:4:4 - U and V are full resolution
    Yuv444,
    /// Monochrome - Y only
    Mono,
}

/// Bit depth
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    /// 8-bit
    Bit8,
    /// 10-bit
    Bit10,
    /// 12-bit
    Bit12,
}

impl BitDepth {
    pub fn bits(&self) -> u8 {
        match self {
            BitDepth::Bit8 => 8,
            BitDepth::Bit10 => 10,
            BitDepth::Bit12 => 12,
        }
    }

    pub fn bytes_per_sample(&self) -> usize {
        match self {
            BitDepth::Bit8 => 1,
            BitDepth::Bit10 | BitDepth::Bit12 => 2,
        }
    }
}

/// YUV file parameters
#[derive(Debug, Clone)]
pub struct YuvFileParams {
    pub width: u32,
    pub height: u32,
    pub chroma_subsampling: ChromaSubsampling,
    pub bit_depth: BitDepth,
    pub frame_rate: (u32, u32), // (numerator, denominator)
}

impl YuvFileParams {
    /// Calculate frame size in bytes
    pub fn frame_size_bytes(&self) -> usize {
        let width = self.width as usize;
        let height = self.height as usize;
        let bytes_per_sample = self.bit_depth.bytes_per_sample();

        let y_size = width * height * bytes_per_sample;

        let uv_size = match self.chroma_subsampling {
            ChromaSubsampling::Yuv420 => (width / 2) * (height / 2) * bytes_per_sample,
            ChromaSubsampling::Yuv422 => (width / 2) * height * bytes_per_sample,
            ChromaSubsampling::Yuv444 => width * height * bytes_per_sample,
            ChromaSubsampling::Mono => 0,
        };

        y_size + 2 * uv_size
    }

    /// Calculate Y plane size in bytes
    pub fn y_plane_size(&self) -> usize {
        (self.width as usize) * (self.height as usize) * self.bit_depth.bytes_per_sample()
    }

    /// Calculate UV plane size in bytes
    pub fn uv_plane_size(&self) -> usize {
        let width = self.width as usize;
        let height = self.height as usize;
        let bytes_per_sample = self.bit_depth.bytes_per_sample();

        match self.chroma_subsampling {
            ChromaSubsampling::Yuv420 => (width / 2) * (height / 2) * bytes_per_sample,
            ChromaSubsampling::Yuv422 => (width / 2) * height * bytes_per_sample,
            ChromaSubsampling::Yuv444 => width * height * bytes_per_sample,
            ChromaSubsampling::Mono => 0,
        }
    }
}

/// YUV file loader
pub struct YuvLoader {
    params: YuvFileParams,
    file: BufReader<File>,
    is_y4m: bool,
    current_frame: usize,
}

impl YuvLoader {
    /// Open a YUV file (detects .y4m or raw .yuv)
    pub fn open<P: AsRef<Path>>(
        path: P,
        params: Option<YuvFileParams>,
    ) -> Result<Self> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)?;
        let mut reader = BufReader::new(file);

        // Check if it's a Y4M file
        let is_y4m = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("y4m"))
            .unwrap_or(false);

        let params = if is_y4m {
            // Parse Y4M header
            Self::parse_y4m_header(&mut reader)?
        } else {
            // Raw YUV - params must be provided
            params.ok_or(YuvLoaderError::InvalidFormat)?
        };

        Ok(Self {
            params,
            file: reader,
            is_y4m,
            current_frame: 0,
        })
    }

    /// Parse Y4M file header
    fn parse_y4m_header(reader: &mut BufReader<File>) -> Result<YuvFileParams> {
        let mut header_line = String::new();
        reader.read_line(&mut header_line)?;

        if !header_line.starts_with("YUV4MPEG2 ") {
            return Err(YuvLoaderError::InvalidY4mHeader(
                "Missing YUV4MPEG2 signature".to_string(),
            ));
        }

        let mut width = 0;
        let mut height = 0;
        let mut frame_rate = (25, 1); // default 25fps
        let mut chroma_subsampling = ChromaSubsampling::Yuv420;
        let mut bit_depth = BitDepth::Bit8;

        // Parse header parameters
        for part in header_line.split_whitespace().skip(1) {
            if let Some(value) = part.strip_prefix('W') {
                width = value.parse().map_err(|_| {
                    YuvLoaderError::InvalidY4mHeader(format!("Invalid width: {}", value))
                })?;
            } else if let Some(value) = part.strip_prefix('H') {
                height = value.parse().map_err(|_| {
                    YuvLoaderError::InvalidY4mHeader(format!("Invalid height: {}", value))
                })?;
            } else if let Some(value) = part.strip_prefix('F') {
                // Frame rate (e.g., "F25:1")
                let parts: Vec<&str> = value.split(':').collect();
                if parts.len() == 2 {
                    frame_rate = (
                        parts[0].parse().unwrap_or(25),
                        parts[1].parse().unwrap_or(1),
                    );
                }
            } else if let Some(value) = part.strip_prefix('C') {
                // Chroma subsampling (e.g., "C420", "C422", "C444")
                chroma_subsampling = match value {
                    "420" | "420jpeg" | "420mpeg2" | "420paldv" => ChromaSubsampling::Yuv420,
                    "422" => ChromaSubsampling::Yuv422,
                    "444" => ChromaSubsampling::Yuv444,
                    "mono" => ChromaSubsampling::Mono,
                    "420p10" => {
                        bit_depth = BitDepth::Bit10;
                        ChromaSubsampling::Yuv420
                    }
                    "420p12" => {
                        bit_depth = BitDepth::Bit12;
                        ChromaSubsampling::Yuv420
                    }
                    _ => {
                        return Err(YuvLoaderError::UnsupportedChromaSubsampling(
                            value.to_string(),
                        ))
                    }
                };
            }
        }

        if width == 0 || height == 0 {
            return Err(YuvLoaderError::InvalidY4mHeader(
                "Missing width or height".to_string(),
            ));
        }

        Ok(YuvFileParams {
            width,
            height,
            chroma_subsampling,
            bit_depth,
            frame_rate,
        })
    }

    /// Read next frame
    pub fn read_frame(&mut self) -> Result<Option<DecodedFrame>> {
        if self.is_y4m {
            // Skip Y4M frame header ("FRAME\n")
            let mut frame_header = String::new();
            let bytes_read = self.file.read_line(&mut frame_header)?;
            if bytes_read == 0 {
                return Ok(None); // EOF
            }
            if !frame_header.starts_with("FRAME") {
                return Err(YuvLoaderError::InvalidFormat);
            }
        }

        // Read Y plane
        let y_size = self.params.y_plane_size();
        let mut y_plane = vec![0u8; y_size];
        let bytes_read = self.file.read(&mut y_plane)?;
        if bytes_read == 0 {
            return Ok(None); // EOF
        }
        if bytes_read != y_size {
            return Err(YuvLoaderError::FrameSizeMismatch {
                expected: y_size,
                actual: bytes_read,
            });
        }

        // Read U and V planes
        let uv_size = self.params.uv_plane_size();
        let (u_plane, v_plane) = if uv_size > 0 {
            let mut u_plane = vec![0u8; uv_size];
            let mut v_plane = vec![0u8; uv_size];

            self.file.read_exact(&mut u_plane)?;
            self.file.read_exact(&mut v_plane)?;

            (Some(u_plane), Some(v_plane))
        } else {
            (None, None)
        };

        let frame = DecodedFrame {
            width: self.params.width,
            height: self.params.height,
            bit_depth: self.params.bit_depth.bits(),
            y_plane,
            y_stride: self.params.width as usize,
            u_plane,
            u_stride: match self.params.chroma_subsampling {
                ChromaSubsampling::Yuv420 | ChromaSubsampling::Yuv422 => (self.params.width / 2) as usize,
                ChromaSubsampling::Yuv444 => self.params.width as usize,
                ChromaSubsampling::Mono => 0,
            },
            v_plane,
            v_stride: match self.params.chroma_subsampling {
                ChromaSubsampling::Yuv420 | ChromaSubsampling::Yuv422 => (self.params.width / 2) as usize,
                ChromaSubsampling::Yuv444 => self.params.width as usize,
                ChromaSubsampling::Mono => 0,
            },
            timestamp: self.current_frame as i64,
            frame_type: crate::decoder::FrameType::Key, // Unknown for raw YUV
            qp_avg: None,
        };

        self.current_frame += 1;
        Ok(Some(frame))
    }

    /// Get file parameters
    pub fn params(&self) -> &YuvFileParams {
        &self.params
    }

    /// Get current frame index
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }

    /// Seek to a specific frame (by skipping)
    pub fn seek_to_frame(&mut self, frame_index: usize) -> Result<()> {
        // Simple implementation: restart and skip frames
        // For a production implementation, you'd want to use File::seek()
        if frame_index < self.current_frame {
            // Need to restart
            return Err(YuvLoaderError::InvalidFormat); // TODO: implement seek backward
        }

        while self.current_frame < frame_index {
            if self.read_frame()?.is_none() {
                return Err(YuvLoaderError::InvalidFormat); // EOF before target frame
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_size_calculation() {
        let params = YuvFileParams {
            width: 1920,
            height: 1080,
            chroma_subsampling: ChromaSubsampling::Yuv420,
            bit_depth: BitDepth::Bit8,
            frame_rate: (30, 1),
        };

        // Y = 1920 * 1080 = 2,073,600
        // U = V = (1920/2) * (1080/2) = 960 * 540 = 518,400
        // Total = 2,073,600 + 2 * 518,400 = 3,110,400
        assert_eq!(params.frame_size_bytes(), 3_110_400);
    }

    #[test]
    fn test_frame_size_10bit() {
        let params = YuvFileParams {
            width: 1920,
            height: 1080,
            chroma_subsampling: ChromaSubsampling::Yuv420,
            bit_depth: BitDepth::Bit10,
            frame_rate: (30, 1),
        };

        // 10-bit uses 2 bytes per sample
        // Y = 1920 * 1080 * 2 = 4,147,200
        // U = V = 960 * 540 * 2 = 1,036,800
        // Total = 4,147,200 + 2 * 1,036,800 = 6,220,800
        assert_eq!(params.frame_size_bytes(), 6_220_800);
    }
}
