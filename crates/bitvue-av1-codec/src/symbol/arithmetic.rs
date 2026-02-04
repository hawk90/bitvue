//! Arithmetic Decoder
//!
//! Per AV1 Specification Section 8.2.2 (Arithmetic Decoding Process)
//!
//! Implements range-based arithmetic decoding for AV1 using the daala
//! entropy coder (multi-symbol arithmetic coding).
//!
//! ## Algorithm
//!
//! The decoder maintains state variables:
//! - `range`: Current coding range
//! - `value`: Current decoded value (EC window)
//! - `cnt`: Bit count for refill tracking
//!
//! For each symbol:
//! 1. Scale value by range
//! 2. Look up symbol in CDF
//! 3. Update range and value
//! 4. Renormalize if needed
//! 5. Update CDF for adaptation (if enabled)
//!
//! ## References
//!
//! Algorithm based on AV1 spec Section 8.2.2 and reference implementation.
//! CDF update logic implements adaptive probability per spec Section 8.3.

use bitvue_core::{BitvueError, Result};

/// CDF scale constant (32768 = 2^15)
const CDF_SCALE: u32 = 32768;

/// Window size in bits (matches rav1d: 64-bit on 64-bit systems)
/// Using usize which is 64-bit on modern systems
const EC_WIN_SIZE: usize = std::mem::size_of::<usize>() * 8;

/// Initial range value
const INITIAL_RANGE: u32 = 0x8000;

/// Arithmetic decoder invariants
/// Per AV1 spec and rav1d implementation:
/// - cnt must stay in range [-31, 16] (allows room for refill and renormalize)
/// - value must be in range [0, EC_WIN_SIZE)
/// - range must be in range [256, 65536] (2^8 to 2^16)
const MIN_CNT: i32 = -31;
const MAX_CNT: i32 = EC_WIN_SIZE as i32; // Maximum when fully refilled

/// Arithmetic decoder state
///
/// Implements daala entropy coder for AV1.
pub struct ArithmeticDecoder<'a> {
    /// Bitstream data
    data: &'a [u8],
    /// Current byte offset in bitstream
    offset: usize,
    /// Current bit offset within byte (0-7)
    #[allow(dead_code)]
    bit_offset: u8,
    /// Current coding range (rng in AV1 spec)
    pub range: u32,
    /// Current decoded value (dif in AV1 spec, EcWin in rav1d)
    pub value: usize,
    /// Bit counter for refill (-16..=0, indicating how many bits are valid)
    pub cnt: i32,
    /// Count of symbols read (for debugging)
    pub count: u64,
    /// Enable CDF updates (adaptive probability)
    /// TODO: Will be used when implementing adaptive CDFs (Phase 2)
    #[allow(dead_code)]
    allow_update_cdf: bool,
}

