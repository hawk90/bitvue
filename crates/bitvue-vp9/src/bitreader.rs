//! Bit-level reader for VP9 parsing.
//!
//! VP9 uses a different bit reading convention than H.264/HEVC:
//! - Bits are read LSB-first (little-endian bit order) for the uncompressed header
//! - The compressed header uses arithmetic coding (not parsed bit-by-bit)

use crate::error::{Vp9Error, Result};

/// Bit reader for VP9 uncompressed header.
/// Reads bits LSB-first (little-endian bit order).
#[derive(Debug)]
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8, // 0-7, LSB first
}

impl<'a> BitReader<'a> {
    /// Create a new bit reader from a byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Get current bit position.
    pub fn position(&self) -> u64 {
        (self.byte_offset as u64) * 8 + (self.bit_offset as u64)
    }

    /// Get remaining bits.
    pub fn remaining_bits(&self) -> u64 {
        let total_bits = (self.data.len() as u64) * 8;
        total_bits.saturating_sub(self.position())
    }

    /// Check if more data is available.
    pub fn has_more_data(&self) -> bool {
        self.byte_offset < self.data.len()
    }

    /// Read a single bit (LSB-first).
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(Vp9Error::UnexpectedEof(self.position()));
        }

        let byte = self.data[self.byte_offset];
        let bit = (byte >> self.bit_offset) & 1;

        self.bit_offset += 1;
        if self.bit_offset == 8 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }

        Ok(bit == 1)
    }

    /// Read up to 32 bits (LSB-first).
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(Vp9Error::InvalidData(
                "Cannot read more than 32 bits at once".to_string(),
            ));
        }

        let mut result: u32 = 0;
        for i in 0..n {
            if self.read_bit()? {
                result |= 1 << i;
            }
        }
        Ok(result)
    }

    /// Read a literal value (n bits, MSB-first).
    /// VP9 spec uses f(n) for fixed-length values which are MSB-first.
    pub fn read_literal(&mut self, n: u8) -> Result<u32> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(Vp9Error::InvalidData(
                "Cannot read more than 32 bits at once".to_string(),
            ));
        }

        let mut result: u32 = 0;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit()? as u32);
        }
        Ok(result)
    }

    /// Skip n bits.
    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        let new_pos = self.position() + n;
        let total_bits = (self.data.len() as u64) * 8;
        if new_pos > total_bits {
            return Err(Vp9Error::UnexpectedEof(self.position()));
        }
        self.byte_offset = (new_pos / 8) as usize;
        self.bit_offset = (new_pos % 8) as u8;
        Ok(())
    }

    /// Align to byte boundary.
    pub fn byte_align(&mut self) {
        if self.bit_offset != 0 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }
    }

    /// Check if currently byte-aligned.
    pub fn is_byte_aligned(&self) -> bool {
        self.bit_offset == 0
    }

    /// Get remaining bytes (must be byte-aligned).
    pub fn remaining_bytes(&self) -> Option<&'a [u8]> {
        if self.is_byte_aligned() && self.byte_offset < self.data.len() {
            Some(&self.data[self.byte_offset..])
        } else {
            None
        }
    }
}

/// Bit reader for VP9 that reads MSB-first (for marker bits, etc.)
#[derive(Debug)]
pub struct MsbBitReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8, // 0-7, MSB first
}

impl<'a> MsbBitReader<'a> {
    /// Create a new MSB-first bit reader.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Get current bit position.
    pub fn position(&self) -> u64 {
        (self.byte_offset as u64) * 8 + (self.bit_offset as u64)
    }

    /// Read a single bit (MSB-first).
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(Vp9Error::UnexpectedEof(self.position()));
        }

        let byte = self.data[self.byte_offset];
        let bit = (byte >> (7 - self.bit_offset)) & 1;

        self.bit_offset += 1;
        if self.bit_offset == 8 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }

        Ok(bit == 1)
    }

    /// Read up to 32 bits (MSB-first).
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(Vp9Error::InvalidData(
                "Cannot read more than 32 bits at once".to_string(),
            ));
        }

        let mut result: u32 = 0;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit()? as u32);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bits_lsb() {
        let data = [0b10110100, 0b11001010];
        let mut reader = BitReader::new(&data);

        // LSB-first: reading from bit 0 of first byte
        assert_eq!(reader.read_bit().unwrap(), false); // bit 0
        assert_eq!(reader.read_bit().unwrap(), false); // bit 1
        assert_eq!(reader.read_bit().unwrap(), true);  // bit 2
        assert_eq!(reader.read_bit().unwrap(), false); // bit 3
        assert_eq!(reader.read_bit().unwrap(), true);  // bit 4
    }

    #[test]
    fn test_read_literal() {
        let data = [0b10110100];
        let mut reader = BitReader::new(&data);

        // f(3) reads 3 bits MSB-first: 0, 0, 1 = 1
        assert_eq!(reader.read_literal(3).unwrap(), 1);
    }

    #[test]
    fn test_msb_reader() {
        let data = [0b10110100];
        let mut reader = MsbBitReader::new(&data);

        assert_eq!(reader.read_bit().unwrap(), true);  // bit 7
        assert_eq!(reader.read_bit().unwrap(), false); // bit 6
        assert_eq!(reader.read_bit().unwrap(), true);  // bit 5
        assert_eq!(reader.read_bit().unwrap(), true);  // bit 4
    }
}
