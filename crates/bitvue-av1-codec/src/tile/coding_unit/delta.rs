//! Per-superblock side information read right after `skip`/`segment_id`, before any mode info:
//! `cdef_idx()` (spec 5.11.56) and the `delta_q`/`delta_lf` pair (spec 5.11.38). The real spec
//! (and dav1d's `decode_b`) reads these ahead of `intra_frame_mode_info()`/
//! `inter_frame_mode_info()`; reading them anywhere later desyncs the shared `SymbolDecoder`.

use crate::symbol::SymbolDecoder;
use crate::tile::{FrameCodingParams, MiRect, SuperblockCtx};
use bitvue_engine::Result;

/// `cdef_idx()` -- previously never read at all in this crate. Gated only on `!skip` (NOT on
/// whether CDEF is enabled: `cdef_bits` is already `0` then, making the read a no-op), and read at
/// most once per 64x64 CDEF unit within the superblock; `sb.cdef_idx` (`-1` = "not yet read",
/// reset per superblock) tracks that, mirroring dav1d's `cur_sb_cdef_idx_ptr`. A 128x128
/// superblock has 4 units (2x2) addressed by `idx`; a 64x64 one always uses unit `0` (NOT simply
/// `x4 & 16`, which would stay non-zero across successive 64-superblocks at frame-absolute MI
/// coordinates). A block spanning several units propagates its single value to all of them.
pub(super) fn read_cdef_idx(
    decoder: &mut SymbolDecoder,
    sb: &mut SuperblockCtx,
    mi: MiRect,
    skip: bool,
    cdef_bits: u8,
) -> Result<()> {
    if skip {
        return Ok(());
    }
    let idx = if sb.size4 == 32 {
        ((mi.x4 & 16) >> 4) + ((mi.y4 & 16) >> 3)
    } else {
        0
    } as usize;
    if sb.cdef_idx[idx] == -1 {
        let v = decoder.read_bools_n(cdef_bits as u32)? as i8;
        sb.cdef_idx[idx] = v;
        if mi.width > 16 {
            sb.cdef_idx[idx + 1] = v;
        }
        if mi.height > 16 {
            sb.cdef_idx[idx + 2] = v;
        }
        if mi.width == 32 && mi.height == 32 {
            sb.cdef_idx[idx + 3] = v;
        }
    }
    Ok(())
}

/// `read_delta_qindex()` + `read_delta_lf()`. Returns the QP in effect for this block.
///
/// Real spec gate (dav1d's `decode_b`): only at the first leaf visited within each superblock
/// (`x4/y4 == sb.x4/sb.y4`, always the top-left-most leaf given AV1's partition decode order), and
/// -- when that leaf covers the *whole* superblock -- only when it isn't `skip` (a skipped
/// full-superblock block has nothing to dequantize, so the encoder never signals a delta for it).
/// `delta_lf` bits are nested inside that same gate: one component per plane when
/// `delta_lf_multi` (4 normally, 2 for monochrome), otherwise a single shared component.
pub(super) fn read_delta_q_lf(
    decoder: &mut SymbolDecoder,
    sb: &SuperblockCtx,
    mi: MiRect,
    skip: bool,
    frame: &FrameCodingParams,
    current_qp: i16,
) -> i16 {
    let is_first_cu_in_sb = mi.x4 == sb.x4 && mi.y4 == sb.y4;
    let is_full_sb_size = mi.width == sb.size4 && mi.height == sb.size4;
    let have_delta = frame.delta_q_enabled && is_first_cu_in_sb && (!is_full_sb_size || !skip);
    if !have_delta {
        return current_qp;
    }
    let (x, y) = (mi.x4 * 4, mi.y4 * 4);

    let new_qp = match decoder.read_delta_q() {
        Ok(delta_q) => {
            let qp = (current_qp + delta_q).clamp(0, 255);
            tracing::debug!(
                "Delta Q applied at ({}, {}): {} + {} = {}",
                x,
                y,
                current_qp,
                delta_q,
                qp
            );
            qp
        }
        Err(e) => {
            tracing::warn!(
                "Failed to read delta Q at ({}, {}): {}, using current QP",
                x,
                y,
                e
            );
            current_qp
        }
    };

    if frame.delta_lf_present {
        let n_lfs = if frame.delta_lf_multi {
            if frame.tx_type_flags.mono_chrome {
                2
            } else {
                4
            }
        } else {
            1
        };
        for i in 0..n_lfs {
            let cdf_index = if frame.delta_lf_multi { i + 1 } else { 0 };
            if let Err(e) = decoder.read_delta_lf(cdf_index) {
                tracing::warn!("Failed to read delta_lf[{}] at ({}, {}): {}", i, x, y, e);
                break;
            }
        }
    }
    new_qp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mi(x4: u32, y4: u32, width: u32, height: u32) -> MiRect {
        MiRect {
            x4,
            y4,
            width,
            height,
        }
    }

    #[test]
    fn cdef_idx_is_read_once_and_spans_every_unit_the_block_covers() {
        let data = [0xA7u8; 64];
        let mut decoder = SymbolDecoder::new(&data).unwrap();
        let mut sb = SuperblockCtx::new(0, 0, 128);
        read_cdef_idx(&mut decoder, &mut sb, mi(0, 0, 32, 32), false, 2).unwrap();
        let v = sb.cdef_idx[0];
        assert!(v >= 0);
        assert_eq!(sb.cdef_idx, [v; 4], "128x128 block covers all four units");

        // Already read: a later block must not consume more symbols or overwrite the value.
        let before = sb.cdef_idx;
        read_cdef_idx(&mut decoder, &mut sb, mi(0, 0, 32, 32), false, 2).unwrap();
        assert_eq!(sb.cdef_idx, before);
    }

    #[test]
    fn cdef_idx_covers_two_units_for_128x64_and_64x128() {
        let data = [0x5Cu8; 64];
        let mut sb = SuperblockCtx::new(0, 0, 128);
        read_cdef_idx(
            &mut SymbolDecoder::new(&data).unwrap(),
            &mut sb,
            mi(0, 0, 32, 16),
            false,
            2,
        )
        .unwrap();
        let v = sb.cdef_idx[0];
        assert_eq!(sb.cdef_idx, [v, v, -1, -1]);

        let mut sb = SuperblockCtx::new(0, 0, 128);
        read_cdef_idx(
            &mut SymbolDecoder::new(&data).unwrap(),
            &mut sb,
            mi(0, 0, 16, 32),
            false,
            2,
        )
        .unwrap();
        let v = sb.cdef_idx[0];
        assert_eq!(sb.cdef_idx, [v, -1, v, -1]);
    }

    #[test]
    fn cdef_idx_is_not_read_for_skipped_blocks() {
        let data = [0xA7u8; 64];
        let mut sb = SuperblockCtx::new(0, 0, 64);
        read_cdef_idx(
            &mut SymbolDecoder::new(&data).unwrap(),
            &mut sb,
            mi(0, 0, 16, 16),
            true,
            2,
        )
        .unwrap();
        assert_eq!(sb.cdef_idx, [-1; 4]);
    }
}
