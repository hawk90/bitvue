//! Bit-level reader for MPEG-2 parsing.

use crate::error::{Mpeg2Error, Result};

/// Bit-level reader for parsing MPEG-2 syntax elements.
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8,
}

impl<'a> BitReader<'a> {
    /// Create a new bit reader.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Check if more data is available.
    pub fn has_more_data(&self) -> bool {
        self.byte_offset < self.data.len()
    }

    /// Get remaining bits.
    pub fn remaining_bits(&self) -> usize {
        if self.byte_offset >= self.data.len() {
            return 0;
        }
        (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize
    }

    /// Get current bit position.
    pub fn bit_position(&self) -> usize {
        self.byte_offset * 8 + self.bit_offset as usize
    }

    /// Read a single bit.
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(Mpeg2Error::NotEnoughData {
                expected: 1,
                got: 0,
            });
        }

        let bit = (self.data[self.byte_offset] >> (7 - self.bit_offset)) & 1;
        self.bit_offset += 1;
        if self.bit_offset == 8 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }

        Ok(bit == 1)
    }

    /// Read n bits as u32.
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(Mpeg2Error::BitstreamError(format!(
                "cannot read {} bits into u32",
                n
            )));
        }

        let mut value: u32 = 0;
        for _ in 0..n {
            value = (value << 1) | (self.read_bit()? as u32);
        }
        Ok(value)
    }

    /// Read a flag (single bit as bool).
    pub fn read_flag(&mut self) -> Result<bool> {
        self.read_bit()
    }

    /// Skip n bits.
    pub fn skip_bits(&mut self, n: usize) -> Result<()> {
        for _ in 0..n {
            self.read_bit()?;
        }
        Ok(())
    }

    /// Align to byte boundary.
    pub fn byte_align(&mut self) {
        if self.bit_offset != 0 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }
    }

    /// Check if byte aligned.
    pub fn is_byte_aligned(&self) -> bool {
        self.bit_offset == 0
    }

    /// Peek at next n bits without consuming.
    pub fn peek_bits(&self, n: u8) -> Result<u32> {
        let mut temp = BitReader::new(&self.data[self.byte_offset..]);
        temp.bit_offset = self.bit_offset;
        temp.read_bits(n)
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
