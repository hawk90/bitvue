//! Simple MSB-first bit reader for AVS3 bitstream parsing.

use crate::error::{Avs3Error, Result};

/// MSB-first bit reader over a byte slice.
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8, // 0..=7, counts from MSB
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Current byte offset (floor).
    pub fn byte_offset(&self) -> usize {
        self.byte_pos
    }

    /// Remaining bits.
    pub fn bits_remaining(&self) -> usize {
        if self.byte_pos >= self.data.len() {
            return 0;
        }
        (self.data.len() - self.byte_pos) * 8 - self.bit_pos as usize
    }

    /// Read exactly `n` bits (n ≤ 32) as u32.
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        debug_assert!(n <= 32, "read_bits: n={n} > 32");
        if self.bits_remaining() < n as usize {
            return Err(Avs3Error::UnexpectedEof);
        }
        let mut result = 0u32;
        for _ in 0..n {
            let byte = self.data[self.byte_pos];
            let bit = (byte >> (7 - self.bit_pos)) & 1;
            result = (result << 1) | bit as u32;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
        Ok(result)
    }

    /// Read a single bit as bool.
    pub fn read_flag(&mut self) -> Result<bool> {
        Ok(self.read_bits(1)? != 0)
    }

    /// Read unsigned exp-Golomb coded syntax element (ue(v)).
    pub fn read_ue(&mut self) -> Result<u32> {
        let mut leading_zeros = 0u32;
        while self.read_bits(1)? == 0 {
            leading_zeros += 1;
            if leading_zeros > 31 {
                return Err(Avs3Error::BitreaderError(
                    "exp-Golomb: too many leading zeros".into(),
                ));
            }
        }
        if leading_zeros == 0 {
            return Ok(0);
        }
        let suffix = self.read_bits(leading_zeros as u8)?;
        Ok((1 << leading_zeros) - 1 + suffix)
    }

    /// Byte-align the reader (skip remaining bits in current byte).
    pub fn byte_align(&mut self) {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    /// Peek at next `n` bits without consuming them.
    pub fn peek_bits(&self, n: u8) -> Result<u32> {
        let mut cloned = BitReader {
            data: self.data,
            byte_pos: self.byte_pos,
            bit_pos: self.bit_pos,
        };
        cloned.read_bits(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bits_basic() {
        let data = [0b10110100u8, 0b00001111u8];
        let mut r = BitReader::new(&data);
        assert_eq!(r.read_bits(4).unwrap(), 0b1011);
        assert_eq!(r.read_bits(4).unwrap(), 0b0100);
        assert_eq!(r.read_bits(8).unwrap(), 0b00001111);
    }

    #[test]
    fn read_ue() {
        // ue(v): 0→"1", 1→"010", 2→"011", 3→"00100"
        let data = [0b10100110u8, 0b10000000u8];
        let mut r = BitReader::new(&data);
        assert_eq!(r.read_ue().unwrap(), 0); // "1"
        assert_eq!(r.read_ue().unwrap(), 1); // "010"
        assert_eq!(r.read_ue().unwrap(), 2); // "011"
    }
}
