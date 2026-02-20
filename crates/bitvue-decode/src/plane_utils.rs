//! Shared utilities for video plane extraction and manipulation
//!
//! This module provides common functionality for extracting Y, U, V planes
//! from various video decoder backends with consistent overflow protection
//! and bounds checking.

use crate::decoder::DecodeError;

/// Maximum allowed plane size (8K resolution)
pub const MAX_PLANE_SIZE: usize = 7680 * 4320;

/// Result type for plane operations
type Result<T> = std::result::Result<T, DecodeError>;

/// Configuration for plane extraction
#[derive(Debug, Clone, Copy)]
pub struct PlaneConfig {
    /// Width of the plane in pixels
    pub width: usize,
    /// Height of the plane in pixels
    pub height: usize,
    /// Stride (bytes per row) in the source data
    pub stride: usize,
    /// Bit depth (8, 10, or 12)
    pub bit_depth: u8,
}

impl PlaneConfig {
    /// Create a new plane configuration with validation
    pub fn new(width: usize, height: usize, stride: usize, bit_depth: u8) -> Result<Self> {
        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(DecodeError::Decode(format!(
                "Invalid plane dimensions: {}x{}",
                width, height
            )));
        }

        // Validate bit depth
        if !matches!(bit_depth, 8 | 10 | 12) {
            return Err(DecodeError::Decode(format!(
                "Unsupported bit depth: {}",
                bit_depth
            )));
        }

        // Validate stride is sufficient
        let bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };
        let min_stride = width * bytes_per_sample;
        if stride < min_stride {
            return Err(DecodeError::Decode(format!(
                "Stride {} too small for width {} (min {})",
                stride, width, min_stride
            )));
        }

        Ok(Self {
            width,
            height,
            stride,
            bit_depth,
        })
    }

    /// Get bytes per sample based on bit depth
    #[inline]
    pub fn bytes_per_sample(&self) -> usize {
        if self.bit_depth > 8 {
            2
        } else {
            1
        }
    }

    /// Get row bytes (width * bytes_per_sample)
    #[inline]
    pub fn row_bytes(&self) -> usize {
        self.width * self.bytes_per_sample()
    }

    /// Get expected plane size (row_bytes * height)
    pub fn expected_size(&self) -> Result<usize> {
        self.row_bytes().checked_mul(self.height).ok_or_else(|| {
            DecodeError::Decode(format!(
                "Plane size calculation overflow: {}x{} bit_depth={}",
                self.width, self.height, self.bit_depth
            ))
        })
    }

    /// Check if data is contiguous (stride == row_bytes)
    #[inline]
    pub fn is_contiguous(&self) -> bool {
        self.stride == self.row_bytes()
    }

    /// Validate plane size against maximum
    pub fn validate_size(&self) -> Result<()> {
        let size = self.expected_size()?;
        if size > MAX_PLANE_SIZE {
            return Err(DecodeError::Decode(format!(
                "Plane size {} exceeds maximum {}",
                size, MAX_PLANE_SIZE
            )));
        }
        Ok(())
    }
}

/// Extract contiguous plane data (fast path)
#[inline]
fn extract_contiguous(source: &[u8], expected_size: usize, bit_depth: u8) -> Result<Vec<u8>> {
    if expected_size <= source.len() {
        Ok(source[..expected_size].to_vec())
    } else {
        Err(DecodeError::Decode(format!(
            "Contiguous plane data exceeds bounds ({} > {}), bit_depth={}",
            expected_size,
            source.len(),
            bit_depth
        )))
    }
}

/// Extract plane data into existing buffer (zero-allocation)
///
/// This is a performance optimization that reuses an existing buffer
/// instead of allocating a new one. Useful for hot paths.
///
/// # Arguments
///
/// * `source` - Source slice containing plane data
/// * `config` - Plane configuration
/// * `dest` - Destination buffer (must be large enough)
///
/// # Returns
///
/// The number of bytes written to dest.
///
/// # Errors
///
/// Returns an error if:
/// - Destination buffer is too small
/// - Source data is insufficient
/// - Plane dimensions cause overflow
pub fn extract_plane_into(source: &[u8], config: PlaneConfig, dest: &mut [u8]) -> Result<usize> {
    // Validate configuration
    config.validate_size()?;

    let expected_size = config.expected_size()?;
    let row_bytes = config.row_bytes();

    // Validate destination buffer size
    if dest.len() < expected_size {
        return Err(DecodeError::Decode(format!(
            "Destination buffer too small: {} < {}",
            dest.len(),
            expected_size
        )));
    }

    // Fast path: contiguous data - single copy
    if config.is_contiguous() {
        if expected_size > source.len() {
            return Err(DecodeError::Decode(format!(
                "Contiguous plane data exceeds bounds ({} > {}), bit_depth={}",
                expected_size,
                source.len(),
                config.bit_depth
            )));
        }
        dest[..expected_size].copy_from_slice(&source[..expected_size]);
        return Ok(expected_size);
    }

    // Slow path: strided data - copy row by row
    let mut offset = 0;
    for row in 0..config.height {
        let start = row
            .checked_mul(config.stride)
            .ok_or_else(|| DecodeError::Decode(format!("Row offset overflow at row {}", row)))?;

        let end = start.checked_add(row_bytes).ok_or_else(|| {
            DecodeError::Decode(format!("Row end offset overflow at row {}", row))
        })?;

        if end > source.len() {
            return Err(DecodeError::Decode(format!(
                "Plane access out of bounds: row={}, end={}, source_len={}",
                row,
                end,
                source.len()
            )));
        }

        dest[offset..offset + row_bytes].copy_from_slice(&source[start..end]);
        offset += row_bytes;
    }

    Ok(expected_size)
}

