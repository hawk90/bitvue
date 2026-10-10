//! CABAC engine and the CABAC slice-data macroblock walk.

use crate::slice::SliceType;

use super::types::*;

/// LPS range table (H.264 spec Table 9-35).
/// Indexed by [pStateIdx][qCodIRangeIdx], where qCodIRangeIdx = (codIRange >> 6) & 3.
#[rustfmt::skip]
pub(super) static RANGE_LPS: [[u8; 4]; 64] = [
    [128,176,208,240],[128,167,197,227],[128,158,187,216],[123,150,178,205],
    [116,142,169,195],[111,135,160,185],[105,128,152,175],[100,122,144,166],
    [ 95,116,137,158],[ 90,110,130,150],[ 85,104,123,142],[ 81, 99,117,135],
    [ 77, 94,111,128],[ 73, 89,105,122],[ 69, 85,100,116],[ 66, 80, 95,110],
    [ 62, 76, 90,104],[ 59, 72, 86, 99],[ 56, 69, 81, 94],[ 53, 65, 77, 89],
    [ 51, 62, 73, 85],[ 48, 59, 69, 80],[ 46, 56, 66, 76],[ 43, 53, 63, 72],
    [ 41, 50, 59, 69],[ 39, 48, 56, 65],[ 37, 45, 54, 62],[ 35, 43, 51, 59],
    [ 33, 41, 48, 56],[ 32, 39, 46, 53],[ 30, 37, 43, 50],[ 29, 35, 41, 48],
    [ 27, 33, 39, 45],[ 26, 31, 37, 43],[ 24, 30, 35, 41],[ 23, 28, 33, 39],
    [ 22, 27, 32, 37],[ 21, 26, 30, 35],[ 20, 24, 29, 33],[ 19, 23, 27, 31],
    [ 18, 22, 26, 30],[ 17, 21, 24, 28],[ 16, 20, 23, 26],[ 15, 19, 22, 25],
    [ 14, 18, 21, 24],[ 14, 17, 20, 23],[ 13, 16, 19, 22],[ 12, 15, 18, 21],
    [ 12, 14, 17, 20],[ 11, 14, 16, 19],[ 11, 13, 15, 18],[ 10, 12, 15, 17],
    [ 10, 12, 14, 16],[  9, 11, 13, 15],[  9, 11, 12, 14],[  8, 10, 12, 14],
    [  8,  9, 11, 13],[  7,  9, 11, 12],[  7,  9, 10, 12],[  7,  8, 10, 11],
    [  6,  8,  9, 11],[  6,  7,  9, 10],[  6,  7,  8,  9],[  2,  2,  2,  2],
];

/// MPS state transitions (H.264 spec Table 9-36).
#[rustfmt::skip]
pub(super) static TRANS_MPS: [u8; 64] = [
     1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,16,
    17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,
    33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,
    49,50,51,52,53,54,55,56,57,58,59,60,61,62,62,63,
];

/// LPS state transitions (H.264 spec Table 9-36).
#[rustfmt::skip]
pub(super) static TRANS_LPS: [u8; 64] = [
     0, 0, 1, 2, 2, 4, 4, 5, 6, 7, 8, 9, 9,11,11,12,
    13,13,15,15,16,16,18,18,19,19,21,21,22,22,23,24,
    24,25,26,26,27,27,28,29,29,30,30,31,32,32,33,33,
    34,34,35,35,36,36,37,37,38,38,39,39,40,40,41,41,
];

/// Compute (pStateIdx, MPS) for a CABAC context from H.264 Table 9-12.
/// `m` and `n` are the row-specific init params; `qp` is the slice QP (0–51).
pub(super) fn cabac_init_ctx(m: i32, n: i32, qp: i32) -> (u8, u8) {
    let pre = (((m * qp) >> 4) + n).clamp(1, 126);
    if pre <= 63 {
        ((63 - pre) as u8, 0)
    } else {
        ((pre - 64) as u8, 1)
    }
}

/// CABAC arithmetic decoder (H.264 spec Section 9.3).
pub(super) struct CabacDecoder<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: i8,      // 7 = MSB; exhausted when byte_pos >= data.len()
    cod_i_range: u32, // [256, 512)
    cod_i_offset: u32,
}

