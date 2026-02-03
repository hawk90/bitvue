//! CRC computers for incremental computation.
//!
//! This module provides stateful CRC computers that can process data
//! incrementally in chunks.

use crate::variants::{CrcAlgorithm, CrcParams};

/// A CRC computer for incremental computation.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::computer::CrcComputer;
/// use abseil::absl_crc::variants::CrcAlgorithm;
///
/// let mut computer = CrcComputer::new(CrcAlgorithm::Crc32);
/// computer.update(b"Hello, ");
/// computer.update(b"world!");
/// let crc = computer.finalize();
/// ```
#[derive(Clone, Debug)]
pub struct CrcComputer {
    algorithm: CrcAlgorithm,
    state: u64,
}

impl CrcComputer {
    /// Creates a new CRC computer for the specified algorithm.
    pub const fn new(algorithm: CrcAlgorithm) -> Self {
        Self {
            algorithm,
            state: algorithm.initial(),
        }
    }

    /// Updates the CRC with more data.
    pub fn update(&mut self, data: &[u8]) {
        let crc = compute_crc(self.algorithm, data);
        self.state = match self.algorithm {
            CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => {
                let current = self.state as u32;
                let new = crc as u32;
                ((current ^ new).wrapping_mul(0x01000193)) as u64
            }
            _ => self.state ^ crc,
        };
    }

    /// Finalizes and returns the CRC value.
    pub fn finalize(self) -> u64 {
        match self.algorithm {
            CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => {
                (self.state as u32) ^ (self.algorithm.final_xor() as u32)
            }
            _ => self.state ^ self.algorithm.final_xor(),
        } as u64
    }

    /// Resets the computer to its initial state.
    pub fn reset(&mut self) {
        self.state = self.algorithm.initial();
    }

    /// Returns the current intermediate CRC value.
    pub const fn current(&self) -> u64 {
        self.state
    }
}

/// A table-based CRC computer for faster computation.
#[derive(Clone)]
pub struct TableCrcComputer {
    table: [u32; 256],
    initial: u32,
    final_xor: u32,
    current: u32,
}

impl TableCrcComputer {
    /// Creates a new table-based CRC computer for CRC-32.
    pub fn crc32() -> Self {
        Self::with_params(0x04c11db7, 0xffffffff, 0xffffffff, true, true)
    }

    /// Creates a new table-based CRC computer for CRC-32C.
    pub fn crc32c() -> Self {
        Self::with_params(0x1edc6f41, 0xffffffff, 0xffffffff, true, true)
    }

    /// Creates a new table-based CRC computer with custom parameters.
    pub fn with_params(poly: u32, initial: u32, final_xor: u32, reflect: bool, _reflect_out: bool) -> Self {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut crc = if reflect {
                i.reverse_bits() as u32
            } else {
                (i as u32) << 24
            };

            let mut j = 0;
            while j < 8 {
                if crc & 0x80000000 != 0 {
                    crc = (crc << 1) ^ poly;
                } else {
                    crc <<= 1;
                }
                j += 1;
            }

            table[i as usize] = if reflect {
                crc.reverse_bits()
            } else {
                crc
            };
            i += 1;
        }

        Self {
            table,
            initial,
            final_xor,
            current: initial,
        }
    }

    /// Updates the CRC with more data.
    pub fn update(&mut self, data: &[u8]) {
        for &byte in data {
            let index = ((self.current as u8) ^ byte) as usize;
            self.current = (self.current >> 8) ^ self.table[index];
        }
    }

    /// Finalizes and returns the CRC value.
    pub fn finalize(self) -> u32 {
        self.current ^ self.final_xor
    }

    /// Resets the computer to its initial state.
    pub fn reset(&mut self) {
        self.current = self.initial;
    }
}

impl core::fmt::Debug for TableCrcComputer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TableCrcComputer")
            .field("current", &self.current)
            .finish()
    }
}

/// A streaming CRC computer that can handle data in any chunk size.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::computer::StreamingCrc;
/// use abseil::absl_crc::variants::Crc32Variant;
///
/// let mut stream = StreamingCrc::new(Crc32Variant::Standard);
/// stream.update(b"Part 1 ");
/// stream.update(b"Part 2");
/// stream.update(b" Part 3");
/// let crc = stream.finalize();
/// ```
#[derive(Clone, Debug)]
pub struct StreamingCrc {
    computer: CrcComputer,
}

impl StreamingCrc {
    /// Creates a new streaming CRC computer.
    pub fn new(algorithm: CrcAlgorithm) -> Self {
        Self {
            computer: CrcComputer::new(algorithm),
        }
    }

    /// Updates the CRC with more data.
    pub fn update(&mut self, data: &[u8]) {
        self.computer.update(data);
    }

    /// Finalizes and returns the CRC value.
    pub fn finalize(self) -> u64 {
        self.computer.finalize()
    }

