//! Unified bit-level stream reader for codec parsing
//!
//! This module provides a comprehensive BitReader implementation that supports
//! both MSB-first (big-endian) and LSB-first (little-endian) bit reading,
//! used across various video codec standards (AV1, HEVC, VVC, AVC, VP9, etc.).
//!
//! # Architecture
//!
//! - [`BitReader`]: Core MSB-first reader (most codecs)
//! - [`LsbBitReader`]: LSB-first reader (VP9 uncompressed header)
//! - Extension traits for codec-specific operations (Exp-Golomb, UVLC, etc.)
//!
//! # Example
//!
//! ```ignore
//! use bitvue_core::BitReader;
//!
//! let data = [0b10110100, 0b11110000];
//! let mut reader = BitReader::new(&data);
//!
//! // Read 4 bits (MSB-first): 0b1011
//! let bits = reader.read_bits(4).unwrap();
//! assert_eq!(bits, 0b1011);
//!
//! // Check position (in bits)
//! assert_eq!(reader.position(), 4);
//! ```

use crate::{BitvueError, Result};

/// Core bit-level reader for MSB-first (big-endian bit order) parsing.
///
/// This is the standard bit reader used by most video codecs:
/// - AV1 (OBU headers, syntax elements)
/// - HEVC/H.265 (NAL units, syntax elements)
/// - VVC/H.266 (NAL units, syntax elements)
/// - AVC/H.264 (NAL units, syntax elements)
/// - MPEG-2 (syntax elements)
/// - AV3 (OBU parsing)
///
/// # Bit Reading Order
///
/// Bits are read MSB-first within each byte. For a byte `0b10110100`:
/// - Bit 0: 1 (MSB, position 7)
/// - Bit 1: 0 (position 6)
/// - Bit 2: 1 (position 5)
/// - Bit 3: 1 (position 4)
/// - Bit 4: 0 (position 3)
/// - Bit 5: 1 (position 2)
/// - Bit 6: 0 (position 1)
/// - Bit 7: 0 (LSB, position 0)
#[derive(Debug, Clone, Copy)]
pub struct BitReader<'a> {
    /// Source data slice
    data: &'a [u8],
    /// Current byte offset
    byte_offset: usize,
    /// Current bit offset within the byte (0-7, MSB first)
    bit_offset: u8,
}

impl<'a> BitReader<'a> {
    /// Creates a new BitReader from a byte slice.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to read from
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Returns the current position in bits from the start.
    ///
    /// Uses checked arithmetic to prevent overflow on malicious inputs
    /// (e.g., extremely large byte offsets).
    #[inline]
    pub fn position(&self) -> u64 {
        let byte_bits = (self.byte_offset as u64)
            .checked_mul(8)
            .unwrap_or(u64::MAX);
        byte_bits
            .checked_add(self.bit_offset as u64)
            .unwrap_or(u64::MAX)
    }

    /// Returns the current byte offset.
    #[inline]
    pub fn byte_position(&self) -> usize {
        self.byte_offset
    }

    /// Returns the number of remaining bits.
    #[inline]
    pub fn remaining_bits(&self) -> u64 {
        if self.byte_offset >= self.data.len() {
            return 0;
        }
        let full_bytes = self.data.len() - self.byte_offset - 1;
        let bits_in_current = 8 - self.bit_offset as u64;
        bits_in_current + (full_bytes as u64) * 8
    }