/// Extract plane data from a slice with stride handling
///
/// Optimized for contiguous data (stride == row_bytes) with single-copy fast path.
/// Falls back to row-by-row copying for strided data.
///
/// # Arguments
///
/// * `source` - Source slice containing plane data
/// * `config` - Plane configuration (width, height, stride, bit depth)
///
/// # Returns
///
/// A vector containing the extracted plane data without stride padding.
///
/// # Errors
///
/// Returns an error if:
/// - Plane dimensions cause integer overflow
/// - Source data is insufficient for the requested dimensions
/// - Plane size exceeds maximum allowed
pub fn extract_plane(source: &[u8], config: PlaneConfig) -> Result<Vec<u8>> {
    // Validate configuration
    config.validate_size()?;

    let expected_size = config.expected_size()?;
    let row_bytes = config.row_bytes();

    // Fast path: contiguous data (stride == row_bytes) - single copy
    if config.is_contiguous() {
        return extract_contiguous(source, expected_size, config.bit_depth);
    }

    // Slow path: strided data - copy row by row with pre-allocated buffer
    // Pre-allocate full buffer without zero-initialization for better performance
    //
    // SAFETY: The loop below writes to every position in `data` before returning:
    // - We iterate over exactly `config.height` rows
    // - Each row writes exactly `row_bytes` bytes at `data[offset..offset + row_bytes]`
    // - Total bytes written: config.height * row_bytes = expected_size
    // - The final `offset` equals `expected_size`, confirming all bytes were written
    // - We validate source bounds before each copy (lines 262-271)
    // - We return early on any error, so partial writes never reach the caller
    #[expect(clippy::uninit_vec)]
    let mut data = Vec::with_capacity(expected_size);
    unsafe {
        data.set_len(expected_size);
    }
    let mut offset = 0;

    for row in 0..config.height {
        // Calculate row offset with overflow check
        let start = row.checked_mul(config.stride).ok_or_else(|| {
            DecodeError::Decode(format!(
                "Row offset overflow at row {}: {} * {}",
                row, row, config.stride
            ))
        })?;

        // Calculate row end with overflow check
        let end = start.checked_add(row_bytes).ok_or_else(|| {
            DecodeError::Decode(format!(
                "Row end offset overflow at row {}: {} + {}",
                row, start, row_bytes
            ))
        })?;

        // Validate bounds before accessing memory
        if end > source.len() {
            return Err(DecodeError::Decode(format!(
                "Plane access out of bounds: row={}, start={}, end={}, source_len={}, bit_depth={}",
                row,
                start,
                end,
                source.len(),
                config.bit_depth
            )));
        }

        // Use copy_from_slice instead of extend_from_slice to avoid capacity checks
        data[offset..offset + row_bytes].copy_from_slice(&source[start..end]);
        offset += row_bytes;
    }

    Ok(data)
}

/// Extract Y plane (luminance) from source data
///
/// Convenience wrapper around `extract_plane` for Y plane extraction.
#[inline]
pub fn extract_y_plane(
    source: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bit_depth: u8,
) -> Result<Vec<u8>> {
    let config = PlaneConfig::new(width, height, stride, bit_depth)?;
    extract_plane(source, config)
}

/// Extract U or V plane (chroma) from source data for YUV420
///
/// Convenience wrapper around `extract_plane` for chroma plane extraction.
/// Automatically halves width and height for 4:2:0 subsampling.
#[inline]
pub fn extract_uv_plane_420(
    source: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bit_depth: u8,
) -> Result<Vec<u8>> {
    let config = PlaneConfig::new(width / 2, height / 2, stride, bit_depth)?;
    extract_plane(source, config)
}