impl<'a> CabacDecoder<'a> {
    /// Initialize from slice payload bytes immediately after the slice header.
    fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let cod_i_offset = ((data[0] as u32) << 1) | ((data[1] >> 7) as u32);
        Some(CabacDecoder {
            data,
            byte_pos: 1,
            bit_pos: 6,
            cod_i_range: 510,
            cod_i_offset,
        })
    }

    fn read_raw_bit(&mut self) -> u32 {
        if self.byte_pos >= self.data.len() {
            return 0;
        }
        let bit = ((self.data[self.byte_pos] >> (self.bit_pos as u8)) & 1) as u32;
        self.bit_pos -= 1;
        if self.bit_pos < 0 {
            self.byte_pos += 1;
            self.bit_pos = 7;
        }
        bit
    }

    /// Decode one CABAC bin, updating the context (pStateIdx, MPS) in place.
    /// Returns `None` if the stream appears corrupt (sanity check failure).
    fn decode_bin(&mut self, p_state: &mut u8, mps: &mut u8) -> Option<u8> {
        let q = ((self.cod_i_range >> 6) & 3) as usize;
        let range_lps = RANGE_LPS[*p_state as usize][q] as u32;
        let range_mps = self.cod_i_range - range_lps;

        let bin;
        if self.cod_i_offset >= range_mps {
            bin = 1 - *mps;
            self.cod_i_offset -= range_mps;
            self.cod_i_range = range_lps;
            if *p_state == 0 {
                *mps ^= 1;
            }
            *p_state = TRANS_LPS[*p_state as usize];
        } else {
            bin = *mps;
            self.cod_i_range = range_mps;
            *p_state = TRANS_MPS[*p_state as usize];
        }

        while self.cod_i_range < 256 {
            self.cod_i_range <<= 1;
            self.cod_i_offset = (self.cod_i_offset << 1) | self.read_raw_bit();
        }
        if self.cod_i_offset >= self.cod_i_range {
            return None;
        }
        Some(bin)
    }

    /// Bypass decode — no context update (spec 9.3.3.2.3).
    fn decode_bypass(&mut self) -> Option<u8> {
        self.cod_i_offset = (self.cod_i_offset << 1) | self.read_raw_bit();
        if self.cod_i_offset >= self.cod_i_range {
            self.cod_i_offset -= self.cod_i_range;
            Some(1)
        } else {
            Some(0)
        }
    }

    /// Exp-Golomb order-k bypass decode (spec 9.3.3.2.4).
    fn decode_eg_bypass(&mut self, k: u32) -> Option<u32> {
        let mut num_zeros = 0u32;
        loop {
            let bit = self.decode_bypass()?;
            if bit == 1 {
                break;
            }
            num_zeros += 1;
            if num_zeros > 16 {
                return None;
            }
        }
        let suffix_len = num_zeros + k;
        let mut suffix = 0u32;
        for _ in 0..suffix_len {
            suffix = (suffix << 1) | self.decode_bypass()? as u32;
        }
        Some((1 << suffix_len) + suffix - 1)
    }

    /// Decode one signed MVD component.
    /// `ctx_g0`: context for abs_mvd_greater0_flag.
    /// `ctx_g1`: context for abs_mvd_greater1_flag.
    fn decode_mvd_component(
        &mut self,
        ctx_g0: &mut (u8, u8),
        ctx_g1: &mut (u8, u8),
    ) -> Option<i32> {
        let g0 = self.decode_bin(&mut ctx_g0.0, &mut ctx_g0.1)?;
        if g0 == 0 {
            return Some(0);
        }
        let g1 = self.decode_bin(&mut ctx_g1.0, &mut ctx_g1.1)?;
        let abs_val: i32 = if g1 == 0 {
            1
        } else {
            // abs_mvd_minus2: EG(0) via bypass → adds 2
            (self.decode_eg_bypass(0)? + 2) as i32
        };
        let sign = self.decode_bypass()?;
        Some(if sign == 1 { -abs_val } else { abs_val })
    }
}