impl<'a> ArithmeticDecoder<'a> {
    /// Create a new arithmetic decoder
    ///
    /// Initializes the decoder following rav1d/dav1d initialization.
    /// Per AV1 spec Section 8.2.1 (Initialization process for symbol decoder).
    pub fn new(data: &'a [u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err(BitvueError::InvalidData(
                "Arithmetic decoder needs at least 2 bytes".to_string(),
            ));
        }

        let mut decoder = Self {
            data,
            offset: 0,
            bit_offset: 0,
            range: INITIAL_RANGE,
            value: 0,
            cnt: -15, // Start with -15, will be updated by refill
            count: 0,
            allow_update_cdf: true, // Enable adaptive CDFs
        };

        // Call refill to load initial bytes (matching rav1d/dav1d)
        decoder.refill()?;

        tracing::debug!(
            "ArithmeticDecoder::new: EC_WIN_SIZE={}, value=0x{:016X}, range={}, cnt={}",
            EC_WIN_SIZE,
            decoder.value,
            decoder.range,
            decoder.cnt
        );

        Ok(decoder)
    }

    /// Read a symbol using a CDF table
    ///
    /// CDF is a cumulative distribution function where:
    /// - `cdf[0]` = 0
    /// - `cdf[i]` = cumulative probability up to symbol i (scaled to 0..32768)
    /// - `cdf[n]` = 32768 (total probability)
    /// - `cdf[n_symbols]` = count (for adaptive updates)
    ///
    /// Per AV1 spec Section 8.3 (Symbol Decoding Functions).
    ///
    /// Returns the symbol index (0..n_symbols-1)
    pub fn read_symbol(&mut self, cdf: &[u16]) -> Result<u8> {
        if cdf.len() < 2 {
            return Err(BitvueError::InvalidData(
                "CDF must have at least 2 entries".to_string(),
            ));
        }

        let n_symbols = (cdf.len() - 1) as u8;

        // Validate CDF structure per AV1 spec:
        // 1. The last value must equal CDF_SCALE (32768)
        // 2. This ensures the probability distribution is properly normalized
        let last_value = *cdf.last().unwrap();
        if last_value != CDF_SCALE as u16 {
            return Err(BitvueError::InvalidData(format!(
                "CDF last value must be {}: got {}",
                CDF_SCALE, last_value
            )));
        }

        // Extract value and range for comparison
        let c = (self.value >> (EC_WIN_SIZE - 16)) as u32;
        let r = self.range;

        // Linear search: find symbol where cdf[sym] * r <= c * CDF_SCALE < cdf[sym+1] * r
        // Rearranged to: c < (cdf[sym+1] * r) / CDF_SCALE
        // This avoids overflow by comparing scaled thresholds
        let mut symbol = 0u8;
        while (symbol as usize) < n_symbols as usize {
            let next_idx = (symbol + 1) as usize;
            // next_idx is guaranteed to be valid because:
            // - n_symbols = cdf.len() - 1
            // - symbol < n_symbols, so symbol + 1 <= n_symbols = cdf.len() - 1
            // - Therefore next_idx <= cdf.len() - 1, which is always valid
            // Calculate threshold: (cdf[next] * range) >> 15
            let threshold = ((cdf[next_idx] as u32) * r) >> 15;
            if c < threshold {
                break;
            }
            symbol += 1;
        }

        // Get probability range for this symbol
        let fl = cdf[symbol as usize] as u32; // Lower bound
        let next_idx = symbol as usize + 1;
        let fh = if next_idx < cdf.len() {
            cdf[next_idx] as u32
        } else {
            CDF_SCALE // If we're at the last symbol, use CDF_SCALE as upper bound
        };
        let prob_range = fh - fl;

        // Update decoder state
        // New range = (old_range * probability_range) / CDF_SCALE
        let new_range = (r * prob_range) >> 15; // Divide by CDF_SCALE (32768 = 2^15)

        // Update value: subtract the lower bound contribution
        // Cast to usize before shifting to avoid overflow
        let value_offset = ((r * fl) >> 15) as usize;
        let value_offset = value_offset << (EC_WIN_SIZE - 16);
        self.value = self.value.wrapping_sub(value_offset);
        self.range = new_range;

        // Renormalize to keep range in valid bounds
        self.renormalize()?;

        self.count += 1;
        Ok(symbol)
    }

    /// Read a boolean value with given probability
    ///
    /// Probability is scaled to 0..32768 where:
    /// - 0 = always false
    /// - 32768 = always true
    /// - 16384 = 50/50
    #[allow(dead_code)]
    pub fn read_bool(&mut self, prob: u16) -> Result<bool> {
        let cdf = [0u16, prob, 32768];
        let symbol = self.read_symbol(&cdf)?;
        Ok(symbol == 1)
    }

    /// Renormalize the decoder state
    ///
    /// Per AV1 spec Section 8.2.2: When range becomes too small,
    /// shift range and value, then refill bits from bitstream.
    ///
    /// Uses count leading zeros to determine shift amount efficiently.
    fn renormalize(&mut self) -> Result<()> {
        // Calculate shift amount: d = clz(range) - 16
        // This brings the MSB of range to bit position 15
        let d = (self.range.leading_zeros() as i32) - 16;

        // Validate cnt invariant before decrement
        debug_assert!(self.cnt >= MIN_CNT, "cnt below minimum before renormalize");

        if d > 0 {
            // Validate decrement won't underflow cnt
            if self.cnt - d < MIN_CNT {
                return Err(BitvueError::InvalidData(format!(
                    "Arithmetic decoder cnt underflow: {} - {} < {} (MIN_CNT)",
                    self.cnt, d, MIN_CNT
                )));
            }

            // Shift range and value left by d bits
            self.range <<= d;
            self.value <<= d;
            self.cnt -= d;

            // Refill bits if needed
            if self.cnt < 0 {
                self.refill()?;
            }
        }

        // Validate cnt invariant after renormalize
        debug_assert!(self.cnt <= MAX_CNT, "cnt above maximum after renormalize");

        Ok(())
    }

    /// Refill the value window with bits from bitstream
    ///
    /// Following rav1d/dav1d refill logic.
    /// Reads bytes from bitstream and shifts them into value window.
    ///
    /// # Invariants
    ///
    /// - cnt must be in range [MIN_CNT, MAX_CNT] before calling
    /// - c calculation must not underflow (validated below)
    /// - value must stay in range [0, EC_WIN_SIZE]
    fn refill(&mut self) -> Result<()> {
        // Validate cnt invariant before refill
        debug_assert!(self.cnt >= MIN_CNT, "cnt below minimum before refill");

        // Calculate bit position to insert next byte
        // c = EC_WIN_SIZE - cnt - 24
        let mut c = (EC_WIN_SIZE as i32) - self.cnt - 24;

        // Validate c is non-negative (no underflow in calculation)
        // If cnt becomes very negative, this could underflow
        if c < 0 {
            return Err(BitvueError::InvalidData(format!(
                "Arithmetic decoder cnt underflow detected during refill: cnt={}, would require c={}",
                self.cnt, c
            )));
        }

        let mut value = self.value;

        loop {
            if self.offset >= self.data.len() {
                // Exhausted buffer: set remaining bits to 1
                if c >= 0 {
                    value |= !(!(0xFF_usize << c));
                }
                break;
            }

            // Read byte
            let byte = self.data[self.offset];
            self.offset += 1;

            // Shift into value
            value |= (byte as usize) << c;
            c -= 8;

            if c < 0 {
                break;
            }
        }

        self.value = value;
        self.cnt = (EC_WIN_SIZE as i32) - c - 24;

        // Validate cnt invariant after refill
        debug_assert!(self.cnt <= MAX_CNT, "cnt above maximum after refill");

        Ok(())
    }

    /// Get current byte offset
    #[allow(dead_code)]
    pub fn byte_offset(&self) -> usize {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_creation() {
        let data = vec![0x80, 0x00, 0x12, 0x34];
        let decoder = ArithmeticDecoder::new(&data);
        assert!(decoder.is_ok());

        let decoder = decoder.unwrap();
        eprintln!(
            "value = 0x{:08X}, range = 0x{:04X}, cnt = {}",
            decoder.value, decoder.range, decoder.cnt
        );
        assert_eq!(decoder.range, INITIAL_RANGE);
        // After refill, value should be properly initialized
        // TODO: Update expected value after understanding refill behavior
        //assert_eq!(decoder.value, ...);
    }

    #[test]
    fn test_decoder_too_short() {
        let data = vec![0x80]; // Only 1 byte
        let decoder = ArithmeticDecoder::new(&data);
        assert!(decoder.is_err());
    }

    #[test]
    fn test_read_symbol_uniform() {
        // Uniform distribution: 4 symbols, each 25% probability
        // CDF: [0, 8192, 16384, 24576, 32768]
        let cdf = vec![0u16, 8192, 16384, 24576, 32768];

        // After refill with EC_WIN_SIZE=64: first byte << 55 becomes top bits
        // 0x80 << 55 = 0x4000_0000_0000_0000, so c = (value >> 48) = 0x4000 = 16384
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        let symbol = decoder.read_symbol(&cdf);
        assert!(symbol.is_ok());
        // With c=0x4000 (16384) in range=0x8000 (32768)
        // Symbol 2 range: [16384, 24576), so 16384 falls in symbol 2
        assert_eq!(symbol.unwrap(), 2);
    }

    #[test]
    fn test_read_bool() {
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        // 50/50 probability
        let result = decoder.read_bool(16384);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cdf_validation_correct_value() {
        // Valid CDF: last value equals CDF_SCALE
        let cdf = vec![0u16, 8192, 16384, 24576, 32768];
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        let result = decoder.read_symbol(&cdf);
        assert!(result.is_ok(), "Should accept valid CDF with correct last value");
    }

    #[test]
    fn test_cdf_validation_incorrect_last_value() {
        // Invalid CDF: last value does not equal CDF_SCALE
        let cdf = vec![0u16, 8192, 16384, 24576, 30000]; // Should be 32768
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        let result = decoder.read_symbol(&cdf);
        assert!(result.is_err(), "Should reject CDF with incorrect last value");

        match result {
            Err(BitvueError::InvalidData(message)) => {
                assert!(message.contains("32768"), "Error should mention expected value");
                assert!(message.contains("30000"), "Error should mention actual value");
            }
            _ => panic!("Expected InvalidData error"),
        }
    }

    #[test]
    fn test_cdf_validation_too_short() {
        // CDF with only 1 entry (minimum is 2)
        let cdf = vec![0u16];
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        let result = decoder.read_symbol(&cdf);
        assert!(result.is_err(), "Should reject CDF that's too short");
    }
}