    /// Returns the number of remaining bytes (partial byte counts as 1).
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        if self.byte_offset >= self.data.len() {
            0
        } else {
            self.data.len() - self.byte_offset
        }
    }

    /// Returns true if there's more data to read.
    #[inline]
    pub fn has_more(&self) -> bool {
        self.byte_offset < self.data.len()
    }

    /// Reads a single bit (returns true for 1, false for 0).
    ///
    /// # Errors
    ///
    /// Returns [`BitvueError::UnexpectedEof`] if attempting to read beyond data.
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

    /// Reads n bits and returns them as a u32 (MSB first).
    ///
    /// # Arguments
    ///
    /// * `n` - Number of bits to read (1-32)
    ///
    /// # Errors
    ///
    /// - [`BitvueError::UnexpectedEof`] - Not enough data available
    /// - [`BitvueError::Parse`] - n > 32
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

    /// Reads n bits and returns them as a u64 (MSB first).
    ///
    /// # Arguments
    ///
    /// * `n` - Number of bits to read (1-64)
    ///
    /// # Errors
    ///
    /// - [`BitvueError::UnexpectedEof`] - Not enough data available
    /// - [`BitvueError::Parse`] - n > 64
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

    /// Reads a single byte.
    ///
    /// If byte-aligned, uses fast path. Otherwise reads 8 bits.
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

    /// Reads multiple bytes into a slice.
    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        for byte in buf.iter_mut() {
            *byte = self.read_byte()?;
        }
        Ok(())
    }

    /// Skips n bits.
    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        // Use checked arithmetic to prevent overflow on very large skip values
        let new_pos = self
            .position()
            .checked_add(n)
            .ok_or_else(|| BitvueError::Decode("Skip would cause position overflow".to_string()))?;

        let new_byte = (new_pos / 8) as usize;
        let new_bit = (new_pos % 8) as u8;

        if new_byte > self.data.len() || (new_byte == self.data.len() && new_bit > 0) {
            return Err(BitvueError::UnexpectedEof(self.position()));
        }

        self.byte_offset = new_byte;
        self.bit_offset = new_bit;
        Ok(())
    }

    /// Aligns to the next byte boundary.
    pub fn byte_align(&mut self) {
        if self.bit_offset != 0 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }
    }

    /// Returns true if currently byte-aligned.
    #[inline]
    pub fn is_byte_aligned(&self) -> bool {
        self.bit_offset == 0
    }

    /// Returns a slice of the remaining data (byte-aligned).
    ///
    /// Note: If not byte-aligned, this returns from the current byte position.
    pub fn remaining_data(&self) -> &'a [u8] {
        if self.byte_offset >= self.data.len() {
            &[]
        } else {
            &self.data[self.byte_offset..]
        }
    }

    /// Peeks at the next n bits without consuming them.
    pub fn peek_bits(&self, n: u8) -> Result<u32> {
        let mut temp = Self {
            data: self.data,
            byte_offset: self.byte_offset,
            bit_offset: self.bit_offset,
        };
        temp.read_bits(n)
    }
}

/// Bit-level reader for LSB-first (little-endian bit order) parsing.
///
/// Used by VP9 for the uncompressed header. VP9's compressed header uses
/// arithmetic coding and doesn't use bit-by-bit reading.
///
/// # Bit Reading Order
///
/// Bits are read LSB-first within each byte. For a byte `0b10110100`:
/// - Bit 0: 0 (LSB, position 0)
/// - Bit 1: 0 (position 1)
/// - Bit 2: 1 (position 2)
/// - Bit 3: 0 (position 3)
/// - Bit 4: 1 (position 4)
/// - Bit 5: 1 (position 5)
/// - Bit 6: 0 (position 6)
/// - Bit 7: 1 (MSB, position 7)
#[derive(Debug, Clone, Copy)]
pub struct LsbBitReader<'a> {
    /// Source data slice
    data: &'a [u8],
    /// Current byte offset
    byte_offset: usize,
    /// Current bit offset within the byte (0-7, LSB first)
    bit_offset: u8,
}

impl<'a> LsbBitReader<'a> {
    /// Creates a new LSB-first BitReader from a byte slice.
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Returns the current position in bits from the start.
    ///
    /// Uses checked arithmetic to prevent overflow on malicious inputs
    /// (e.g., extremely large byte offsets).
    #[inline]
    pub fn position(&self) -> u64 {
        let byte_bits = (self.byte_offset as u64)
            .checked_mul(8)
            .unwrap_or(u64::MAX);
        byte_bits
            .checked_add(self.bit_offset as u64)
            .unwrap_or(u64::MAX)
    }

