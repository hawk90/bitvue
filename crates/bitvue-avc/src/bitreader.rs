//! Bit-level reader for H.264/AVC parsing.

use crate::error::{AvcError, Result};

/// Bit-level reader for parsing NAL unit payloads.
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
        self.byte_offset < self.data.len() ||
            (self.byte_offset == self.data.len() && self.bit_offset == 0)
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
            return Err(AvcError::NotEnoughData {
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
            return Err(AvcError::BitstreamError(format!(
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

    /// Read n bits as u64.
    pub fn read_bits_u64(&mut self, n: u8) -> Result<u64> {
        if n == 0 {
            return Ok(0);
        }
        if n > 64 {
            return Err(AvcError::BitstreamError(format!(
                "cannot read {} bits into u64",
                n
            )));
        }

        let mut value: u64 = 0;
        for _ in 0..n {
            value = (value << 1) | (self.read_bit()? as u64);
        }
        Ok(value)
    }

    /// Read unsigned Exp-Golomb coded value.
    pub fn read_ue(&mut self) -> Result<u32> {
        let mut leading_zeros = 0u32;
        while !self.read_bit()? {
            leading_zeros += 1;
            if leading_zeros > 32 {
                return Err(AvcError::BitstreamError(
                    "Exp-Golomb overflow".to_string(),
                ));
            }
        }

        if leading_zeros == 0 {
            return Ok(0);
        }

        let value = self.read_bits(leading_zeros as u8)?;
        Ok((1 << leading_zeros) - 1 + value)
    }

    /// Read signed Exp-Golomb coded value.
    pub fn read_se(&mut self) -> Result<i32> {
        let ue = self.read_ue()?;
        let value = ((ue + 1) / 2) as i32;
        if ue % 2 == 0 {
            Ok(-value)
        } else {
            Ok(value)
        }
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

    /// Read more_rbsp_data() check - true if there's more data before trailing bits.
    pub fn more_rbsp_data(&self) -> bool {
        if self.byte_offset >= self.data.len() {
            return false;
        }

        // Find the last byte with data
        let mut last_byte_idx = self.data.len() - 1;
        while last_byte_idx > self.byte_offset && self.data[last_byte_idx] == 0 {
            last_byte_idx -= 1;
        }

        // Check if we're past the stop bit
        if self.byte_offset > last_byte_idx {
            return false;
        }
        if self.byte_offset < last_byte_idx {
            return true;
        }

        // Same byte - check for stop bit
        let remaining = 8 - self.bit_offset;
        let mask = (1u8 << remaining) - 1;
        let trailing = self.data[last_byte_idx] & mask;

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
