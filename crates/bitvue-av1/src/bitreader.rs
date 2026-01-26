//! Bit-level stream reader for AV1 parsing
//!
//! Provides bit-accurate reading operations required for parsing AV1 OBU headers
//! and syntax elements.

use bitvue_core::{BitvueError, Result};

/// A bit-level reader for parsing bitstreams
#[derive(Debug)]
pub struct BitReader<'a> {
    /// Source data
    data: &'a [u8],
    /// Current byte offset
    byte_offset: usize,
    /// Current bit offset within the byte (0-7, MSB first)
    bit_offset: u8,
}

impl<'a> BitReader<'a> {
    /// Creates a new BitReader from a byte slice
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Returns the current position in bits from the start
    #[inline]
    pub fn position(&self) -> u64 {
        (self.byte_offset as u64) * 8 + (self.bit_offset as u64)
    }

    /// Returns the current byte offset
    #[inline]
    pub fn byte_position(&self) -> usize {
        self.byte_offset
    }

    /// Returns the number of remaining bytes (partial byte counts as 1)
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        if self.byte_offset >= self.data.len() {
            0
        } else {
            self.data.len() - self.byte_offset
        }
    }

    /// Returns the number of remaining bits
    #[inline]
    pub fn remaining_bits(&self) -> u64 {
        if self.byte_offset >= self.data.len() {
            return 0;
        }
        let full_bytes = self.data.len() - self.byte_offset - 1;
        let bits_in_current = 8 - self.bit_offset as u64;
        bits_in_current + (full_bytes as u64) * 8
    }

    /// Returns true if there's more data to read
    #[inline]
    pub fn has_more(&self) -> bool {
        self.byte_offset < self.data.len()
    }

    /// Reads a single bit (returns true for 1, false for 0)
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(BitvueError::UnexpectedEof(self.position()));
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

    /// Reads n bits and returns them as a u32 (MSB first)
    ///
    /// # Arguments
    /// * `n` - Number of bits to read (1-32)
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(BitvueError::Parse {
                offset: self.position(),
                message: format!("Cannot read more than 32 bits at once, requested {}", n),
            });
        }

        let mut result: u32 = 0;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit()? as u32);
        }
        Ok(result)
    }

    /// Reads n bits and returns them as a u64 (MSB first)
    ///
    /// # Arguments
    /// * `n` - Number of bits to read (1-64)
    pub fn read_bits_u64(&mut self, n: u8) -> Result<u64> {
        if n == 0 {
            return Ok(0);
        }
        if n > 64 {
            return Err(BitvueError::Parse {
                offset: self.position(),
                message: format!("Cannot read more than 64 bits at once, requested {}", n),
            });
        }

        let mut result: u64 = 0;
        for _ in 0..n {
            result = (result << 1) | (self.read_bit()? as u64);
        }
        Ok(result)
    }

    /// Reads a single byte
    pub fn read_byte(&mut self) -> Result<u8> {
        if self.bit_offset == 0 {
            // Byte-aligned, fast path
            if self.byte_offset >= self.data.len() {
                return Err(BitvueError::UnexpectedEof(self.position()));
            }
            let byte = self.data[self.byte_offset];
            self.byte_offset += 1;
            Ok(byte)
        } else {
            // Not byte-aligned, read 8 bits
            self.read_bits(8).map(|v| v as u8)
        }
    }

    /// Reads multiple bytes into a slice
    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        for byte in buf.iter_mut() {
            *byte = self.read_byte()?;
        }
        Ok(())
    }

    /// Skips n bits
    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        let new_pos = self.position() + n;
        let new_byte = (new_pos / 8) as usize;
        let new_bit = (new_pos % 8) as u8;

        if new_byte > self.data.len() || (new_byte == self.data.len() && new_bit > 0) {
            return Err(BitvueError::UnexpectedEof(self.position()));
        }

        self.byte_offset = new_byte;
        self.bit_offset = new_bit;
        Ok(())
    }

    /// Aligns to the next byte boundary
    pub fn byte_align(&mut self) {
        if self.bit_offset != 0 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }
    }

    /// Returns a slice of the remaining data (byte-aligned)
    ///
    /// Note: If not byte-aligned, this returns from the current byte position
    pub fn remaining_data(&self) -> &'a [u8] {
        if self.byte_offset >= self.data.len() {
            &[]
        } else {
            &self.data[self.byte_offset..]
        }
    }

    /// Reads an unsigned variable length code (uvlc)
    ///
    /// AV1 spec: uvlc() reads leadingZeros, then value
    pub fn read_uvlc(&mut self) -> Result<u32> {
        let mut leading_zeros = 0u32;
        while !self.read_bit()? {
            leading_zeros += 1;
            if leading_zeros > 32 {
                return Err(BitvueError::Parse {
                    offset: self.position(),
                    message: "uvlc leading zeros exceeded 32".to_string(),
                });
            }
        }

        if leading_zeros == 0 {
            return Ok(0);
        }

        let value = self.read_bits(leading_zeros as u8)?;
        Ok((1 << leading_zeros) - 1 + value)
    }

    /// Reads a signed value using su(n) syntax
    ///
    /// AV1 spec: su(n) is an n-bit signed value
    pub fn read_su(&mut self, n: u8) -> Result<i32> {
        let value = self.read_bits(n)?;
        let sign_mask = 1u32 << (n - 1);
        if value & sign_mask != 0 {
            Ok(value as i32 - (1i32 << n))
        } else {
            Ok(value as i32)
        }
    }

    /// Reads a literal value with n bits (same as read_bits, named for spec consistency)
    #[inline]
    pub fn read_literal(&mut self, n: u8) -> Result<u32> {
        self.read_bits(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bit() {
        let data = [0b10110100];
        let mut reader = BitReader::new(&data);

        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
    }

    #[test]
    fn test_read_bits() {
        let data = [0b10110100, 0b11110000];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_bits(4).unwrap(), 0b1011);
        assert_eq!(reader.read_bits(4).unwrap(), 0b0100);
        assert_eq!(reader.read_bits(8).unwrap(), 0b11110000);
    }

    #[test]
    fn test_read_byte() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_byte().unwrap(), 0xAB);
        assert_eq!(reader.read_byte().unwrap(), 0xCD);
        assert_eq!(reader.read_byte().unwrap(), 0xEF);
    }

    #[test]
    fn test_position() {
        let data = [0xFF, 0xFF];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.position(), 0);
        reader.read_bits(3).unwrap();
        assert_eq!(reader.position(), 3);
        reader.read_bits(5).unwrap();
        assert_eq!(reader.position(), 8);
        reader.read_bits(4).unwrap();
        assert_eq!(reader.position(), 12);
    }

    #[test]
    fn test_skip_and_align() {
        let data = [0xFF, 0xFF, 0xFF];
        let mut reader = BitReader::new(&data);

        reader.read_bits(3).unwrap();
        assert_eq!(reader.position(), 3);

        reader.byte_align();
        assert_eq!(reader.position(), 8);

        reader.skip_bits(4).unwrap();
        assert_eq!(reader.position(), 12);
    }

    #[test]
    fn test_uvlc() {
        // uvlc encoding: 0 -> 1 (single 1 bit)
        //               1 -> 010 (one 0, then 1, then 0)
        //               2 -> 011
        //               3 -> 00100
        let data = [0b10100110, 0b01000000];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_uvlc().unwrap(), 0); // 1
        assert_eq!(reader.read_uvlc().unwrap(), 1); // 010
        assert_eq!(reader.read_uvlc().unwrap(), 2); // 011
        assert_eq!(reader.read_uvlc().unwrap(), 3); // 00100
    }

    #[test]
    fn test_eof_error() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data);

        reader.read_bits(8).unwrap();
        assert!(reader.read_bit().is_err());
    }
}