    /// Returns the number of remaining bits.
    #[inline]
    pub fn remaining_bits(&self) -> u64 {
        let total_bits = (self.data.len() as u64) * 8;
        total_bits.saturating_sub(self.position())
    }

    /// Returns true if there's more data to read.
    #[inline]
    pub fn has_more(&self) -> bool {
        self.byte_offset < self.data.len()
    }

    /// Reads a single bit (LSB-first).
    pub fn read_bit(&mut self) -> Result<bool> {
        if self.byte_offset >= self.data.len() {
            return Err(BitvueError::UnexpectedEof(self.position()));
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

    /// Reads up to 32 bits (LSB-first).
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
        for i in 0..n {
            if self.read_bit()? {
                result |= 1 << i;
            }
        }
        Ok(result)
    }

    /// Skips n bits.
    pub fn skip_bits(&mut self, n: u64) -> Result<()> {
        // Use checked arithmetic to prevent overflow on very large skip values
        let new_pos = self
            .position()
            .checked_add(n)
            .ok_or_else(|| BitvueError::Decode("Skip would cause position overflow".to_string()))?;

        let total_bits = (self.data.len() as u64) * 8;
        if new_pos > total_bits {
            return Err(BitvueError::UnexpectedEof(self.position()));
        }
        self.byte_offset = (new_pos / 8) as usize;
        self.bit_offset = (new_pos % 8) as u8;
        Ok(())
    }

    /// Aligns to the next byte boundary.
    pub fn byte_align(&mut self) {
        if self.bit_offset != 0 {
            self.bit_offset = 0;
            self.byte_offset += 1;
        }
    }

    /// Returns true if currently byte-aligned.
    #[inline]
    pub fn is_byte_aligned(&self) -> bool {
        self.bit_offset == 0
    }
}

// ============================================================================
// Extension Traits for Codec-Specific Operations
// ============================================================================

/// Exp-Golomb coding extension for BitReader.
///
/// Used by H.264/AVC, HEVC/H.265, and VVC/H.266 for variable-length coding.
pub trait ExpGolombReader {
    /// Reads an unsigned Exp-Golomb coded value (ue(v)).
    ///
    /// Format: [M zeros][1][INFO]
    /// Value = 2^M + INFO - 1
    ///
    /// # Examples
    /// - ue(0) = 1 (binary: 1)
    /// - ue(1) = 010 (binary: 010)
    /// - ue(2) = 011 (binary: 011)
    /// - ue(3) = 00100 (binary: 00100)
    fn read_ue(&mut self) -> Result<u32>;

    /// Reads a signed Exp-Golomb coded value (se(v)).
    ///
    /// Derived from ue(v): se = (-1)^(k+1) * ceil(k/2)
    fn read_se(&mut self) -> Result<i32>;
}

impl ExpGolombReader for BitReader<'_> {
    fn read_ue(&mut self) -> Result<u32> {
        // Fast path: Use leading_zeros() intrinsic when we have 32+ bits available
        if self.remaining_bits() >= 32 {
            let bits = self.peek_bits(32)?;
            let leading_zeros = bits.leading_zeros();

            if leading_zeros >= 32 {
                return Err(BitvueError::Parse {
                    offset: self.position(),
                    message: "Exp-Golomb leading zeros exceeded 32".to_string(),
                });
            }

            // Skip the leading zeros and the stop bit
            self.skip_bits(leading_zeros as u64 + 1)?;

            if leading_zeros == 0 {
                return Ok(0);
            }

            // Read the remaining bits
            let value = self.read_bits(leading_zeros as u8)?;
            return Ok((1 << leading_zeros) - 1 + value);
        }

        // Fallback: Original bit-by-bit implementation for short reads
        let mut leading_zeros = 0u32;
        while !self.read_bit()? {
            leading_zeros += 1;
            if leading_zeros > 32 {
                return Err(BitvueError::Parse {
                    offset: self.position(),
                    message: "Exp-Golomb leading zeros exceeded 32".to_string(),
                });
            }
        }

        if leading_zeros == 0 {
            return Ok(0);
        }

        let value = self.read_bits(leading_zeros as u8)?;
        Ok((1 << leading_zeros) - 1 + value)
    }

