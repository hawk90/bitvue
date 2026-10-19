//! Bit-level stream reader for AV1 parsing
//!
//! Provides bit-accurate reading operations required for parsing AV1 OBU headers
//! and syntax elements.
//!
//! This module provides a wrapper around the shared BitReader from bitvue_engine
//! with AV1-specific extensions.

use bitvue_engine::{BitReader as CoreBitReader, Result, UvlcReader};

/// AV1-specific bit reader wrapper
///
/// This wraps the core BitReader and provides AV1-specific extensions.
pub struct BitReader<'a> {
    inner: CoreBitReader<'a>,
}

impl<'a> BitReader<'a> {
    /// Creates a new BitReader from a byte slice
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

    /// Returns the current position in bits from the start
    #[inline]
    pub fn position(&self) -> u64 {
        self.inner.position()
    }

    /// Returns the current byte offset
    #[inline]
    pub fn byte_position(&self) -> usize {
        self.inner.byte_position()
    }

    /// Returns the number of remaining bytes (partial byte counts as 1)
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        self.inner.remaining_bytes()
    }

    /// Returns the number of remaining bits
    #[inline]
    pub fn remaining_bits(&self) -> u64 {
        self.inner.remaining_bits()
    }

    /// Returns true if there's more data to read
    #[inline]
    pub fn has_more(&self) -> bool {
        self.inner.has_more()
    }

    /// Reads a single bit (returns true for 1, false for 0)
    pub fn read_bit(&mut self) -> Result<bool> {
        self.inner.read_bit()
    }

    /// Reads n bits and returns them as a u32 (MSB first)
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        self.inner.read_bits(n)
    }

    /// Reads n bits and returns them as a u64 (MSB first)
    pub fn read_bits_u64(&mut self, n: u8) -> Result<u64> {
        self.inner.read_bits_u64(n)
    }

    /// Reads a single byte
    pub fn read_byte(&mut self) -> Result<u8> {
        self.inner.read_byte()
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
        self.inner.skip_bits(n)
    }

    /// Aligns to the next byte boundary
    pub fn byte_align(&mut self) {
        self.inner.byte_align();
    }

    /// Returns a slice of the remaining data (byte-aligned)
    pub fn remaining_data(&self) -> &[u8] {
        self.inner.remaining_data()
    }

    /// Reads an unsigned variable length code (uvlc)
    ///
    /// AV1 spec: uvlc() reads leadingZeros, then value
    ///
    /// This uses the UvlcReader trait from bitvue_engine.
    pub fn read_uvlc(&mut self) -> Result<u32> {
        UvlcReader::read_uvlc(&mut self.inner)
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

    #[test]
    fn positions_and_remaining_counts_follow_the_bits_read() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut r = BitReader::new(&data);
        assert_eq!((r.position(), r.byte_position()), (0, 0));
        assert_eq!((r.remaining_bits(), r.remaining_bytes()), (24, 3));
        assert!(r.has_more());

        r.read_bits(3).unwrap();
        assert_eq!((r.position(), r.byte_position()), (3, 0));
        assert_eq!(
            (r.remaining_bits(), r.remaining_bytes()),
            (21, 3),
            "a partial byte counts"
        );

        r.skip_bits(13).unwrap();
        assert_eq!((r.position(), r.remaining_bits()), (16, 8));
        r.read_bits_u64(8).unwrap();
        assert!(!r.has_more());
        assert_eq!(r.remaining_bits(), 0);
        assert!(r.skip_bits(1).is_err(), "nothing left to skip");
    }

    #[test]
    fn read_bytes_fills_the_buffer_and_stops_at_the_end_of_the_data() {
        let data = [1, 2, 3, 4];
        let mut r = BitReader::new(&data);
        let mut two = [0u8; 2];
        r.read_bytes(&mut two).unwrap();
        assert_eq!(two, [1, 2]);

        // Not byte aligned: bytes are read across the boundary.
        let mut r = BitReader::new(&data);
        r.read_bits(4).unwrap();
        let mut one = [0u8; 1];
        r.read_bytes(&mut one).unwrap();
        assert_eq!(one, [0x10]);

        // Asking for more than is left is an error, and an empty request always succeeds.
        let mut r = BitReader::new(&data);
        let mut five = [0u8; 5];
        assert!(r.read_bytes(&mut five).is_err());
        assert!(BitReader::new(&[]).read_bytes(&mut []).is_ok());
    }

    #[test]
    fn read_literal_is_read_bits_and_remaining_data_starts_at_the_current_byte() {
        let data = [0b1010_1100, 0xFF, 0x00];
        let mut a = BitReader::new(&data);
        let mut b = BitReader::new(&data);
        for n in [1u8, 3, 4, 8] {
            assert_eq!(
                a.read_literal(n).unwrap(),
                b.read_bits(n).unwrap(),
                "n = {n}"
            );
        }
        assert!(a.read_literal(9).is_err());

        let mut r = BitReader::new(&data);
        assert_eq!(r.remaining_data(), &data[..]);
        r.read_bits(3).unwrap();
        assert_eq!(
            r.remaining_data(),
            &data[..],
            "inside a byte: from the current byte"
        );
        r.byte_align();
        assert_eq!(r.remaining_data(), &data[1..]);
        r.read_byte().unwrap();
        r.read_byte().unwrap();
        assert!(r.remaining_data().is_empty());
    }

    #[test]
    fn the_inner_reader_is_the_same_reader() {
        let data = [0xF0, 0x0F];
        let mut r = BitReader::new(&data);
        assert_eq!(r.inner_mut().read_bits(4).unwrap(), 0xF);
        assert_eq!(
            r.position(),
            4,
            "a read through inner_mut moves the wrapper"
        );
        r.read_bits(4).unwrap();
        assert_eq!(r.inner().position(), 8, "and the other way round");
    }
}
