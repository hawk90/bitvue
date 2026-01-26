//! Bit reader for AV3 OBU parsing.

use crate::error::{Av3Error, Result};

/// Bit reader for reading from byte slices.
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_offset: u8,
}

impl<'a> BitReader<'a> {
    /// Create a new bit reader from byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_offset: 0,
        }
    }

    /// Check if we have more bits to read.
    pub fn has_more(&self) -> bool {
        self.byte_pos < self.data.len()
            || (self.byte_pos == self.data.len() && self.bit_offset < 8)
    }

    /// Read a single bit.
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_pos >= self.data.len() {
            return Err(Av3Error::InsufficientData {
                expected: 1,
                actual: 0,
            });
        }

        let byte = self.data[self.byte_pos];
        let bit = (byte >> (7 - self.bit_offset)) & 1;

        self.bit_offset += 1;
        if self.bit_offset >= 8 {
            self.bit_offset = 0;
            self.byte_pos += 1;
        }

        Ok(bit != 0)
    }

    /// Read n bits.
    pub fn read_bits(&mut self, n: u8) -> Result<u64> {
        if n > 64 {
            return Err(Av3Error::InvalidData(format!(
                "Cannot read {} bits at once (max 64)",
                n
            )));
        }

        let mut result: u64 = 0;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit()? as u64);
        }
        Ok(result)
    }

    /// Read n bits as usize.
    pub fn read_bits_usize(&mut self, n: u8) -> Result<usize> {
        self.read_bits(n).map(|v| v as usize)
    }

    /// Read unsigned integer with leb128 encoding.
    pub fn read_leb128(&mut self) -> Result<u64> {
        let mut value: u64 = 0;
        let mut shift = 0;

        loop {
            let byte = self.read_bits(8)?;
            value |= ((byte & 0x7F) as u64) << shift;

            if (byte & 0x80) == 0 {
                break;
            }

            shift += 7;
            if shift >= 64 {
                return Err(Av3Error::InvalidData("LEB128 overflow".to_string()));
            }
        }

        Ok(value)
    }

    /// Read signed integer with leb128 encoding.
    pub fn read_leb128_i64(&mut self) -> Result<i64> {
        let mut value: i64 = 0;
        let mut shift = 0;
        let mut byte: u64;

        loop {
            byte = self.read_bits(8)?;
            value |= ((byte & 0x7F) as i64) << shift;

            if (byte & 0x80) == 0 {
                break;
            }

            shift += 7;
            if shift >= 64 {
                return Err(Av3Error::InvalidData("LEB128 overflow".to_string()));
            }
        }

        // Sign extend
        if shift < 64 && (byte & 0x40) != 0 {
            value |= -1i64 << (shift + 7);
        }

        Ok(value)
    }

    /// Get current byte position.
    pub fn byte_pos(&self) -> usize {
        self.byte_pos
    }

    /// Get current bit offset.
    pub fn bit_offset(&self) -> u8 {
        self.bit_offset
    }

    /// Align to next byte boundary.
    pub fn byte_align(&mut self) {
        if self.bit_offset > 0 {
            self.bit_offset = 0;
            self.byte_pos += 1;
        }
    }

    /// Get remaining bytes.
    pub fn remaining(&self) -> usize {
        if self.byte_pos >= self.data.len() {
            return 0;
        }
        self.data.len() - self.byte_pos - (if self.bit_offset > 0 { 1 } else { 0 })
    }

    /// Get slice of remaining data.
    pub fn as_slice(&self) -> &'a [u8] {
        if self.byte_pos >= self.data.len() {
            return &[];
        }
        &self.data[self.byte_pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bit() {
        let data = [0b10101010];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bit().unwrap(), true);
        assert_eq!(reader.read_bit().unwrap(), false);
        assert_eq!(reader.read_bit().unwrap(), true);
        assert_eq!(reader.read_bit().unwrap(), false);
    }

    #[test]
    fn test_read_bits() {
        let data = [0b11011010, 0b10110011];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1101);
        assert_eq!(reader.read_bits(8).unwrap(), 0b10101011);
        assert_eq!(reader.read_bits(4).unwrap(), 0b0011);
    }

    #[test]
    fn test_byte_align() {
        let data = [0b10101010, 0b11001100];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(3).unwrap(), 0b101);
        reader.byte_align();
        assert_eq!(reader.byte_pos(), 1);
        assert_eq!(reader.read_bits(8).unwrap(), 0b11001100);
    }
}
