//! Bit reader for AV3 OBU parsing.
//!
//! This module provides a wrapper around the shared BitReader from bitvue_core
//! with AV3-specific error mapping and LEB128 support.

use bitvue_core::{BitReader as CoreBitReader, Leb128Reader};

use crate::error::{Av3Error, Result};

/// AV3-specific bit reader wrapper
pub struct BitReader<'a> {
    inner: CoreBitReader<'a>,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            inner: CoreBitReader::new(data),
        }
    }

    pub fn inner(&self) -> &CoreBitReader<'a> {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut CoreBitReader<'a> {
        &mut self.inner
    }

    pub fn has_more(&self) -> bool {
        self.inner.has_more()
    }

    pub fn read_bit(&mut self) -> Result<bool> {
        self.inner
            .read_bit()
            .map_err(|_| Av3Error::InsufficientData {
                expected: 1,
                actual: 0,
            })
    }

    pub fn read_bits(&mut self, n: u8) -> Result<u64> {
        self.inner
            .read_bits_u64(n)
            .map_err(|_| Av3Error::InsufficientData {
                expected: n as usize,
                actual: 0,
            })
    }

    pub fn read_bits_usize(&mut self, n: u8) -> Result<usize> {
        self.read_bits(n).map(|v| v as usize)
    }

    pub fn read_leb128(&mut self) -> Result<u64> {
        Leb128Reader::read_leb128(&mut self.inner).map_err(|_| Av3Error::InsufficientData {
            expected: 1,
            actual: 0,
        })
    }

    pub fn read_leb128_i64(&mut self) -> Result<i64> {
        Leb128Reader::read_leb128_i64(&mut self.inner).map_err(|_| Av3Error::InsufficientData {
            expected: 1,
            actual: 0,
        })
    }

    pub fn byte_pos(&self) -> usize {
        self.inner.byte_position()
    }
    pub fn bit_offset(&self) -> u8 {
        (self.inner.position() % 8) as u8
    }
    pub fn byte_align(&mut self) {
        self.inner.byte_align();
    }
    pub fn remaining(&self) -> usize {
        self.inner.remaining_bytes()
    }
    pub fn as_slice(&self) -> &[u8] {
        self.inner.remaining_data()
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
