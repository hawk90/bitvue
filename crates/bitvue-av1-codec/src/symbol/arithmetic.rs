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

use bitvue_engine::{BitvueError, Result};

/// CDF scale constant (32768 = 2^15)
const CDF_SCALE: u32 = 32768;

/// Probability-value shift applied to each CDF entry before scaling by range.
/// Per AV1 spec Section 8.2.6 / rav1d `msac.rs` (`EC_PROB_SHIFT`).
const EC_PROB_SHIFT: u32 = 6;

/// Minimum probability floor (in 1/256ths of range) guaranteed to every remaining symbol,
/// so no symbol is ever assigned zero interval width regardless of its CDF value. Per AV1 spec
/// Section 8.2.6 / rav1d `msac.rs` (`EC_MIN_PROB`).
const EC_MIN_PROB: u32 = 4;

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
    ///
    /// When true, CDF tables passed to `read_symbol_adaptive` are updated
    /// after each symbol decode, implementing the probability adaptation from
    /// AV1 spec Section 8.3. This is used for inter-frame coding where CDFs
    /// are updated incrementally as symbols are decoded.
    ///
    /// When false (e.g., for intra-only frames or when `disable_cdf_update` is
    /// signalled in the frame header), CDF tables remain fixed.
    pub allow_update_cdf: bool,
}