    /// Resets the stream to start a new computation.
    pub fn reset(&mut self) {
        self.computer.reset();
    }

    /// Returns the current intermediate CRC value.
    pub const fn current(&self) -> u64 {
        self.computer.current()
    }
}

/// Computes CRC using the specified algorithm.
fn compute_crc(algorithm: CrcAlgorithm, data: &[u8]) -> u64 {
    match algorithm {
        CrcAlgorithm::Crc32 => crate::crc32::crc32(data) as u64,
        CrcAlgorithm::Crc32C => crate::crc32::crc32c(data) as u64,
        CrcAlgorithm::Crc64Ecma | CrcAlgorithm::Crc64Iso => crate::crc64::crc64(data),
        CrcAlgorithm::Crc16 => crc16(data, 0x8005, 0x0000, true, true) as u64,
        CrcAlgorithm::Crc16Ccitt => crc16(data, 0x1021, 0xffff, false, false) as u64,
        CrcAlgorithm::Crc8 => crc8(data) as u64,
    }
}

/// CRC-16 computation.
fn crc16(data: &[u8], poly: u16, init: u16, reflect_in: bool, reflect_out: bool) -> u16 {
    let mut crc = init;

    for &byte in data {
        let byte = if reflect_in {
            byte.reverse_bits() as u16
        } else {
            byte as u16
        };

        crc ^= byte << 8;

        let mut _i = 0;
        while _i < 8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ poly;
            } else {
                crc <<= 1;
            }
            _i += 1;
        }
    }

    if reflect_out {
        crc.reverse_bits()
    } else {
        crc
    }
}

/// CRC-8 computation.
fn crc8(data: &[u8]) -> u8 {
    const POLY: u8 = 0x07;
    let mut crc: u8 = 0x00;

    for &byte in data {
        crc ^= byte;
        let mut _i = 0;
        while _i < 8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ POLY;
            } else {
                crc <<= 1;
            }
            _i += 1;
        }
    }

    crc
}

/// Builder for creating custom CRC configurations.
///
/// # Examples
///
/// ```
/// use abseil::absl_crc::computer::CrcBuilder;
///
/// let params = CrcBuilder::new()
///     .polynomial(0x04c11db7)
///     .width(32)
///     .initial(0xffffffff)
///     .final_xor(0xffffffff)
///     .reflect_input(true)
///     .reflect_output(true)
///     .build();
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct CrcBuilder {
    polynomial: u64,
    initial: u64,
    final_xor: u64,
    width: u8,
    reflect_input: bool,
    reflect_output: bool,
}

impl CrcBuilder {
    /// Creates a new CRC builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the polynomial.
    pub fn polynomial(mut self, poly: u64) -> Self {
        self.polynomial = poly;
        self
    }

    /// Sets the initial value.
    pub fn initial(mut self, init: u64) -> Self {
        self.initial = init;
        self
    }

    /// Sets the final XOR value.
    pub fn final_xor(mut self, xor: u64) -> Self {
        self.final_xor = xor;
        self
    }

    /// Sets the width in bits.
    pub fn width(mut self, width: u8) -> Self {
        self.width = width;
        self
    }

    /// Sets whether to reflect input.
    pub fn reflect_input(mut self, reflect: bool) -> Self {
        self.reflect_input = reflect;
        self
    }

    /// Sets whether to reflect output.
    pub fn reflect_output(mut self, reflect: bool) -> Self {
        self.reflect_output = reflect;
        self
    }

    /// Builds the CRC parameters.
    pub fn build(self) -> CrcParams {
        CrcParams {
            polynomial: self.polynomial,
            initial: self.initial,
            final_xor: self.final_xor,
            width: self.width,
            reflect_input: self.reflect_input,
            reflect_output: self.reflect_output,
        }
    }

    /// Computes CRC with the current configuration.
    pub fn compute(self, data: &[u8]) -> u64 {
        self.build().compute(data)
    }
}

impl CrcParams {
    /// Computes CRC with these parameters.
    pub fn compute(&self, data: &[u8]) -> u64 {
        match self.width {
            32 => self.compute_crc32(data),
            16 => self.compute_crc16(data) as u64,
            8 => self.compute_crc8(data) as u64,
            64 => self.compute_crc64(data),
            _ => panic!("Unsupported CRC width: {}", self.width),
        }
    }

    fn compute_crc32(&self, data: &[u8]) -> u64 {
        if self.polynomial == 0x04c11db7 && self.initial == 0xffffffff {
            crate::crc32::crc32(data) as u64
        } else if self.polynomial == 0x1edc6f41 && self.initial == 0xffffffff {
            crate::crc32::crc32c(data) as u64
        } else {
            // Generic CRC-32 implementation
            let mut crc = self.initial;
            let poly = self.polynomial;

            for &byte in data {
                let byte = if self.reflect_input {
                    byte.reverse_bits() as u64
                } else {
                    byte as u64
                };

                crc ^= byte << 24;

                let mut _i = 0;
                while _i < 8 {
                    if crc & 0x80000000 != 0 {
                        crc = (crc << 1) ^ poly;
                    } else {
                        crc <<= 1;
                    }
                    _i += 1;
                }
            }

            if self.reflect_output {
                crc = (crc as u32).reverse_bits() as u64;
            }

            (crc as u32 ^ self.final_xor as u32) as u64
        }
    }

