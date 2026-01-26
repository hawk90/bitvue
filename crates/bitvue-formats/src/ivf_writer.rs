//! IVF file writer for AV1 bitstreams
//!
//! Writes AV1 OBU data to IVF (Indeo Video Format) container.
//! IVF is the simplest container format for AV1, used for testing and debugging.

use bitvue_core::{BitvueError, Result};

/// IVF file writer
pub struct IvfWriter {
    /// Output buffer
    output: Vec<u8>,
    /// Frame count
    frame_count: u32,
    /// Frame count offset in header (for updating later)
    frame_count_offset: usize,
}

impl IvfWriter {
    /// Create a new IVF writer with video parameters
    ///
    /// # Arguments
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `framerate_num` - Framerate numerator (e.g., 30)
    /// * `framerate_den` - Framerate denominator (e.g., 1 for 30fps)
    pub fn new(width: u16, height: u16, framerate_num: u32, framerate_den: u32) -> Self {
        let mut writer = Self {
            output: Vec::new(),
            frame_count: 0,
            frame_count_offset: 0,
        };
        writer.write_header(width, height, framerate_num, framerate_den);
        writer
    }

    /// Write IVF file header
    fn write_header(&mut self, width: u16, height: u16, framerate_num: u32, framerate_den: u32) {
        // IVF header format:
        // 0-3:   signature "DKIF"
        // 4-5:   version (0)
        // 6-7:   header size (32)
        // 8-11:  fourcc "AV01"
        // 12-13: width
        // 14-15: height
        // 16-19: framerate denominator
        // 20-23: framerate numerator
        // 24-27: frame count (placeholder, updated in finalize)
        // 28-31: unused

        self.output.extend_from_slice(b"DKIF"); // Signature
        self.output.extend_from_slice(&0u16.to_le_bytes()); // Version
        self.output.extend_from_slice(&32u16.to_le_bytes()); // Header size
        self.output.extend_from_slice(b"AV01"); // Fourcc
        self.output.extend_from_slice(&width.to_le_bytes());
        self.output.extend_from_slice(&height.to_le_bytes());
        self.output.extend_from_slice(&framerate_den.to_le_bytes());
        self.output.extend_from_slice(&framerate_num.to_le_bytes());

        // Frame count placeholder (will be updated in finalize)
        self.frame_count_offset = self.output.len();
        self.output.extend_from_slice(&0u32.to_le_bytes());

        // Unused bytes
        self.output.extend_from_slice(&[0u8; 4]);
    }

    /// Write a frame to the IVF file
    ///
    /// # Arguments
    /// * `frame_data` - Raw OBU data for this frame
    /// * `timestamp` - Presentation timestamp (PTS)
    pub fn write_frame(&mut self, frame_data: &[u8], timestamp: u64) -> Result<()> {
        if frame_data.is_empty() {
            return Err(BitvueError::InvalidData(
                "Cannot write empty frame data".to_string(),
            ));
        }

        // IVF frame format:
        // 0-3:  frame size (little-endian)
        // 4-11: timestamp (little-endian)
        // 12+:  frame data

        let size = frame_data.len() as u32;
        self.output.extend_from_slice(&size.to_le_bytes());
        self.output.extend_from_slice(&timestamp.to_le_bytes());
        self.output.extend_from_slice(frame_data);

        self.frame_count += 1;
        Ok(())
    }

    /// Finalize the IVF file and return the complete data
    ///
    /// This updates the frame count in the header and returns the output buffer.
    pub fn finalize(mut self) -> Vec<u8> {
        // Update frame count in header
        self.output[self.frame_count_offset..self.frame_count_offset + 4]
            .copy_from_slice(&self.frame_count.to_le_bytes());

        self.output
    }

    /// Get current frame count
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// Get current output size
    pub fn output_size(&self) -> usize {
        self.output.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ivf_writer_header() {
        let writer = IvfWriter::new(1920, 1080, 30, 1);

        // Check signature
        assert_eq!(&writer.output[0..4], b"DKIF");

        // Check version
        let version = u16::from_le_bytes([writer.output[4], writer.output[5]]);
        assert_eq!(version, 0);

        // Check header size
        let header_size = u16::from_le_bytes([writer.output[6], writer.output[7]]);
        assert_eq!(header_size, 32);

        // Check fourcc
        assert_eq!(&writer.output[8..12], b"AV01");

        // Check dimensions
        let width = u16::from_le_bytes([writer.output[12], writer.output[13]]);
        let height = u16::from_le_bytes([writer.output[14], writer.output[15]]);
        assert_eq!(width, 1920);
        assert_eq!(height, 1080);

        // Check framerate
        let fps_den = u32::from_le_bytes([
            writer.output[16],
            writer.output[17],
            writer.output[18],
            writer.output[19],
        ]);
        let fps_num = u32::from_le_bytes([
            writer.output[20],
            writer.output[21],
            writer.output[22],
            writer.output[23],
        ]);
        assert_eq!(fps_den, 1);
        assert_eq!(fps_num, 30);

        // Header should be exactly 32 bytes
        assert_eq!(writer.output.len(), 32);
    }

    #[test]
    fn test_ivf_writer_write_frame() {
        let mut writer = IvfWriter::new(640, 480, 25, 1);

        let frame_data = vec![0x12, 0x00, 0x0A]; // Dummy OBU data
        let result = writer.write_frame(&frame_data, 0);
        assert!(result.is_ok());

        // Check frame was written
        assert_eq!(writer.frame_count(), 1);

        // Frame header is 12 bytes (4 size + 8 timestamp)
        // Total: 32 (header) + 12 (frame header) + 3 (frame data) = 47
        assert_eq!(writer.output_size(), 32 + 12 + 3);
    }

    #[test]
    fn test_ivf_writer_multiple_frames() {
        let mut writer = IvfWriter::new(320, 240, 30, 1);

        // Write 3 frames
        for i in 0..3 {
            let frame_data = vec![0xFF; 100]; // 100 bytes of dummy data
            let timestamp = i as u64 * 1000;
            writer.write_frame(&frame_data, timestamp).unwrap();
        }

        assert_eq!(writer.frame_count(), 3);

        // Expected size: 32 (header) + 3 * (12 + 100) = 32 + 336 = 368
        assert_eq!(writer.output_size(), 32 + 3 * 112);
    }

    #[test]
    fn test_ivf_writer_finalize() {
        let mut writer = IvfWriter::new(1280, 720, 60, 1);

        // Write some frames
        for i in 0..5 {
            let frame_data = vec![0xAB; 50];
            writer.write_frame(&frame_data, i as u64).unwrap();
        }

        let output = writer.finalize();

        // Check frame count was updated in header (offset 24-27)
        let frame_count = u32::from_le_bytes([output[24], output[25], output[26], output[27]]);
        assert_eq!(frame_count, 5);

        // Check output is complete
        // 32 (header) + 5 * (12 + 50) = 32 + 310 = 342
        assert_eq!(output.len(), 32 + 5 * 62);
    }

    #[test]
    fn test_ivf_writer_empty_frame() {
        let mut writer = IvfWriter::new(640, 480, 30, 1);

        let result = writer.write_frame(&[], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_ivf_writer_large_frame() {
        let mut writer = IvfWriter::new(3840, 2160, 24, 1);

        // Large frame (1 MB)
        let frame_data = vec![0x55; 1024 * 1024];
        let result = writer.write_frame(&frame_data, 0);
        assert!(result.is_ok());

        assert_eq!(writer.frame_count(), 1);
    }
}
