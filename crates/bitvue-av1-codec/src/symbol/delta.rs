//! `delta_q` and `delta_lf` values (spec 5.11.38/39).

use super::*;

impl<'a> SymbolDecoder<'a> {
    /// Read delta Q (quantization parameter delta)
    ///
    /// Per AV1 Spec Section 5.11.38 (Quantization Parameter Delta)
    ///
    /// Returns the delta Q value (can be positive or negative).
    /// Range: -MAX_DELTA_Q to +MAX_DELTA_Q where MAX_DELTA_Q = 63
    ///
    /// # Process
    /// 1. Read delta_q_abs using a variable-length code
    /// 2. If delta_q_abs > 0, read delta_q_sign_bit
    /// 3. Apply sign to get final delta Q value
    ///
    /// # Example
    /// ```ignore
    /// let delta_q = decoder.read_delta_q()?;
    /// // delta_q could be: 0, +1, -1, +2, -2, ..., +63, -63
    /// ```
    /// Real `read_delta_qindex()` (spec 5.11.38), ported exactly from dav1d's `decode_b`
    /// delta-q block (`src/decode.c`), not the spec pseudocode's abstraction alone: a 4-outcome
    /// adaptive symbol (`0..=2` used directly as the magnitude, `3` triggers a golomb-style
    /// variable-length extension -- a real 3-bit `n_bits` selector followed by `n_bits` raw
    /// equi-probable bits, NOT another adaptive symbol read), then -- only if the magnitude is
    /// nonzero -- a single real equi-probable (50/50) sign bit (`dav1d_msac_decode_bool_equi`,
    /// NOT an adaptive CDF, unlike `mv_sign`/`cfl_alpha`'s per-component signs). This crate's
    /// previous implementation substituted a hand-picked single-symbol CDF for the golomb
    /// extension and an adaptive CDF for the sign -- a real desync bug for any block whose delta
    /// magnitude reached the extension (common for real content with meaningful QP variation).
    pub fn read_delta_q(&mut self) -> Result<i16> {
        let cdf = self.cdf_context.get_delta_q_cdf_mut();
        let base = self.decoder.read_symbol_adaptive(cdf)?;
        let abs: i32 = if base < 3 {
            base as i32
        } else {
            let n_bits = 1 + self.read_bools_n(3)?;
            self.read_bools_n(n_bits)? as i32 + 1 + (1i32 << n_bits)
        };
        if abs == 0 {
            return Ok(0);
        }
        let negative = self.read_bool_equi()?;
        let delta_q = if negative { -abs } else { abs } as i16;
        tracing::debug!("Delta Q: {}", delta_q);
        Ok(delta_q)
    }

    /// Real `read_delta_lf()` (spec 5.11.38) -- same real golomb-extension + equi-probable-sign
    /// shape as `read_delta_q`'s doc, over the separate `delta_lf` CDF family. `cdf_index`:
    /// `0` for the single-component (`!delta_lf_multi`) case, `1..=4` for `delta_lf_multi`'s
    /// per-plane components (`crate::tile::coding_unit::parse_coding_unit`'s doc for the real
    /// `n_lfs`/index derivation).
    pub fn read_delta_lf(&mut self, cdf_index: usize) -> Result<i16> {
        let cdf = self.cdf_context.get_delta_lf_cdf_mut(cdf_index);
        let base = self.decoder.read_symbol_adaptive(cdf)?;
        let abs: i32 = if base < 3 {
            base as i32
        } else {
            let n_bits = 1 + self.read_bools_n(3)?;
            self.read_bools_n(n_bits)? as i32 + 1 + (1i32 << n_bits)
        };
        if abs == 0 {
            return Ok(0);
        }
        let negative = self.read_bool_equi()?;
        Ok(if negative { -abs } else { abs } as i16)
    }
}
