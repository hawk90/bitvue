//! Bit-level reader with Exp-Golomb support for VVC parsing.
//!
//! VVC uses the same bit reading conventions as HEVC:
//! - Bits are read MSB-first (big-endian bit order)
//! - Exp-Golomb coding for variable-length values

use crate::error::{VvcError, Result};

/// Bit reader for VVC bitstream parsing.
/// Reads bits MSB-first (big-endian bit order).
#[derive(Debug)]
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8, // 0-7, MSB first
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

    /// Read a single bit.
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(VvcError::UnexpectedEof(self.position()));
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

    /// Read up to 32 bits.
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(VvcError::InvalidData(
                "Cannot read more than 32 bits at once".to_string(),
            ));
        }

        let mut result: u32 = 0;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit()? as u32);
        }
        Ok(result)
    }

    /// Read up to 64 bits.
    pub fn read_bits_u64(&mut self, n: u8) -> Result<u64> {
        if n == 0 {
            return Ok(0);
        }
        if n > 64 {
            return Err(VvcError::InvalidData(
                "Cannot read more than 64 bits at once".to_string(),
            ));
        }

        let mut result: u64 = 0;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit()? as u64);
        }
        Ok(result)
    }

    /// Read a single byte.
    pub fn read_byte(&mut self) -> Result<u8> {
        self.read_bits(8).map(|v| v as u8)
    }

    /// Skip n bits.
    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        let new_pos = self.position() + n;
        let total_bits = (self.data.len() as u64) * 8;
        if new_pos > total_bits {
            return Err(VvcError::UnexpectedEof(self.position()));
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

    // =========================================================================
    // Exp-Golomb coded values
    // =========================================================================

    /// Read unsigned Exp-Golomb coded value (ue(v)).
    pub fn read_ue(&mut self) -> Result<u32> {
        let mut leading_zeros: u32 = 0;
        while !self.read_bit()? {
            leading_zeros += 1;
            if leading_zeros > 32 {
                return Err(VvcError::InvalidData(
                    "Exp-Golomb leading zeros exceed 32".to_string(),
                ));
            }
        }

        if leading_zeros == 0 {
            return Ok(0);
        }

        let info = self.read_bits(leading_zeros as u8)?;
        Ok((1 << leading_zeros) - 1 + info)
    }

    /// Read signed Exp-Golomb coded value (se(v)).
    pub fn read_se(&mut self) -> Result<i32> {
        let ue = self.read_ue()?;
        let sign = if ue & 1 == 0 { -1 } else { 1 };
        Ok(sign * ((ue + 1) / 2) as i32)
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
        let mut temp = Self {
            data: self.data,
            byte_offset: self.byte_offset,
            bit_offset: self.bit_offset,
        };
        temp.read_bits(n)
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
