//! VP9 Superframe parsing.
//!
//! VP9 superframes allow multiple frames to be packed into a single container frame.
//! This is commonly used for frame dependencies (e.g., hidden alternate reference frames).
//!
//! Superframe format:
//! - Multiple VP9 frames concatenated
//! - Superframe index at the end (if multiple frames)
//!
//! Index format (if present):
//! - [marker byte][frame_sizes...][marker byte]
//! - Marker: 110XXXXX where XXX = size_bytes-1, XXXXX[4:3] = frame_count-1

use crate::error::{Result, Vp9Error};
use serde::{Deserialize, Serialize};

/// VP9 Superframe index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperframeIndex {
    /// Number of frames in the superframe.
    pub frame_count: u8,
    /// Size of each frame in bytes.
    pub frame_sizes: Vec<u32>,
    /// Byte offset of each frame within the superframe.
    pub frame_offsets: Vec<u32>,
}

impl SuperframeIndex {
    /// Check if this is a valid superframe (more than one frame).
    pub fn is_superframe(&self) -> bool {
        self.frame_count > 1
    }

    /// Get the total size of all frames.
    pub fn total_frame_size(&self) -> u32 {
        self.frame_sizes.iter().sum()
    }
}

/// Check if data contains a superframe index.
pub fn has_superframe_index(data: &[u8]) -> bool {
    if data.len() < 2 {
        return false;
    }

    // Check for superframe marker at the end
    let marker = data[data.len() - 1];

    // Marker format: 110XXXXX
    // Top 3 bits must be 110 (0xC0 masked with 0xE0)
    if (marker & 0xE0) != 0xC0 {
        return false;
    }

    // Extract fields from marker
    let size_bytes = ((marker >> 3) & 0x03) + 1;
    let frame_count = (marker & 0x07) + 1;

    // Index size: marker + (frame_count * size_bytes) + marker
    let index_size = 2 + (frame_count as usize * size_bytes as usize);

    if data.len() < index_size {
        return false;
    }

    // Verify the first marker matches
    let first_marker = data[data.len() - index_size];
    first_marker == marker
}

/// Parse superframe index from data.
pub fn parse_superframe_index(data: &[u8]) -> Result<SuperframeIndex> {
    if !has_superframe_index(data) {
        // Single frame, no superframe index
        return Ok(SuperframeIndex {
            frame_count: 1,
            frame_sizes: vec![data.len() as u32],
            frame_offsets: vec![0],
        });
    }

    let marker = data[data.len() - 1];

    // Extract fields from marker
    let size_bytes = ((marker >> 3) & 0x03) + 1;
    let frame_count = (marker & 0x07) + 1;

    // Index size: marker + (frame_count * size_bytes) + marker
    let index_size = 2 + (frame_count as usize * size_bytes as usize);

    // Parse frame sizes
    let mut frame_sizes = Vec::with_capacity(frame_count as usize);
    let index_start = data.len() - index_size + 1; // Skip first marker

    for i in 0..frame_count as usize {
        let mut size: u32 = 0;
        for j in 0..size_bytes as usize {
            size |= (data[index_start + i * size_bytes as usize + j] as u32) << (j * 8);
        }
        frame_sizes.push(size);
    }

    // Calculate offsets
    let mut frame_offsets = Vec::with_capacity(frame_count as usize);
    let mut offset: u32 = 0;
    for size in &frame_sizes {
        frame_offsets.push(offset);
        offset += size;
    }

    Ok(SuperframeIndex {
        frame_count,
        frame_sizes,
        frame_offsets,
    })
}

/// Extract individual frames from a superframe.
pub fn extract_frames(data: &[u8]) -> Result<Vec<&[u8]>> {
    let index = parse_superframe_index(data)?;

    if index.frame_count == 1 && !has_superframe_index(data) {
        // Single frame, return as-is
        return Ok(vec![data]);
    }

    let mut frames = Vec::with_capacity(index.frame_count as usize);

    for i in 0..index.frame_count as usize {
        let start = index.frame_offsets[i] as usize;
        let end = start + index.frame_sizes[i] as usize;

        if end > data.len() {
            return Err(Vp9Error::InvalidData(format!(
                "Frame {} extends beyond data: end={}, len={}",
                i,
                end,
                data.len()
            )));
        }

        frames.push(&data[start..end]);
    }

    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_frame() {
        let data = vec![0x82, 0x49, 0x83, 0x42]; // Just some VP9-like data
        let index = parse_superframe_index(&data).unwrap();

        assert_eq!(index.frame_count, 1);
        assert_eq!(index.frame_sizes, vec![4]);
        assert_eq!(index.frame_offsets, vec![0]);
        assert!(!index.is_superframe());
    }

    #[test]
    fn test_superframe_detection() {
        // Not a superframe - wrong marker
        let data = vec![0x00, 0x01, 0x02, 0x03];
        assert!(!has_superframe_index(&data));

        // Superframe with 2 frames, 1-byte sizes
        // Marker: 0xC1 = 110 00 001 (1 byte sizes, 2 frames)
        // Index: [marker][size1][size2][marker]
        let mut data = vec![0; 10]; // 5 + 3 bytes for two frames
        data.extend_from_slice(&[0xC1, 5, 3, 0xC1]); // Index at end

        assert!(has_superframe_index(&data));
    }

    #[test]
    fn test_parse_superframe() {
        // Create a superframe with 2 frames of 5 and 3 bytes
        let mut data = vec![0xAA; 5]; // Frame 1
        data.extend_from_slice(&[0xBB; 3]); // Frame 2

        // Add superframe index: 2 frames, 1-byte sizes
        // Marker: 0xC1 = 110 00 001
        data.extend_from_slice(&[0xC1, 5, 3, 0xC1]);

        let index = parse_superframe_index(&data).unwrap();

        assert_eq!(index.frame_count, 2);
        assert_eq!(index.frame_sizes, vec![5, 3]);
        assert_eq!(index.frame_offsets, vec![0, 5]);
        assert!(index.is_superframe());
    }

    #[test]
    fn test_extract_frames() {
        let mut data = vec![0xAA; 5];
        data.extend_from_slice(&[0xBB; 3]);
        data.extend_from_slice(&[0xC1, 5, 3, 0xC1]);

        let frames = extract_frames(&data).unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], &[0xAA; 5]);
        assert_eq!(frames[1], &[0xBB; 3]);
    }
}
