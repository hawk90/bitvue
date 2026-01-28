//! Bit-level reader for H.264/AVC parsing.
//!
//! This module provides a wrapper around the shared BitReader from bitvue_core
//! with AVC-specific error mapping and Exp-Golomb support.

use bitvue_core::{BitReader as CoreBitReader, ExpGolombReader};

use crate::error::{AvcError, Result};

/// AVC-specific bit reader wrapper
///
/// This wraps the core BitReader and provides AVC-specific error mapping.
pub struct BitReader<'a> {
    inner: CoreBitReader<'a>,
}

impl<'a> BitReader<'a> {
    /// Create a new bit reader.
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
        self.inner.read_bit().map_err(|_| AvcError::NotEnoughData {
            expected: 1,
            got: 0,
        })
    }

    /// Read n bits as u32.
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        self.inner
            .read_bits(n)
            .map_err(|_| AvcError::NotEnoughData {
                expected: n as usize,
                got: 0,
            })
    }

    /// Read n bits as u64.
    pub fn read_bits_u64(&mut self, n: u8) -> Result<u64> {
        self.inner
            .read_bits_u64(n)
            .map_err(|_| AvcError::NotEnoughData {
                expected: n as usize,
                got: 0,
            })
    }

    /// Read unsigned Exp-Golomb coded value.
    ///
    /// This uses the ExpGolombReader trait from bitvue_core.
    pub fn read_ue(&mut self) -> Result<u32> {
        ExpGolombReader::read_ue(&mut self.inner).map_err(|_| AvcError::NotEnoughData {
            expected: 1,
            got: 0,
        })
    }

    /// Read signed Exp-Golomb coded value.
    ///
    /// This uses the ExpGolombReader trait from bitvue_core.
    pub fn read_se(&mut self) -> Result<i32> {
        ExpGolombReader::read_se(&mut self.inner).map_err(|_| AvcError::NotEnoughData {
            expected: 1,
            got: 0,
        })
    }

    /// Read a flag (single bit as bool).
    pub fn read_flag(&mut self) -> Result<bool> {
        self.read_bit()
    }

    /// Skip n bits.
    pub fn skip_bits(&mut self, n: usize) -> Result<()> {
        self.inner
            .skip_bits(n as u64)
            .map_err(|_| AvcError::NotEnoughData {
                expected: n,
                got: 0,
            })
    }

    /// Align to byte boundary.
    pub fn byte_align(&mut self) {
        self.inner.byte_align();
    }

    /// Check if byte aligned.
    pub fn is_byte_aligned(&self) -> bool {
        self.inner.is_byte_aligned()
    }

    /// Read more_rbsp_data() check - true if there's more data before trailing bits.
    pub fn more_rbsp_data(&self) -> bool {
        let pos = self.inner.position() as usize;
        let data = self.inner.remaining_data();

        if pos / 8 >= data.len() {
            return false;
        }

        // Find the last byte with data
        let mut last_byte_idx = data.len() - 1;
        while last_byte_idx > 0 && data[last_byte_idx] == 0 {
            last_byte_idx -= 1;
        }

        // Check if we're past the stop bit
        let current_bit_in_byte = pos % 8;
        let current_byte = pos / 8;

        if current_byte > last_byte_idx {
            return false;
        }
        if current_byte < last_byte_idx {
            return true;
        }

        // Same byte - check for stop bit
        let remaining = 8 - current_bit_in_byte;
        let mask = (1u8 << remaining) - 1;
        let trailing = data[last_byte_idx] & mask;

        // If there's only the stop bit (10...0), no more data
        // Otherwise, there's more data
        trailing != (1u8 << (remaining - 1))
    }
}

/// Remove emulation prevention bytes (0x03 after 0x00 0x00).
pub fn remove_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x03 {
            result.push(0x00);
            result.push(0x00);
            i += 3; // Skip the emulation prevention byte
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
        let data = [0b10110100, 0b01010101];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_bits(1).unwrap(), 1);
        assert_eq!(reader.read_bits(2).unwrap(), 0b01);
        assert_eq!(reader.read_bits(3).unwrap(), 0b101);
        assert_eq!(reader.read_bits(2).unwrap(), 0b00);
    }

    #[test]
    fn test_read_ue() {
        // 1 -> codeword 1 (binary: 1)
        // 0 -> codeword 010 (binary: 010)
        let data = [0b10100000];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_ue().unwrap(), 0); // codeword: 1
        assert_eq!(reader.read_ue().unwrap(), 1); // codeword: 010
    }

    #[test]
    fn test_remove_emulation_prevention() {
        let data = [0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x03, 0x02];
        let result = remove_emulation_prevention_bytes(&data);
        assert_eq!(result, vec![0x00, 0x00, 0x01, 0x00, 0x00, 0x02]);
    }
}
