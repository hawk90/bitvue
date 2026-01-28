//! TrackedBitReader - BitReader wrapper with absolute position tracking
//!
//! Wraps the standard `BitReader` to track absolute bit positions from the
//! file start, enabling precise bit range tracking for syntax tree generation.

use crate::bitreader::BitReader as Av1BitReader;
use bitvue_core::{types::BitRange, Result};

/// A bit reader that tracks absolute bit positions from file start
///
/// This wraps the standard `BitReader` and adds a global offset, allowing
/// us to track exact bit positions for syntax tree generation.
pub struct TrackedBitReader<'a> {
    /// Underlying bit reader (relative positioning)
    pub reader: Av1BitReader<'a>,

    /// Absolute bit offset from file start
    global_offset: u64,
}

impl<'a> TrackedBitReader<'a> {
    /// Create a new tracked reader
    ///
    /// # Arguments
    ///
    /// * `data` - The data to read from
    /// * `global_offset` - Absolute bit offset of this data from file start
    pub fn new(data: &'a [u8], global_offset: u64) -> Self {
        Self {
            reader: Av1BitReader::new(data),
            global_offset,
        }
    }

    /// Get the current absolute bit position (from file start)
    #[inline]
    pub fn position(&self) -> u64 {
        self.global_offset + self.reader.position()
    }

    /// Mark the current position (convenience method for syntax building)
    #[inline]
    pub fn mark(&self) -> u64 {
        self.position()
    }

    /// Read a single bit and return value + bit range
    pub fn read_bit_tracked(&mut self) -> Result<(bool, BitRange)> {
        let start = self.position();
        let value = self.reader.read_bit()?;
        let end = self.position();
        Ok((value, BitRange::new(start, end)))
    }

    /// Read n bits and return value + bit range
    pub fn read_bits_tracked(&mut self, n: u8) -> Result<(u32, BitRange)> {
        let start = self.position();
        let value = self.reader.read_bits(n)?;
        let end = self.position();
        Ok((value, BitRange::new(start, end)))
    }

    /// Read n bits as u64 and return value + bit range
    pub fn read_bits_u64_tracked(&mut self, n: u8) -> Result<(u64, BitRange)> {
        let start = self.position();
        let value = self.reader.read_bits_u64(n)?;
        let end = self.position();
        Ok((value, BitRange::new(start, end)))
    }

    /// Read a byte and return value + bit range
    pub fn read_byte_tracked(&mut self) -> Result<(u8, BitRange)> {
        let start = self.position();
        let value = self.reader.read_byte()?;
        let end = self.position();
        Ok((value, BitRange::new(start, end)))
    }

    /// Read unsigned variable length code and return value + bit range
    pub fn read_uvlc_tracked(&mut self) -> Result<(u32, BitRange)> {
        let start = self.position();
        let value = self.reader.read_uvlc()?;
        let end = self.position();
        Ok((value, BitRange::new(start, end)))
    }

    /// Read signed value and return value + bit range
    pub fn read_su_tracked(&mut self, n: u8) -> Result<(i32, BitRange)> {
        let start = self.position();
        let value = self.reader.read_su(n)?;
        let end = self.position();
        Ok((value, BitRange::new(start, end)))
    }

    /// Read literal value and return value + bit range
    #[inline]
    pub fn read_literal_tracked(&mut self, n: u8) -> Result<(u32, BitRange)> {
        self.read_bits_tracked(n)
    }

    // Delegate remaining methods to inner reader (no tracking needed)

    /// Align to next byte boundary
    #[inline]
    pub fn byte_align(&mut self) {
        self.reader.byte_align();
    }

    /// Get remaining data slice
    #[inline]
    pub fn remaining_data(&self) -> &'a [u8] {
        self.reader.inner().remaining_data()
    }

    /// Get remaining bits count
    #[inline]
    pub fn remaining_bits(&self) -> u64 {
        self.reader.remaining_bits()
    }

    /// Check if more data is available
    #[inline]
    pub fn has_more(&self) -> bool {
        self.reader.has_more()
    }

    /// Skip n bits
    #[inline]
    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        self.reader.skip_bits(n)
    }

    /// Get current byte position in the data slice (not global)
    #[inline]
    pub fn byte_position(&self) -> usize {
        self.reader.byte_position()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_tracking() {
        let data = [0xFF, 0xFF];
        let mut reader = TrackedBitReader::new(&data, 1000);

        // Initial position should be global_offset
        assert_eq!(reader.position(), 1000);

        // Read 3 bits
        let (val, range) = reader.read_bits_tracked(3).unwrap();
        assert_eq!(val, 0b111);
        assert_eq!(range.start_bit, 1000);
        assert_eq!(range.end_bit, 1003);
        assert_eq!(reader.position(), 1003);

        // Read 5 bits
        let (val, range) = reader.read_bits_tracked(5).unwrap();
        assert_eq!(val, 0b11111);
        assert_eq!(range.start_bit, 1003);
        assert_eq!(range.end_bit, 1008);
        assert_eq!(reader.position(), 1008);
    }

    #[test]
    fn test_mark() {
        let data = [0xFF];
        let mut reader = TrackedBitReader::new(&data, 500);

        let mark1 = reader.mark();
        assert_eq!(mark1, 500);

        reader.read_bits_tracked(4).unwrap();
        let mark2 = reader.mark();
        assert_eq!(mark2, 504);
    }

    #[test]
    fn test_read_bit_tracked() {
        let data = [0b10110100];
        let mut reader = TrackedBitReader::new(&data, 0);

        let (bit, range) = reader.read_bit_tracked().unwrap();
        assert!(bit);
        assert_eq!(range, BitRange::new(0, 1));

        let (bit, range) = reader.read_bit_tracked().unwrap();
        assert!(!bit);
        assert_eq!(range, BitRange::new(1, 2));
    }

    #[test]
    fn test_byte_align() {
        let data = [0xFF, 0xFF, 0xFF];
        let mut reader = TrackedBitReader::new(&data, 100);

        reader.read_bits_tracked(3).unwrap();
        assert_eq!(reader.position(), 103);

        reader.byte_align();
        assert_eq!(reader.position(), 108); // Aligned to byte 1 (100 + 8)
    }

    #[test]
    fn test_uvlc_tracked() {
        // uvlc(0) = 1 (single bit)
        // uvlc(1) = 010 (3 bits)
        let data = [0b10100000];
        let mut reader = TrackedBitReader::new(&data, 0);

        let (val, range) = reader.read_uvlc_tracked().unwrap();
        assert_eq!(val, 0);
        assert_eq!(range.start_bit, 0);
        assert_eq!(range.end_bit, 1);

        let (val, range) = reader.read_uvlc_tracked().unwrap();
        assert_eq!(val, 1);
        assert_eq!(range.start_bit, 1);
        assert_eq!(range.end_bit, 4);
    }
}