/// Parse CABAC-coded P/B slice: decode mb_skip_flag and, for P_L0_16x16 MBs,
/// decode ref_idx and MVD.
///
/// Context states are initialized ONCE at slice start and persist across MBs,
/// matching the H.264 spec requirement (spec 9.3.2).
///
/// For non-P16x16 MBs (other mb_type values), CABAC decoding stops to avoid
/// context divergence; subsequent MBs are emitted as Inter with no MV.
pub(super) fn parse_cabac_slice_mbs(
    payload_after_header: &[u8],
    first_mb: u32,
    total_mbs: u32,
    pic_width: u32,
    slice_qp: i16,
    slice_type: SliceType,
    cabac_init_idc: u32,
) -> Vec<Macroblock> {
    let is_b = slice_type.is_b();
    let is_intra = slice_type.is_intra();
    let _ = cabac_init_idc; // reserved for multi-idc support

    // I slices have no skip flags.
    if is_intra {
        return (first_mb..total_mbs)
            .map(|mb_addr| Macroblock {
                mb_addr,
                x: (mb_addr % pic_width) * 16,
                y: (mb_addr / pic_width) * 16,
                mb_type: MbType::I16x16,
                skip: false,
                qp: slice_qp,
                mv_l0: None,
                mv_l1: None,
                ref_idx_l0: None,
                ref_idx_l1: None,
            })
            .collect();
    }

    let mb_type_non_skip = if is_b { MbType::B16x16 } else { MbType::PLuma };
    let skip_type = if is_b { MbType::BSkip } else { MbType::PSkip };

    let mut cabac = match CabacDecoder::new(payload_after_header) {
        Some(c) => c,
        None => {
            return (first_mb..total_mbs)
                .map(|mb_addr| Macroblock {
                    mb_addr,
                    x: (mb_addr % pic_width) * 16,
                    y: (mb_addr / pic_width) * 16,
                    mb_type: mb_type_non_skip,
                    skip: false,
                    qp: slice_qp,
                    mv_l0: None,
                    mv_l1: None,
                    ref_idx_l0: None,
                    ref_idx_l1: None,
                })
                .collect();
        }
    };

    let qp = slice_qp as i32;

    // Initialize CABAC contexts ONCE at slice start (spec 9.3.2).
    // P-slice mb_skip_flag: ctxIdx 11–13 (Table 9-12, init_idc=0)
    // B-slice mb_skip_flag: ctxIdx 24–26
    let mut skip_ctx: [(u8, u8); 3] = if is_b {
        [
            cabac_init_ctx(-3, 71, qp),
            cabac_init_ctx(-3, 70, qp),
            cabac_init_ctx(-3, 70, qp),
        ]
    } else {
        [
            cabac_init_ctx(0, 26, qp),
            cabac_init_ctx(0, 31, qp),
            cabac_init_ctx(0, 28, qp),
        ]
    };

    // P-slice mb_type first bin: ctxIdx 14 (Table 9-12, init_idc=0: m=25, n=34).
    // 1 → P_L0_16x16; 0 → other type.
    let mut mb_type_ctx: (u8, u8) = cabac_init_ctx(25, 34, qp);

    // B-slice mb_type: ctxIdx 27 bin0 (m=-3, n=68), ctxIdx 28 bin1 (m=0, n=60).
    // bin0=0 → B_Direct_16x16; bin0=1,bin1=0 → B_L0_16x16; bin0=1,bin1=1 → other.
    let mut b_mb_ctx_b0: (u8, u8) = cabac_init_ctx(-3, 68, qp);
    let mut b_mb_ctx_b1: (u8, u8) = cabac_init_ctx(0, 60, qp);

    // ref_idx_l0: ctxIdx 54 (P-slice, simplified ctxIdxInc=0, init_idc=0: m=5, n=10).
    let mut ref_idx_ctx: (u8, u8) = cabac_init_ctx(5, 10, qp);

    // abs_mvd_greater0_flag[x]: ctxIdx 40 (init_idc=0: m=26, n=13); [y]: ctxIdx 45 (same).
    // abs_mvd_greater1_flag[x]: ctxIdx 42 (m=28, n=13);             [y]: ctxIdx 47 (same).
    let mut mvd_g0: [(u8, u8); 2] = [cabac_init_ctx(26, 13, qp), cabac_init_ctx(26, 13, qp)];
    let mut mvd_g1: [(u8, u8); 2] = [cabac_init_ctx(28, 13, qp), cabac_init_ctx(28, 13, qp)];

    let mut mbs = Vec::with_capacity((total_mbs - first_mb) as usize);
    let mut was_skipped = vec![false; total_mbs as usize];
    let mut failed = false;

    for mb_addr in first_mb..total_mbs {
        if failed {
            mbs.push(Macroblock {
                mb_addr,
                x: (mb_addr % pic_width) * 16,
                y: (mb_addr / pic_width) * 16,
                mb_type: mb_type_non_skip,
                skip: false,
                qp: slice_qp,
                mv_l0: None,
                mv_l1: None,
                ref_idx_l0: None,
                ref_idx_l1: None,
            });
            continue;
        }

        // condTermFlag: 1 when neighbor exists AND was NOT skip.
        let cond_a = if mb_addr % pic_width > 0 && !was_skipped[(mb_addr - 1) as usize] {
            1usize
        } else {
            0
        };
        let cond_b = if mb_addr >= pic_width && !was_skipped[(mb_addr - pic_width) as usize] {
            1usize
        } else {
            0
        };
        let ctx_idx = (cond_a + cond_b).min(2);

        let skip = match cabac.decode_bin(&mut skip_ctx[ctx_idx].0, &mut skip_ctx[ctx_idx].1) {
            Some(1) => true,
            Some(_) => false,
            None => {
                failed = true;
                false
            }
        };

        was_skipped[mb_addr as usize] = skip;

        let (final_mb_type, mv) = if skip {
            (skip_type, None)
        } else if is_b {
            // B-slice: decode first two bins of mb_type (spec 9.3.2.5, Table 9-36).
            // bin0=0 → B_Direct_16x16 (no explicit MV); bin0=1,bin1=0 → B_L0_16x16.
            // bin0=1,bin1=1 → other types (B_L1, B_Bi, B8x8) — stop to avoid divergence.
            match cabac.decode_bin(&mut b_mb_ctx_b0.0, &mut b_mb_ctx_b0.1) {
                Some(0) => {
                    // B_Direct_16x16: spatial/temporal derived MV — no explicit MVD
                    (MbType::B16x16, None)
                }
                Some(1) => {
                    match cabac.decode_bin(&mut b_mb_ctx_b1.0, &mut b_mb_ctx_b1.1) {
                        Some(0) => {
                            // B_L0_16x16: decode ref_idx_l0 + MVD
                            let _ref_bit =
                                match cabac.decode_bin(&mut ref_idx_ctx.0, &mut ref_idx_ctx.1) {
                                    Some(v) => v,
                                    None => {
                                        failed = true;
                                        0
                                    }
                                };
                            if failed {
                                (mb_type_non_skip, None)
                            } else {
                                let mvd_x =
                                    cabac.decode_mvd_component(&mut mvd_g0[0], &mut mvd_g1[0]);
                                let mvd_y =
                                    cabac.decode_mvd_component(&mut mvd_g0[1], &mut mvd_g1[1]);
                                match (mvd_x, mvd_y) {
                                    (Some(dx), Some(dy)) => {
                                        (MbType::B16x16, Some(MotionVector::new(dx, dy)))
                                    }
                                    _ => {
                                        failed = true;
                                        (mb_type_non_skip, None)
                                    }
                                }
                            }
                        }
                        _ => {
                            // B_L1/B_Bi/B8x8 or decode failure: stop CABAC
                            failed = true;
                            (mb_type_non_skip, None)
                        }
                    }
                }
                _ => {
                    failed = true;
                    (mb_type_non_skip, None)
                }
            }
        } else {
            // P-slice: decode mb_type first bin to distinguish P_L0_16x16 from rest.
            // For P_L0_16x16 (first bin = 1 in H.264 binarization), decode ref_idx + MVD.
            // Note: H.264 P-slice mb_type binarization — bin=1 means P_L0_16x16.
            match cabac.decode_bin(&mut mb_type_ctx.0, &mut mb_type_ctx.1) {
                Some(1) => {
                    // P_L0_16x16: decode ref_idx then MVD.
                    // ref_idx: unary via CABAC — single bin (ctxIdx 54, assume ref_idx<2)
                    let _ref_bit = match cabac.decode_bin(&mut ref_idx_ctx.0, &mut ref_idx_ctx.1) {
                        Some(v) => v,
                        None => {
                            failed = true;
                            0
                        }
                    };
                    if failed {
                        (mb_type_non_skip, None)
                    } else {
                        let mvd_x = cabac.decode_mvd_component(&mut mvd_g0[0], &mut mvd_g1[0]);
                        let mvd_y = cabac.decode_mvd_component(&mut mvd_g0[1], &mut mvd_g1[1]);
                        match (mvd_x, mvd_y) {
                            (Some(dx), Some(dy)) => {
                                (MbType::PLuma, Some(MotionVector::new(dx, dy)))
                            }
                            _ => {
                                failed = true;
                                (mb_type_non_skip, None)
                            }
                        }
                    }
                }
                Some(_) => {
                    // Non-P16x16 mb_type: stop CABAC to avoid context divergence.
                    // Remaining MBs get Inter with no MV.
                    failed = true;
                    (mb_type_non_skip, None)
                }
                None => {
                    failed = true;
                    (mb_type_non_skip, None)
                }
            }
        };

        mbs.push(Macroblock {
            mb_addr,
            x: (mb_addr % pic_width) * 16,
            y: (mb_addr / pic_width) * 16,
            mb_type: final_mb_type,
            skip,
            qp: slice_qp,
            mv_l0: mv,
            mv_l1: None,
            ref_idx_l0: None,
            ref_idx_l1: None,
        });
    }

    mbs
}