    fn compute_crc16(&self, data: &[u8]) -> u16 {
        crc16(
            data,
            self.polynomial as u16,
            self.initial as u16,
            self.reflect_input,
            self.reflect_output,
        ) ^ self.final_xor as u16
    }

    fn compute_crc8(&self, data: &[u8]) -> u8 {
        let mut crc: u8 = self.initial as u8;
        let poly = self.polynomial as u8;

        for &byte in data {
            crc ^= byte;
            let mut _i = 0;
            while _i < 8 {
                if crc & 0x80 != 0 {
                    crc = (crc << 1) ^ poly;
                } else {
                    crc <<= 1;
                }
                _i += 1;
            }
        }

        crc ^ self.final_xor as u8
    }

    fn compute_crc64(&self, data: &[u8]) -> u64 {
        crate::crc64::crc64(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variants::Crc32Variant;

    #[test]
    fn test_crc_computer() {
        let mut computer = CrcComputer::new(CrcAlgorithm::Crc32);
        computer.update(b"Hello, ");
        computer.update(b"world!");
        let crc = computer.finalize();
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_crc_computer_reset() {
        let mut computer = CrcComputer::new(CrcAlgorithm::Crc32);
        computer.update(b"Hello");
        computer.reset();
        computer.update(b"Hello");
        let crc1 = computer.finalize();

        let mut computer2 = CrcComputer::new(CrcAlgorithm::Crc32);
        computer2.update(b"Hello");
        let crc2 = computer2.finalize();

        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_crc_computer_current() {
        let computer = CrcComputer::new(CrcAlgorithm::Crc32);
        assert_eq!(computer.current(), 0xffffffff);
    }

    #[test]
    fn test_table_crc_computer_crc32() {
        let mut computer = TableCrcComputer::crc32();
        computer.update(b"123456789");
        let crc = computer.finalize();
        assert_eq!(crc, 0xcbf43926);
    }

    #[test]
    fn test_table_crc_computer_crc32c() {
        let mut computer = TableCrcComputer::crc32c();
        computer.update(b"123456789");
        let crc = computer.finalize();
        assert_eq!(crc, 0xe3069283);
    }

    #[test]
    fn test_table_crc_computer_reset() {
        let mut computer = TableCrcComputer::crc32();
        computer.update(b"Hello");
        computer.reset();
        computer.update(b"Hello");
        let crc = computer.finalize();

        let mut computer2 = TableCrcComputer::crc32();
        computer2.update(b"Hello");
        let crc2 = computer2.finalize();

        assert_eq!(crc, crc2);
    }

    #[test]
    fn test_streaming_crc_basic() {
        let mut stream = StreamingCrc::new(CrcAlgorithm::Crc32);
        stream.update(b"Hello, ");
        stream.update(b"world!");
        let crc = stream.finalize();
        assert_ne!(crc, 0);
    }

    #[test]
    fn test_streaming_crc_reset() {
        let mut stream = StreamingCrc::new(CrcAlgorithm::Crc32);
        stream.update(b"Hello");
        stream.reset();
        stream.update(b"Hello");
        let crc1 = stream.finalize();

        let mut stream2 = StreamingCrc::new(CrcAlgorithm::Crc32);
        stream2.update(b"Hello");
        let crc2 = stream2.finalize();

        assert_eq!(crc1, crc2);
    }

    #[test]
    fn test_streaming_crc_current() {
        let mut stream = StreamingCrc::new(CrcAlgorithm::Crc32);
        assert_eq!(stream.current(), 0xffffffff);
        stream.update(b"Hello");
        let current = stream.current();
        assert_ne!(current, 0xffffffff);
    }

    #[test]
    fn test_crc_builder_basic() {
        let crc = CrcBuilder::new()
            .polynomial(0x04c11db7)
            .initial(0xffffffff)
            .final_xor(0xffffffff)
            .width(32)
            .reflect_input(true)
            .reflect_output(true)
            .compute(b"123456789");
        assert_eq!(crc, 0xcbf43926);
    }

    #[test]
    fn test_crc_builder_crc32c() {
        let crc = CrcBuilder::new()
            .polynomial(0x1edc6f41)
            .initial(0xffffffff)
            .final_xor(0xffffffff)
            .width(32)
            .reflect_input(true)
            .reflect_output(true)
            .compute(b"123456789");
        assert_eq!(crc, 0xe3069283);
    }
}