    fn read_se(&mut self) -> Result<i32> {
        let ue = self.read_ue()?;
        let sign = if ue & 1 == 0 { -1 } else { 1 };
        Ok(sign * ((ue + 1) / 2) as i32)
    }
}

/// UVLC (Unsigned Variable Length Code) extension for BitReader.
///
/// Used by AV1 for variable-length coding.
pub trait UvlcReader {
    /// Reads an unsigned variable length code (uvlc).
    ///
    /// AV1 spec: uvlc() reads leadingZeros, then value
    fn read_uvlc(&mut self) -> Result<u32>;
}

impl UvlcReader for BitReader<'_> {
    fn read_uvlc(&mut self) -> Result<u32> {
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
}

/// LEB128 (Little Endian Base 128) extension for BitReader.
///
/// Used by AV3 for variable-length integer coding.
pub trait Leb128Reader {
    /// Reads an unsigned LEB128 encoded integer.
    fn read_leb128(&mut self) -> Result<u64>;

    /// Reads a signed LEB128 encoded integer.
    fn read_leb128_i64(&mut self) -> Result<i64>;
}

impl Leb128Reader for BitReader<'_> {
    fn read_leb128(&mut self) -> Result<u64> {
        let mut value: u64 = 0;
        let mut shift = 0;

        loop {
            // Check for overflow BEFORE shifting to prevent undefined behavior
            if shift >= 64 {
                return Err(BitvueError::Parse {
                    offset: self.position(),
                    message: "LEB128 overflow".to_string(),
                });
            }

            let byte = self.read_bits(8)?;
            value |= ((byte & 0x7F) as u64) << shift;

            if (byte & 0x80) == 0 {
                break;
            }

            shift += 7;
        }

        Ok(value)
    }

    fn read_leb128_i64(&mut self) -> Result<i64> {
        let mut value: i64 = 0;
        let mut shift = 0;
        let mut byte: u64;

        loop {
            // Check for overflow BEFORE shifting to prevent undefined behavior
            if shift >= 64 {
                return Err(BitvueError::Parse {
                    offset: self.position(),
                    message: "LEB128 overflow".to_string(),
                });
            }

            byte = self.read_bits(8)? as u64;
            value |= ((byte & 0x7F) as i64) << shift;

            if (byte & 0x80) == 0 {
                break;
            }

            shift += 7;
        }

        // Sign extend
        if shift < 64 && (byte & 0x40) != 0 {
            value |= -1i64 << (shift + 7);
        }

        Ok(value)
    }
}

// ============================================================================
// Emulation Prevention
// ============================================================================

