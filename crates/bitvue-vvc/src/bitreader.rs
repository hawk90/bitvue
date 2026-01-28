//! Bit-level reader with Exp-Golomb support for VVC parsing.
//!
//! VVC uses the same bit reading conventions as HEVC:
//! - Bits are read MSB-first (big-endian bit order)
//! - Exp-Golomb coding for variable-length values
//!
//! This module provides a wrapper around the shared BitReader from bitvue_core
//! with VVC-specific error mapping and Exp-Golomb support.

use bitvue_core::{BitReader as CoreBitReader, ExpGolombReader};

use crate::error::{VvcError, Result};

/// VVC-specific bit reader wrapper
///
/// This wraps the core BitReader and provides VVC-specific error mapping.
pub struct BitReader<'a> {
    inner: CoreBitReader<'a>,
}

impl<'a> BitReader<'a> {
    /// Create a new bit reader from a byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            inner: CoreBitReader::new(data),
        }
    }

    /// Get the inner reader
    pub fn inner(&self) -> &CoreBitReader<'a> {
        &self.inner
    }

    /// Get mutable access to the inner reader
    pub fn inner_mut(&mut self) -> &mut CoreBitReader<'a> {
        &mut self.inner
    }

    /// Get current bit position.
    pub fn position(&self) -> u64 {
        self.inner.position()
    }

    /// Get remaining bits.
    pub fn remaining_bits(&self) -> u64 {
        self.inner.remaining_bits()
    }

    /// Check if more data is available.
    pub fn has_more_data(&self) -> bool {
        self.inner.has_more()
    }

    /// Read a single bit.
    pub fn read_bit(&mut self) -> Result<bool> {
        self.inner.read_bit().map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }

    /// Read up to 32 bits.
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        self.inner.read_bits(n).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }

    /// Read up to 64 bits.
    pub fn read_bits_u64(&mut self, n: u8) -> Result<u64> {
        self.inner.read_bits_u64(n).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }

    /// Read a single byte.
    pub fn read_byte(&mut self) -> Result<u8> {
        self.inner.read_byte().map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }

    /// Skip n bits.
    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        self.inner.skip_bits(n).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }

    /// Align to byte boundary.
    pub fn byte_align(&mut self) {
        self.inner.byte_align();
    }

    /// Check if currently byte-aligned.
    pub fn is_byte_aligned(&self) -> bool {
        self.inner.is_byte_aligned()
    }

    // =========================================================================
    // Exp-Golomb coded values
    // =========================================================================

    /// Read unsigned Exp-Golomb coded value (ue(v)).
    ///
    /// This uses the ExpGolombReader trait from bitvue_core.
    pub fn read_ue(&mut self) -> Result<u32> {
        ExpGolombReader::read_ue(&mut self.inner).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }

    /// Read signed Exp-Golomb coded value (se(v)).
    ///
    /// This uses the ExpGolombReader trait from bitvue_core.
    pub fn read_se(&mut self) -> Result<i32> {
        ExpGolombReader::read_se(&mut self.inner).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }

    /// Read u(n) - fixed-length unsigned integer.
    pub fn read_u(&mut self, n: u8) -> Result<u32> {
        self.read_bits(n)
    }

    /// Read f(n) - fixed-pattern bit string.
    pub fn read_f(&mut self, n: u8) -> Result<u32> {
        self.read_bits(n)
    }

    /// Read RBSP trailing bits.
    pub fn read_rbsp_trailing_bits(&mut self) -> Result<()> {
        let stop_bit = self.read_bit()?;
        if !stop_bit {
            return Err(VvcError::InvalidData(
                "Expected rbsp_stop_one_bit to be 1".to_string(),
            ));
        }
        while !self.is_byte_aligned() {
            let zero_bit = self.read_bit()?;
            if zero_bit {
                return Err(VvcError::InvalidData(
                    "Expected rbsp_alignment_zero_bit to be 0".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Check for more RBSP data.
    pub fn more_rbsp_data(&self) -> bool {
        self.has_more_data() && self.remaining_bits() > 0
    }

    /// Peek at next n bits without consuming them.
    pub fn peek_bits(&self, n: u8) -> Result<u32> {
        self.inner.peek_bits(n).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => VvcError::UnexpectedEof(pos),
            _ => VvcError::InvalidData(e.to_string()),
        })
    }
}

/// Remove emulation prevention bytes (0x03) from NAL unit payload.
pub fn remove_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x03 {
            result.push(0x00);
            result.push(0x00);
            i += 3;
        } else {
            result.push(data[i]);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bits() {
        let data = [0b10110100, 0b11001010];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_bit().unwrap(), true);
        assert_eq!(reader.read_bit().unwrap(), false);
        assert_eq!(reader.read_bits(3).unwrap(), 0b110);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1001);
    }

    #[test]
    fn test_read_ue() {
        // ue(0) = 1
        let data = [0b10000000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_ue().unwrap(), 0);

        // ue(1) = 010
        let data = [0b01000000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_ue().unwrap(), 1);

        // ue(2) = 011
        let data = [0b01100000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_ue().unwrap(), 2);
    }

    #[test]
    fn test_read_se() {
        // se(0) = ue(0) -> 0
        let data = [0b10000000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_se().unwrap(), 0);

        // se(1) = ue(1) -> +1
        let data = [0b01000000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_se().unwrap(), 1);

        // se(-1) = ue(2) -> -1
        let data = [0b01100000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_se().unwrap(), -1);
    }

    #[test]
    fn test_emulation_prevention() {
        let data = [0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x03, 0x02];
        let result = remove_emulation_prevention_bytes(&data);
        assert_eq!(result, vec![0x00, 0x00, 0x01, 0x00, 0x00, 0x02]);
    }
}
