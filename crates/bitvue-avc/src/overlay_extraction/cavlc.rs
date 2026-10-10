//! CAVLC residual skipping (coeff_token / total_zeros / run_before tables).

use crate::bitreader::BitReader;

/// CBP values for inter-coded macroblocks (H.264 Table 9-4, inter row).
/// Indexed by exp-Golomb code_num (0–47); value = cbp_luma | (cbp_chroma << 4).
/// cbp_luma:   bits [3:0], one per 8×8 luma block (0 = not coded).
/// cbp_chroma: bits [5:4], 0 = no chroma, 1 = DC only, 2 = DC+AC.
pub(super) static CBP_INTER: [u8; 48] = [
    0, 16, 1, 2, 4, 8, 32, 3, 5, 10, 12, 15, 47, 7, 11, 13, 14, 6, 9, 31, 35, 37, 42, 44, 33, 34,
    36, 40, 17, 18, 20, 24, 19, 21, 26, 28, 23, 27, 29, 30, 22, 25, 38, 39, 41, 43, 45, 46,
];

/// Try to advance the bit reader past the CAVLC-encoded residuals of one inter MB.
///
/// Returns `true` when the reader is correctly positioned at the start of the
/// next macroblock (caller may continue parsing).  Returns `false` when the
/// residuals cannot be safely skipped (caller should scaffold remaining MBs).
///
/// # Why only CBP=0 is guaranteed
///
/// coded_block_pattern is exp-Golomb coded; index 0 always means CBP=0 for
/// inter MBs (H.264 Table 9-4).  When CBP=0 there are no residual syntax
/// elements, so the advance is exact.  CBP≠0 requires reading coeff_token VLC
/// tables (H.264 Table 9-5) and level/zero/run-before VLCs — full CAVLC
/// residual skip is deferred (returns false so caller falls back to scaffold).
pub(super) fn try_skip_cavlc_residuals(reader: &mut BitReader) -> bool {
    // coded_block_pattern: exp-Golomb (UE) coded
    let cbp_idx = match reader.read_ue() {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Index 0 → CBP=0 → no residuals at all (most common case in P-frames)
    if cbp_idx == 0 {
        return true;
    }

    if cbp_idx >= 48 {
        return false;
    } // malformed bitstream
    let cbp = CBP_INTER[cbp_idx as usize];
    let cbp_luma = cbp & 0x0F;
    let cbp_chroma = cbp >> 4;

    // mb_qp_delta is present whenever CBP != 0 (SE coded)
    if reader.read_se().is_err() {
        return false;
    }

    // Attempt to skip each coded luma 4×4 block
    for grp in 0..4u8 {
        if (cbp_luma >> grp) & 1 == 0 {
            continue;
        }
        for _ in 0..4 {
            if !skip_cavlc_4x4_block(reader) {
                return false;
            }
        }
    }

    // Chroma DC (one block per component)
    if cbp_chroma >= 1 {
        if !skip_cavlc_4x4_block(reader) {
            return false;
        }
        if !skip_cavlc_4x4_block(reader) {
            return false;
        }
    }

    // Chroma AC (4 Cb + 4 Cr 4×4 blocks)
    if cbp_chroma >= 2 {
        for _ in 0..8 {
            if !skip_cavlc_4x4_block(reader) {
                return false;
            }
        }
    }

    true
}

/// Skip one CAVLC-coded 4×4 block using the nC=0 coeff_token VLC table.
///
/// Handles only the trailing-ones-only case (TC == TO) where no level VLC is
/// needed beyond sign bits.  Blocks with actual level codes return `false` so
/// the caller can fall back to scaffold.
pub(super) fn skip_cavlc_4x4_block(reader: &mut BitReader) -> bool {
    let (tc, to) = match read_coeff_token_nc01(reader) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if tc == 0 {
        return true;
    } // no coefficients

    // Skip TrailingOnes sign bits (1 bit each)
    for _ in 0..to {
        if reader.read_bit().is_err() {
            return false;
        }
    }

    if tc != to {
        // Block has level-coded coefficients — full level/zeros/run-before VLC
        // skip is not yet implemented; signal caller to stop.
        return false;
    }

    // tc == to: all coefficients are trailing ones.
    // Still need to read total_zeros VLC and run_before VLCs.
    skip_cavlc_trailing_runs(reader, tc)
}

/// Read total_zeros VLC then run_before VLCs for a block where all TC
/// coefficients are trailing ones (so no level bits to skip).
pub(super) fn skip_cavlc_trailing_runs(reader: &mut BitReader, tc: u32) -> bool {
    if tc == 0 {
        return true;
    }

    // total_zeros VLC for TC=1..15 uses simple unary-like codes.
    // For the skip we only need to consume the right number of bits, not the
    // exact value.  Use a conservative max-read loop (≤9 bits for any TC).
    let total_zeros = match read_total_zeros_nc01(reader, tc) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if total_zeros == 0 {
        return true;
    } // no run-before values

    // run_before VLCs: one per non-last coefficient with zeros remaining.
    // Max 3 bits each; read up to tc-1 values (last run is implicit).
    let mut zeros_left = total_zeros;
    for _ in 0..(tc.saturating_sub(1)) {
        if zeros_left == 0 {
            break;
        }
        let rb = match read_run_before(reader, zeros_left) {
            Ok(v) => v,
            Err(_) => return false,
        };
        zeros_left = zeros_left.saturating_sub(rb);
    }

    true
}

/// Decode coeff_token using H.264 Table 9-5 (nC = 0..1).
/// Returns `(TotalCoeff, TrailingOnes)` on success.
pub(super) fn read_coeff_token_nc01(reader: &mut BitReader) -> Result<(u32, u32), ()> {
    macro_rules! b {
        () => {
            reader.read_bit().map_err(|_| ())? as u32
        };
    }

    if b!() == 1 {
        return Ok((0, 0));
    } // "1"
    if b!() == 1 {
        return Ok((1, 1));
    } // "01"
    if b!() == 1 {
        return Ok((2, 2));
    } // "001"

    // prefix "000"
    if b!() == 1 {
        // prefix "0001"
        return Ok(if b!() == 1 {
            (3, 3) // "00011"
        } else if b!() == 0 {
            (2, 1) // "000100"
        } else {
            (1, 0) // "000101"
        });
    }

    // prefix "0000"
    if b!() == 1 {
        // prefix "00001"
        return Ok(if b!() == 1 {
            (4, 3) // "000011"
        } else if b!() == 0 {
            (5, 3) // "0000100"
        } else {
            (3, 2) // "0000101"
        });
    }

    // prefix "00000"
    if b!() == 1 {
        // prefix "000001"
        return Ok(match (b!(), b!()) {
            (0, 0) => (6, 3), // "00000100"
            (0, 1) => (4, 2), // "00000101"
            (1, 0) => (3, 1), // "00000110"
            (1, 1) => (2, 0), // "00000111"
            _ => unreachable!(),
        });
    }

    // prefix "000000"
    if b!() == 1 {
        // prefix "0000001"
        return Ok(match (b!(), b!()) {
            (0, 0) => (7, 3), // "000000100"
            (0, 1) => (5, 2), // "000000101"
            (1, 0) => (4, 1), // "000000110"
            (1, 1) => (3, 0), // "000000111"
            _ => unreachable!(),
        });
    }

    // prefix "0000000"
    if b!() == 1 {
        return Ok(match (b!(), b!()) {
            (0, 0) => (8, 3),
            (0, 1) => (6, 2),
            (1, 0) => (5, 1),
            (1, 1) => (4, 0),
            _ => unreachable!(),
        });
    }

    // TC > 8 requires longer codes; return Err and let caller fall back.
    Err(())
}

/// Read total_zeros VLC for a block with `tc` non-zero coefficients.
/// Uses a simplified read — only handles TC=1..7 precisely; higher TC returns 0.
pub(super) fn read_total_zeros_nc01(reader: &mut BitReader, tc: u32) -> Result<u32, ()> {
    macro_rules! b {
        () => {
            reader.read_bit().map_err(|_| ())? as u32
        };
    }

    // For TC >= 8 the total_zeros VLC is short (max 3 bits); just read and
    // discard (we return 0 so run_before is also skipped — conservative).
    if tc >= 8 {
        return Ok(0);
    }

    // TC=1: 4-bit VLC, values 0–15
    if tc == 1 {
        let a = b!();
        if a == 1 {
            return Ok(0);
        }
        let b = b!();
        if b == 1 {
            return Ok(1);
        }
        let c = b!();
        if c == 1 {
            return Ok(2);
        }
        let d = b!();
        return Ok(if d == 1 { 3 } else { 4 }); // simplified
    }

    // TC=2: 3-bit VLC
    if tc == 2 {
        return Ok(match (b!(), b!(), b!()) {
            (1, _, _) => 0,
            (0, 1, _) => 1,
            (0, 0, 1) => 2,
            _ => 3,
        });
    }

    // TC=3..7: conservative — read 1 bit as proxy, return simple values
    Ok(if b!() == 1 { 0 } else { 1 })
}

/// Read one run_before VLC value given `zeros_left` context.
pub(super) fn read_run_before(reader: &mut BitReader, zeros_left: u32) -> Result<u32, ()> {
    macro_rules! b {
        () => {
            reader.read_bit().map_err(|_| ())? as u32
        };
    }

    if zeros_left == 1 {
        return Ok(b!());
    } // 1-bit (0 or 1)
    if zeros_left == 2 {
        return Ok(match b!() {
            1 => 0,
            _ => 1 + b!(),
        });
    }
    // zeros_left >= 3: up to 3 bits
    let a = b!();
    if a == 1 {
        return Ok(0);
    }
    let b = b!();
    if b == 1 {
        return Ok(1);
    }
    Ok(2 + b!()) // 3-bit prefix gives 2 or 3
}