/// Validate plane dimensions to prevent unbounded loops
///
/// Checks that dimensions don't exceed reasonable limits (8K resolution).
pub fn validate_dimensions(width: usize, height: usize) -> Result<()> {
    const MAX_DIMENSION: usize = 7680; // 8K horizontal

    if width > MAX_DIMENSION {
        return Err(DecodeError::Decode(format!(
            "Width {} exceeds maximum {}",
            width, MAX_DIMENSION
        )));
    }

    if height > MAX_DIMENSION {
        return Err(DecodeError::Decode(format!(
            "Height {} exceeds maximum {}",
            height, MAX_DIMENSION
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane_config_valid() {
        let config = PlaneConfig::new(1920, 1080, 1920, 8).unwrap();
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.bytes_per_sample(), 1);
        assert_eq!(config.row_bytes(), 1920);
        assert!(config.is_contiguous());
    }

    #[test]
    fn test_plane_config_strided() {
        let config = PlaneConfig::new(1920, 1080, 2048, 8).unwrap();
        assert_eq!(config.stride, 2048);
        assert!(!config.is_contiguous());
    }

    #[test]
    fn test_plane_config_10bit() {
        let config = PlaneConfig::new(1920, 1080, 3840, 10).unwrap();
        assert_eq!(config.bytes_per_sample(), 2);
        assert_eq!(config.row_bytes(), 3840);
        assert!(config.is_contiguous());
    }

    #[test]
    fn test_plane_config_invalid_dimensions() {
        assert!(PlaneConfig::new(0, 1080, 1920, 8).is_err());
        assert!(PlaneConfig::new(1920, 0, 1920, 8).is_err());
    }

    #[test]
    fn test_plane_config_invalid_bit_depth() {
        assert!(PlaneConfig::new(1920, 1080, 1920, 16).is_err());
    }

    #[test]
    fn test_plane_config_insufficient_stride() {
        assert!(PlaneConfig::new(1920, 1080, 1000, 8).is_err());
    }

    #[test]
    fn test_extract_plane_contiguous() {
        let data = vec![42u8; 1920 * 1080];
        let config = PlaneConfig::new(1920, 1080, 1920, 8).unwrap();
        let result = extract_plane(&data, config).unwrap();
        assert_eq!(result.len(), 1920 * 1080);
        assert_eq!(result[0], 42);
    }

    #[test]
    fn test_extract_plane_strided() {
        // Create strided data: 4x4 with stride 8
        let mut data = vec![0u8; 8 * 4];
        for row in 0..4 {
            for col in 0..4 {
                data[row * 8 + col] = (row * 4 + col) as u8;
            }
        }

        let config = PlaneConfig::new(4, 4, 8, 8).unwrap();
        let result = extract_plane(&data, config).unwrap();

        assert_eq!(result.len(), 16);
        assert_eq!(result[0], 0); // Row 0, col 0
        assert_eq!(result[4], 4); // Row 1, col 0
        assert_eq!(result[8], 8); // Row 2, col 0
        assert_eq!(result[12], 12); // Row 3, col 0
    }

    #[test]
    fn test_extract_plane_bounds_error() {
        let data = vec![42u8; 100]; // Too small
        let config = PlaneConfig::new(1920, 1080, 1920, 8).unwrap();
        assert!(extract_plane(&data, config).is_err());
    }

    #[test]
    fn test_extract_uv_plane_420() {
        let data = vec![42u8; 960 * 540]; // Half size
        let result = extract_uv_plane_420(&data, 1920, 1080, 960, 8).unwrap();
        assert_eq!(result.len(), 960 * 540);
    }

    #[test]
    fn test_validate_dimensions_ok() {
        assert!(validate_dimensions(1920, 1080).is_ok());
        assert!(validate_dimensions(3840, 2160).is_ok());
        assert!(validate_dimensions(7680, 4320).is_ok());
    }

    #[test]
    fn test_validate_dimensions_exceeds() {
        assert!(validate_dimensions(8000, 1080).is_err());
        assert!(validate_dimensions(1920, 8000).is_err());
    }

    #[test]
    fn test_extract_plane_into_contiguous() {
        let source = vec![42u8; 1920 * 1080];
        let config = PlaneConfig::new(1920, 1080, 1920, 8).unwrap();
        let mut dest = vec![0u8; 1920 * 1080];

        let written = extract_plane_into(&source, config, &mut dest).unwrap();

        assert_eq!(written, 1920 * 1080);
        assert_eq!(dest[0], 42);
        assert_eq!(dest[100], 42);
    }

    #[test]
    fn test_extract_plane_into_strided() {
        // Create strided data: 4x4 with stride 8
        let mut source = vec![0u8; 8 * 4];
        for row in 0..4 {
            for col in 0..4 {
                source[row * 8 + col] = (row * 4 + col) as u8;
            }
        }

        let config = PlaneConfig::new(4, 4, 8, 8).unwrap();
        let mut dest = vec![0u8; 16];

        let written = extract_plane_into(&source, config, &mut dest).unwrap();

        assert_eq!(written, 16);
        assert_eq!(dest[0], 0);
        assert_eq!(dest[4], 4);
        assert_eq!(dest[8], 8);
    }

    #[test]
    fn test_extract_plane_into_buffer_too_small() {
        let source = vec![42u8; 1920 * 1080];
        let config = PlaneConfig::new(1920, 1080, 1920, 8).unwrap();
        let mut dest = vec![0u8; 100]; // Too small

        assert!(extract_plane_into(&source, config, &mut dest).is_err());
    }
}
