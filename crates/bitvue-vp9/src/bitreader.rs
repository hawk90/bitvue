//! Bit-level reader for VP9 parsing.
//!
//! This module provides wrappers around the shared BitReader types from bitvue_core
//! with VP9-specific error mapping.

use bitvue_core::{BitReader as CoreMsbReader, LsbBitReader as CoreLsbReader};

use crate::error::{Result, Vp9Error};

/// VP9 LSB-first bit reader for uncompressed header
pub struct BitReader<'a> {
    inner: CoreLsbReader<'a>,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            inner: CoreLsbReader::new(data),
        }
    }

    pub fn inner(&self) -> &CoreLsbReader<'a> {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut CoreLsbReader<'a> {
        &mut self.inner
    }

    pub fn position(&self) -> u64 {
        self.inner.position()
    }
    pub fn remaining_bits(&self) -> u64 {
        self.inner.remaining_bits()
    }
    pub fn has_more_data(&self) -> bool {
        self.inner.has_more()
    }

    pub fn read_bit(&mut self) -> Result<bool> {
        self.inner.read_bit().map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => Vp9Error::UnexpectedEof(pos),
            _ => Vp9Error::InvalidData(e.to_string()),
        })
    }

    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        self.inner.read_bits(n).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => Vp9Error::UnexpectedEof(pos),
            _ => Vp9Error::InvalidData(e.to_string()),
        })
    }

    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        self.inner.skip_bits(n).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => Vp9Error::UnexpectedEof(pos),
            _ => Vp9Error::InvalidData(e.to_string()),
        })
    }

    pub fn byte_align(&mut self) {
        self.inner.byte_align();
    }
    pub fn is_byte_aligned(&self) -> bool {
        self.inner.is_byte_aligned()
    }

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
}

/// VP9 MSB-first bit reader for marker bits, etc.
pub struct MsbBitReader<'a> {
    inner: CoreMsbReader<'a>,
}

impl<'a> MsbBitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            inner: CoreMsbReader::new(data),
        }
    }

    pub fn position(&self) -> u64 {
        self.inner.position()
    }

    pub fn read_bit(&mut self) -> Result<bool> {
        self.inner.read_bit().map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => Vp9Error::UnexpectedEof(pos),
            _ => Vp9Error::InvalidData(e.to_string()),
        })
    }

    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        self.inner.read_bits(n).map_err(|e| match e {
            bitvue_core::BitvueError::UnexpectedEof(pos) => Vp9Error::UnexpectedEof(pos),
            _ => Vp9Error::InvalidData(e.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bits_lsb() {
        let data = [0b10110100, 0b11001010];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bit().unwrap(), false); // bit 0
        assert_eq!(reader.read_bit().unwrap(), false); // bit 1
        assert_eq!(reader.read_bit().unwrap(), true); // bit 2
        assert_eq!(reader.read_bit().unwrap(), false); // bit 3
        assert_eq!(reader.read_bit().unwrap(), true); // bit 4
    }

    #[test]
    fn test_read_literal() {
        let data = [0b10110100];
        let mut reader = BitReader::new(&data);
        // read_literal reads bits LSB-first but assembles MSB-first
        // First 3 bits LSB-first: 0 (bit 0), 0 (bit 1), 1 (bit 2)
        // Assembled MSB-first: 0b001 = 1
        assert_eq!(reader.read_literal(3).unwrap(), 1);
    }

    #[test]
    fn test_msb_reader() {
        let data = [0b10110100];
        let mut reader = MsbBitReader::new(&data);
        assert_eq!(reader.read_bit().unwrap(), true); // bit 7
        assert_eq!(reader.read_bit().unwrap(), false); // bit 6
        assert_eq!(reader.read_bit().unwrap(), true); // bit 5
        assert_eq!(reader.read_bit().unwrap(), true); // bit 4
    }
}
