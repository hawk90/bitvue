//! Bit-level reader for MPEG-2 parsing.
//!
//! This module provides a wrapper around the shared BitReader from bitvue_engine
//! with MPEG-2-specific error mapping.

use bitvue_engine::{BitReader as CoreBitReader, BlanketBitReaderError, WrappedBitReader};

use crate::error::{Mpeg2Error, Result};

impl BlanketBitReaderError for Mpeg2Error {
    fn not_enough_data(expected: usize) -> Self {
        Mpeg2Error::NotEnoughData { expected, got: 0 }
    }
}

/// MPEG-2-specific bit reader wrapper
///
/// This wraps the core BitReader and provides MPEG-2-specific error mapping.
pub struct BitReader<'a> {
    inner: WrappedBitReader<'a, Mpeg2Error>,
}

impl<'a> BitReader<'a> {
    /// Create a new bit reader.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            inner: WrappedBitReader::new(data),
        }
    }

    /// Get the inner reader
    pub fn inner(&self) -> &CoreBitReader<'a> {
        self.inner.inner()
    }

    /// Get mutable access to the inner reader
    pub fn inner_mut(&mut self) -> &mut CoreBitReader<'a> {
        self.inner.inner_mut()
    }

    /// Check if more data is available.
    pub fn has_more_data(&self) -> bool {
        self.inner.has_more()
    }

    /// Get remaining bits.
    pub fn remaining_bits(&self) -> usize {
        self.inner.remaining_bits() as usize
    }

    /// Get current bit position.
    pub fn bit_position(&self) -> usize {
        self.inner.position() as usize
    }

    /// Read a single bit.
    pub fn read_bit(&mut self) -> Result<bool> {
        self.inner.read_bit_blanket()
    }

    /// Read n bits as u32.
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        self.inner.read_bits_blanket(n)
    }

    /// Read n bits as u64.
    pub fn read_bits_u64(&mut self, n: u8) -> Result<u64> {
        self.inner.read_bits_u64_blanket(n)
    }

    /// Read a flag (single bit as bool).
    pub fn read_flag(&mut self) -> Result<bool> {
        self.read_bit()
    }

    /// Skip n bits.
    pub fn skip_bits(&mut self, n: usize) -> Result<()> {
        self.inner.skip_bits_blanket(n as u64)
    }

    /// Align to byte boundary.
    pub fn byte_align(&mut self) {
        self.inner.byte_align();
    }

    /// Check if byte aligned.
    pub fn is_byte_aligned(&self) -> bool {
        self.inner.is_byte_aligned()
    }

    /// Peek at next n bits without consuming.
    pub fn peek_bits(&self, n: u8) -> Result<u32> {
        self.inner
            .inner()
            .peek_bits(n)
            .map_err(|e| Mpeg2Error::BitstreamError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bits() {
        let data = [0b10110100, 0b01010101];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_bits(1).unwrap(), 1);
        assert_eq!(reader.read_bits(2).unwrap(), 0b01);
        assert_eq!(reader.read_bits(3).unwrap(), 0b101);
        assert_eq!(reader.read_bits(2).unwrap(), 0b00);
    }
}
