//! Bit buffer for storing and manipulating bits.
//!
//! This module provides a circular buffer for bit-level operations.

/// A circular bit buffer for storing bits.
#[derive(Clone, Debug)]
pub struct BitBuffer {
    data: Vec<u8>,
    capacity_bits: usize,
    read_pos: usize,
    write_pos: usize,
    size_bits: usize,
}

impl BitBuffer {
    /// Creates a new bit buffer with the specified capacity in bits.
    ///
    /// # Panics
    ///
    /// Panics if `capacity_bits` is 0 (would cause division by zero in operations).
    pub fn with_capacity(capacity_bits: usize) -> Self {
        if capacity_bits == 0 {
            panic!("BitBuffer::with_capacity: capacity_bits must be > 0");
        }
        let bytes = (capacity_bits + 7) / 8;
        BitBuffer {
            data: vec![0; bytes],
            capacity_bits,
            read_pos: 0,
            write_pos: 0,
            size_bits: 0,
        }
    }

    /// Returns the number of bits stored in the buffer.
    pub fn len(&self) -> usize {
        self.size_bits
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.size_bits == 0
    }

    /// Returns true if the buffer is full.
    pub fn is_full(&self) -> bool {
        self.size_bits == self.capacity_bits
    }

    /// Returns the capacity in bits.
    pub fn capacity(&self) -> usize {
        self.capacity_bits
    }

    /// Writes a single bit to the buffer.
    pub fn write_bit(&mut self, bit: bool) -> bool {
        if self.is_full() {
            return false;
        }
        let byte = &mut self.data[self.write_pos / 8];
        let bit_offset = self.write_pos % 8;
        if bit {
            *byte |= 1 << bit_offset;
        } else {
            *byte &= !(1 << bit_offset);
        }
        self.write_pos = (self.write_pos + 1) % self.capacity_bits;
        self.size_bits += 1;
        true
    }

    /// Reads a single bit from the buffer.
    pub fn read_bit(&mut self) -> Option<bool> {
        if self.is_empty() {
            return None;
        }
        let byte = self.data[self.read_pos / 8];
        let bit_offset = self.read_pos % 8;
        let bit = (byte >> bit_offset) & 1 == 1;
        self.read_pos = (self.read_pos + 1) % self.capacity_bits;
        self.size_bits -= 1;
        Some(bit)
    }

    /// Writes multiple bits from a value.
    pub fn write_bits(&mut self, value: u64, count: usize) -> bool {
        if count == 0 {
            return true;
        }
        if count > 64 || count > self.capacity_bits {
            return false;
        }
        for i in 0..count {
            if !self.write_bit((value >> i) & 1 == 1) {
                return false;
            }
        }
        true
    }

    /// Reads multiple bits into a value.
    pub fn read_bits(&mut self, count: usize) -> Option<u64> {
        if count == 0 {
            return Some(0);
        }
        if count > 64 || count > self.size_bits {
            return None;
        }
        let mut result = 0u64;
        for i in 0..count {
            if let Some(bit) = self.read_bit() {
                result |= (bit as u64) << i;
            } else {
                return None;
            }
        }
        Some(result)
    }

    /// Clears all bits from the buffer.
    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.size_bits = 0;
    }
}

impl Default for BitBuffer {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_buffer_write_read_bit() {
        let mut buf = BitBuffer::with_capacity(8);
        assert!(buf.write_bit(true));
        assert!(buf.write_bit(false));
        assert_eq!(buf.read_bit(), Some(true));
        assert_eq!(buf.read_bit(), Some(false));
        assert!(buf.read_bit().is_none());
    }

    #[test]
    fn test_bit_buffer_write_read_bits() {
        let mut buf = BitBuffer::with_capacity(16);
        assert!(buf.write_bits(0b1011, 4));
        assert_eq!(buf.read_bits(4), Some(0b1011));
    }

    #[test]
    fn test_bit_buffer_capacity() {
        let buf = BitBuffer::with_capacity(32);
        assert_eq!(buf.capacity(), 32);
    }

    #[test]
    fn test_bit_buffer_is_empty() {
        let buf = BitBuffer::with_capacity(8);
        assert!(buf.is_empty());
        buf.write_bit(true);
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_bit_buffer_is_full() {
        let mut buf = BitBuffer::with_capacity(8);
        for _ in 0..8 {
            buf.write_bit(true);
        }
        assert!(buf.is_full());
    }

    #[test]
    #[should_panic(expected = "capacity_bits must be > 0")]
    fn test_bit_buffer_zero_capacity() {
        BitBuffer::with_capacity(0);
    }
}