/// Removes emulation prevention bytes from H.264/AVC, H.265/HEVC, and H.266/VVC bitstreams.
///
/// In these codecs, the byte sequence `0x00 0x00 0x03` is used to prevent
/// accidental emulation of start codes (`0x00 0x00 0x01`) in the bitstream.
/// This function removes those `0x03` bytes to restore the original data.
///
/// # Arguments
///
/// * `data` - The raw bitstream data that may contain emulation prevention bytes
///
/// # Returns
///
/// A new Vec<u8> with emulation prevention bytes removed
///
/// # Example
///
/// ```
/// use bitvue_core::remove_emulation_prevention_bytes;
///
/// let raw = [0x00, 0x00, 0x03, 0x01, 0xFF];
/// let cleaned = remove_emulation_prevention_bytes(&raw);
/// assert_eq!(cleaned, vec![0x00, 0x00, 0x01, 0xFF]);
/// ```
pub fn remove_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x03 {
            // Found emulation prevention byte (0x00 0x00 0x03)
            // Output only the two 0x00 bytes, skip the 0x03
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // MSB-first BitReader tests

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
    fn test_peek_bits() {
        let data = [0b10110100];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.peek_bits(4).unwrap(), 0b1011);
        assert_eq!(reader.position(), 0); // Position unchanged

        reader.read_bits(2).unwrap();
        assert_eq!(reader.peek_bits(4).unwrap(), 0b1101);
    }

    #[test]
    fn test_eof_error() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data);

        reader.read_bits(8).unwrap();
        assert!(reader.read_bit().is_err());
    }

    // Exp-Golomb tests

    #[test]
    fn test_read_ue() {
        // ue(0) = 1 (single bit)
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

        // ue(3) = 00100
        let data = [0b00100000];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_ue().unwrap(), 3);
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

    // UVLC tests

    #[test]
    fn test_uvlc() {
        // uvlc(0) = 1
        // uvlc(1) = 010
        // uvlc(2) = 011
        // uvlc(3) = 00100
        let data = [0b10100110, 0b01000000];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_uvlc().unwrap(), 0); // 1
        assert_eq!(reader.read_uvlc().unwrap(), 1); // 010
        assert_eq!(reader.read_uvlc().unwrap(), 2); // 011
        assert_eq!(reader.read_uvlc().unwrap(), 3); // 00100
    }

    // Emulation prevention tests

    #[test]
    fn test_remove_emulation_prevention() {
        // Test basic removal: 0x00 0x00 0x03 -> 0x00 0x00
        let data = [0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x03, 0x02];
        let result = remove_emulation_prevention_bytes(&data);
        assert_eq!(result, vec![0x00, 0x00, 0x01, 0x00, 0x00, 0x02]);
    }

    #[test]
    fn test_remove_emulation_prevention_no_match() {
        // No emulation prevention bytes
        let data = [0x00, 0x00, 0x01, 0x02, 0x03];
        let result = remove_emulation_prevention_bytes(&data);
        assert_eq!(result, vec![0x00, 0x00, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_remove_emulation_prevention_edge_cases() {
        // Only 0x00 0x00 0x03 at the end
        let data = [0xFF, 0x00, 0x00, 0x03];
        let result = remove_emulation_prevention_bytes(&data);
        assert_eq!(result, vec![0xFF, 0x00, 0x00]);

        // Only 0x00 0x00 0x03 at the start
        let data = [0x00, 0x00, 0x03, 0xFF];
        let result = remove_emulation_prevention_bytes(&data);
        assert_eq!(result, vec![0x00, 0x00, 0xFF]);

        // Multiple consecutive emulation prevention bytes
        let data = [0x00, 0x00, 0x03, 0x00, 0x00, 0x03];
        let result = remove_emulation_prevention_bytes(&data);
        assert_eq!(result, vec![0x00, 0x00, 0x00, 0x00]);
    }

    // LSB-first BitReader tests

    #[test]
    fn test_lsb_read_bit() {
        let data = [0b10110100];
        let mut reader = LsbBitReader::new(&data);

        assert!(!reader.read_bit().unwrap()); // bit 0
        assert!(!reader.read_bit().unwrap()); // bit 1
        assert!(reader.read_bit().unwrap()); // bit 2
        assert!(!reader.read_bit().unwrap()); // bit 3
        assert!(reader.read_bit().unwrap()); // bit 4
        assert!(reader.read_bit().unwrap()); // bit 5
        assert!(!reader.read_bit().unwrap()); // bit 6
        assert!(reader.read_bit().unwrap()); // bit 7
    }

    #[test]
    fn test_lsb_read_bits() {
        let data = [0b10110100];
        let mut reader = LsbBitReader::new(&data);

        // Read 3 bits LSB-first: 0, 0, 1 = 0b100
        // bit 0 = 0, bit 1 = 0, bit 2 = 1
        assert_eq!(reader.read_bits(3).unwrap(), 0b100);

        // Read 3 more bits: 0, 1, 1 = 0b110
        assert_eq!(reader.read_bits(3).unwrap(), 0b110);
    }
}