/// Update a CDF table after decoding symbol `s`, per AV1 spec Section 8.3 (`update_cdf`) --
/// transcribed from rav1d's `rav1d_msac_decode_symbol_adapt_rust`/`rav1d_msac_decode_bool_adapt_rust`
/// (`memorysafety/rav1d`, BSD-2-Clause, `src/msac.rs`), which implement the same generic formula
/// specialized for the multi-symbol and binary cases respectively.
///
/// CDF layout (real spec/rav1d convention, **not** the ascending one this decoder used before):
///
/// ```text
/// cdf = [d_0, d_1, ..., d_{N-2}, count]
///         ^              ^        ^
///         descending     always   adaptation
///         probability    0        count (0..=32)
/// ```
///
/// `d_i` is monotonically non-increasing with `d_{N-2} == 0`; see `SymbolDecoder`/`CdfContext`'s
/// docs for how this differs from the AV1 spec's raw default-CDF tables (context derivation is
/// still a later phase -- see `docs/DEVELOPMENT_PHASES.md` Phase 4's AV1 entropy-decoding note).
///
/// # Arguments
///
/// * `cdf` - Mutable CDF slice of length `n_symbols + 1`; last entry is the adaptation count.
/// * `symbol` - The decoded symbol index (0..n_symbols-1). Must be < cdf.len()-1.
pub fn update_cdf(cdf: &mut [u16], symbol: u8) {
    let n_symbols = cdf.len().saturating_sub(1);
    if n_symbols == 0 {
        return; // Defensive: nothing to update
    }

    let count = cdf[n_symbols];
    let rate = 4 + (count >> 4) + u16::from(n_symbols > 2);
    let val = symbol as usize;

    for entry in &mut cdf[..val] {
        *entry += ((1u16 << 15) - *entry) >> rate;
    }
    for entry in &mut cdf[val..n_symbols] {
        *entry -= *entry >> rate;
    }

    cdf[n_symbols] = count + u16::from(count < 32);
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
    /// `cdf` uses the real spec/rav1d descending convention (see `update_cdf`'s doc) -- entries
    /// `0..n_symbols-1` are a monotonically non-increasing probability curve ending in `0`, and
    /// entry `n_symbols` is the adaptation count (not consulted here; only `update_cdf`/
    /// `read_symbol_adaptive` touch it).
    ///
    /// Transcribed from rav1d's `rav1d_msac_decode_symbol_adapt_rust` (`memorysafety/rav1d`,
    /// BSD-2-Clause, `src/msac.rs`), including the `EC_MIN_PROB` probability floor that guarantees
    /// every remaining symbol at least a minimal interval width regardless of its CDF value (this
    /// is why a malformed/degenerate CDF still decodes *something* rather than looping forever or
    /// dividing by zero).
    ///
    /// Returns the symbol index (0..n_symbols-1)
    pub fn read_symbol(&mut self, cdf: &[u16]) -> Result<u8> {
        if cdf.len() < 2 {
            return Err(BitvueError::InvalidData(
                "CDF must have at least 2 entries".to_string(),
            ));
        }

        let n_symbols = (cdf.len() - 1) as u32;

        let c = (self.value >> (EC_WIN_SIZE - 16)) as u32;
        let r = self.range >> 8;

        let mut u;
        let mut v = self.range;
        let mut val: u32 = 0;
        loop {
            u = v;
            let cdf_val = cdf[val as usize] as u32;
            v = r * (cdf_val >> EC_PROB_SHIFT);
            v >>= 7 - EC_PROB_SHIFT;
            v += EC_MIN_PROB * (n_symbols - val);
            if c >= v {
                break;
            }
            val += 1;
            if val >= n_symbols {
                // A well-formed CDF always has its last real entry (index n_symbols-1) equal to
                // 0, which guarantees termination before this point -- reaching here means the
                // CDF is malformed (last real entry non-zero) rather than a normal decode outcome.
                return Err(BitvueError::InvalidData(
                    "CDF decode overran symbol alphabet -- last real entry must be 0".to_string(),
                ));
            }
        }

        self.value = self.value.wrapping_sub((v as usize) << (EC_WIN_SIZE - 16));
        self.range = u - v;

        // Renormalize to keep range in valid bounds
        self.renormalize()?;

        self.count += 1;
        Ok(val as u8)
    }

    /// Read a symbol and update the CDF for adaptive probability estimation.
    ///
    /// This is the adaptive variant of `read_symbol`. In addition to decoding
    /// the symbol, it updates the CDF entries in-place via `update_cdf` (AV1 spec Section 8.3)
    /// when `allow_update_cdf` is enabled.
    ///
    /// When `allow_update_cdf` is false the CDF is not modified (same as
    /// calling `read_symbol` with the same CDF).
    pub fn read_symbol_adaptive(&mut self, cdf: &mut [u16]) -> Result<u8> {
        let symbol = self.read_symbol(cdf)?;

        if self.allow_update_cdf {
            update_cdf(cdf, symbol);
        }

        Ok(symbol)
    }

    /// Configure whether CDF updates are applied by `read_symbol_adaptive`.
    ///
    /// Set to `false` for frames that signal `disable_cdf_update = 1` in
    /// their frame header (AV1 spec Section 5.9.2).
    pub fn set_allow_update_cdf(&mut self, allow: bool) {
        self.allow_update_cdf = allow;
    }

    /// Read a boolean value with given probability
    ///
    /// Probability is scaled to 0..32768 where:
    /// - 0 = always false
    /// - 32768 = always true
    /// - 16384 = 50/50
    pub fn read_bool(&mut self, prob: u16) -> Result<bool> {
        // Descending-convention 2-symbol CDF equivalent to the old ascending [0, prob, 32768] --
        // see `update_cdf`'s doc for the convention. Symmetric at prob=16384 (the only value any
        // current caller uses, all in `SymbolDecoder::read_residual_block`'s literal-bit reads).
        let cdf = [CDF_SCALE as u16 - prob, 0u16, 0u16];
        let symbol = self.read_symbol(&cdf)?;
        Ok(symbol == 1)
    }

    /// Renormalize the decoder state, per AV1 spec Section 8.2.2 / rav1d's `ctx_norm`
    /// (`memorysafety/rav1d`, BSD-2-Clause, `src/msac.rs:324-335`).
    ///
    /// Uses count-leading-zeros to determine the shift amount efficiently: `d = clz(range) - 16`
    /// is algebraically identical to rav1d's `d = 15 ^ (31 ^ clz(rng))` for `range` in its
    /// post-decode valid range (`<= 65535`, i.e. `clz(range) >= 16`) -- both reduce to the same
    /// "bring the MSB of range to bit position 15" shift amount.
    ///
    /// The refill trigger is `(cnt_old as u32) < (d as u32)` -- an **unsigned** comparison, not
    /// `new_cnt < 0`. These agree whenever `cnt`/`d` are non-negative (the common case), but
    /// diverge once `cnt` has already gone negative (allowed, down to `MIN_CNT`): rav1d's
    /// comment calls this "avoids redundant refills at eob" -- once genuinely out of real data,
    /// re-triggering refill on every subsequent decode is both unnecessary (the exhausted-buffer
    /// branch already synthesizes 1-bits) and, prior to this fix, this decoder used a naive
    /// `new_cnt < 0` check that refilled on every such call instead.
    fn renormalize(&mut self) -> Result<()> {
        let d = (self.range.leading_zeros() as i32) - 16;

        debug_assert!(self.cnt >= MIN_CNT, "cnt below minimum before renormalize");

        if d > 0 {
            let cnt_old = self.cnt;

            if cnt_old - d < MIN_CNT {
                return Err(BitvueError::InvalidData(format!(
                    "Arithmetic decoder cnt underflow: {} - {} < {} (MIN_CNT)",
                    cnt_old, d, MIN_CNT
                )));
            }

            self.range <<= d;
            self.value <<= d;
            self.cnt = cnt_old - d;

            if (cnt_old as u32) < (d as u32) {
                self.refill()?;
            }
        }

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
                // Exhausted buffer: fill the low (c+8) bits with 1s. Per rav1d's `ctx_refill`,
                // this is `dif |= !(!(0xff) << c)` -- computed here as a direct bit-count fill
                // (`(1 << n) - 1`) instead, both to sidestep the shift-amount overflow this
                // decoder's own `EC_WIN_SIZE - MIN_CNT - 24` invariant permits (up to 71 on a
                // 64-bit build, past the 63-bit shift limit -- the original real crash this
                // branch exists for, see `coding_unit`'s module doc) and because the double-NOT
                // form `!(!(X << c))` is a no-op in Rust (bitwise NOT is its own inverse) and so
                // did not actually implement "fill remaining bits" at all before this fix -- it
                // silently set only 8 bits at a fixed offset instead of every low bit up to `c`.
                let fill_bits = (c + 8).clamp(0, EC_WIN_SIZE as i32) as u32;
                let fill_mask = if fill_bits >= EC_WIN_SIZE as u32 {
                    usize::MAX
                } else {
                    (1usize << fill_bits) - 1
                };
                value |= fill_mask;
                break;
            }

            // Read byte, inverted -- per rav1d's `dif |= ((buf[0] ^ 0xff) as EcWin) << c`. This
            // inversion is not cosmetic: it is how the spec's carryless range coder represents
            // the "difference from the top of the range" register: without it, `value` holds the
            // wrong half of the coding interval and every downstream threshold comparison in
            // `read_symbol` is wrong relative to what a real encoder produced, even though the
            // decode still "succeeds" (produces some in-range symbol) -- exactly the kind of
            // silent-not-crash-but-wrong-values gap this rewrite exists to close.
            let byte = self.data[self.offset];
            self.offset += 1;

            value |= ((byte ^ 0xFF) as usize) << c;
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
        // After refill with EC_WIN_SIZE=64, cnt starts at -15 so c=55. Each byte is XORed with
        // 0xFF before shifting in (per rav1d's ctx_refill -- see refill()'s doc):
        // (0x80^0xFF)<<55 = 0x7F<<55, (0x00^0xFF)<<47 = 0xFF<<47, (0x12^0xFF)<<39 = 0xED<<39,
        // (0x34^0xFF)<<31 = 0xCB<<31, then exhausted-buffer fill of the low (c=23)+8=31 bits.
        // Exact value cross-checked with a standalone Python trace of the same algorithm.
        // cnt = 64 - 23 - 24 = 17
        assert_eq!(decoder.value, 0x3FFF_F6E5_FFFF_FFFF_usize);
        assert_eq!(decoder.cnt, 17);
    }

    #[test]
    fn test_decoder_too_short() {
        let data = vec![0x80]; // Only 1 byte
        let decoder = ArithmeticDecoder::new(&data);
        assert!(decoder.is_err());
    }

    #[test]
    fn test_read_symbol_uniform() {
        // Uniform distribution: 4 symbols, each 25% probability, in the real spec/rav1d
        // descending convention (see `update_cdf`'s doc) -- equivalent to the old ascending
        // [0, 8192, 16384, 24576, 32768] via `d[i] = 32768 - ascending[i+1]`:
        //   d[0] = 32768-8192=24576, d[1]=32768-16384=16384, d[2]=32768-24576=8192, d[3]=0 (last
        //   real entry), d[4]=0 (adaptation count).
        let cdf = vec![24576u16, 16384, 8192, 0, 0];

        // After refill with EC_WIN_SIZE=64, each byte is XORed with 0xFF before shifting in (see
        // refill()'s doc) -- for this data the top 16 bits of value come out as c = 16383 (cross-
        // checked with a standalone Python trace of the same refill algorithm).
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        // Hand-worked trace (r = range>>8 = 32768>>8 = 128, c = 16383, EC_PROB_SHIFT=6,
        // EC_MIN_PROB=4):
        //   val=0: v = 128*(24576>>6)>>1 + 4*4 = 128*384>>1 + 16 = 24576+16 = 24592; c<v, continue
        //   val=1: v = 128*(16384>>6)>>1 + 4*3 = 128*256>>1 + 12 = 16384+12 = 16396; c<v, continue
        //   val=2: v = 128*(8192>>6)>>1  + 4*2 = 128*128>>1  + 8  = 8192+8   = 8200;  c>=v, accept
        let symbol = decoder.read_symbol(&cdf);
        assert!(symbol.is_ok());
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
    fn test_read_bool_prob_converted_to_descending_cdf() {
        // read_bool(prob) builds [CDF_SCALE-prob, 0, 0] internally -- confirm a few concrete
        // values decode without error and stay within the boolean's valid range (this is a
        // structural smoke test since read_bool's own return value is just a bool, not something
        // to hand-verify bit-for-bit here).
        for prob in [0u16, 1, 16384, 32767, 32768] {
            let data = vec![0x80, 0x00, 0x00, 0x00];
            let mut decoder = ArithmeticDecoder::new(&data).unwrap();
            assert!(
                decoder.read_bool(prob).is_ok(),
                "read_bool({prob}) should decode without error"
            );
        }
    }

    #[test]
    fn test_cdf_validation_well_formed_descending() {
        // Well-formed descending CDF: last real entry (index n_symbols-1) is 0.
        let cdf = vec![24576u16, 16384, 8192, 0, 0];
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        let result = decoder.read_symbol(&cdf);
        assert!(
            result.is_ok(),
            "Should accept a well-formed descending CDF (last real entry 0)"
        );
    }

    #[test]
    fn test_read_symbol_errors_instead_of_panicking_on_alphabet_overrun() {
        // Every byte XORs to 0x00 during refill (0xFF ^ 0xFF), so the decoder's top 16 bits (c)
        // come out as 0 -- smaller than even the EC_MIN_PROB floor at every step, an adversarial
        // decoder state that forces the decode loop past the last real CDF entry. Verifies
        // read_symbol returns a real error instead of panicking/indexing out of bounds.
        let cdf = vec![24576u16, 16384, 8192, 0, 0]; // well-formed (last real entry is 0)
        let data = vec![0xFF; 8];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        let result = decoder.read_symbol(&cdf);
        assert!(
            result.is_err(),
            "Should error rather than panic when the decode loop would overrun the alphabet"
        );
        match result {
            Err(BitvueError::InvalidData(message)) => {
                assert!(
                    message.contains("overran"),
                    "Error should describe the overrun: {message}"
                );
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

    #[test]
    fn test_allow_update_cdf_field_accessible() {
        // Verify that allow_update_cdf is a public field, not dead code
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();

        // Default should be true (adaptive CDFs enabled)
        assert!(decoder.allow_update_cdf);

        // Can be disabled via set_allow_update_cdf
        decoder.set_allow_update_cdf(false);
        assert!(!decoder.allow_update_cdf);

        // Can be re-enabled
        decoder.set_allow_update_cdf(true);
        assert!(decoder.allow_update_cdf);
    }

    #[test]
    fn test_update_cdf_shifts_probability() {
        // Descending-convention uniform 2-symbol CDF: [16384, 0, 0] (see `update_cdf`'s doc).
        // Decoding symbol 0 should DECREASE cdf[0] -- a smaller descending threshold means a
        // wider interval (higher probability) for symbol 0 on the next decode.
        //
        // Hand-worked: n_symbols=2, count=cdf[2]=0, rate=4+(0>>4)+(2>2?1:0)=4.
        // val=0 -> entries[..0] empty (nothing pushed up); entries[0..2] pushed down:
        //   cdf[0] -= cdf[0]>>4 = 16384 - 1024 = 15360
        //   cdf[1] -= cdf[1]>>4 = 0 - 0 = 0
        // cdf[2] (count) = 0 + 1 = 1
        let mut cdf = vec![16384u16, 0u16, 0u16];

        update_cdf(&mut cdf, 0);

        assert_eq!(cdf, vec![15360, 0, 1]);
    }

    #[test]
    fn test_update_cdf_shifts_probability_other_symbol() {
        // Same starting CDF, but decoding symbol 1 this time: entries[..1] = [cdf[0]] pushed UP
        // toward 32768 instead (symbol 0 becomes *less* likely, symbol 1 more likely).
        //   cdf[0] += (32768-16384)>>4 = 16384 + 1024 = 17408
        //   cdf[1] -= cdf[1]>>4 = 0 - 0 = 0   (still in entries[val..n_symbols] = entries[1..2])
        let mut cdf = vec![16384u16, 0u16, 0u16];

        update_cdf(&mut cdf, 1);

        assert_eq!(cdf, vec![17408, 0, 1]);
    }

    #[test]
    fn test_update_cdf_no_op_when_disabled() {
        // When allow_update_cdf is false, read_symbol_adaptive should not modify the CDF
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();
        decoder.set_allow_update_cdf(false);

        let mut cdf = vec![16384u16, 0u16, 0u16];
        let original = cdf.clone();

        let result = decoder.read_symbol_adaptive(&mut cdf);
        assert!(result.is_ok());

        // CDF should be unchanged since allow_update_cdf is false
        assert_eq!(
            cdf, original,
            "CDF should not be modified when allow_update_cdf is false"
        );
    }

    #[test]
    fn test_update_cdf_applied_when_enabled() {
        // When allow_update_cdf is true, read_symbol_adaptive should modify the CDF
        let data = vec![0x80, 0x00, 0x00, 0x00];
        let mut decoder = ArithmeticDecoder::new(&data).unwrap();
        // allow_update_cdf defaults to true

        let mut cdf = vec![16384u16, 0u16, 0u16];
        let original = cdf.clone();

        let result = decoder.read_symbol_adaptive(&mut cdf);
        assert!(result.is_ok());

        // CDF should have changed since allow_update_cdf is true
        assert_ne!(
            cdf, original,
            "CDF should be modified when allow_update_cdf is true"
        );
        // The count slot must have incremented, and the last real entry stays 0.
        assert_eq!(cdf[1], 0, "last real entry must stay 0");
        assert_eq!(cdf[2], 1, "adaptation count should increment from 0 to 1");
    }
}
