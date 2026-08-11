//! Real AV1 coefficient scan order + `coeff_base`/`coeff_br` neighbor-context derivation (spec
//! 8.3.2's `get_coef_base_ctx`/`get_br_ctx`, dav1d's `get_lo_ctx`).
//!
//! Source: rav1d (`memorysafety/rav1d`, BSD-2-Clause) `src/scan.rs` (scan tables) and its C
//! predecessor `src/recon_tmpl.c`'s `get_lo_ctx`/`DECODE_COEFS_CLASS` (context formula --
//! verified by reading the C directly, not from memory, per this session's established
//! precedent). Only the **2D** (default) scan needs literal table data; `TX_CLASS_H`/`_V` compute
//! their coefficient-grid position from a closed-form row-major/column-major formula instead --
//! and for this crate's *square-transform-only* scope, that formula is bit-for-bit identical for
//! H and V (both reduce to `x = c % dim, y = c / dim`; the only place dav1d's H/V differ is how
//! they pack a coefficient *array index* nothing here ever reads), so no 3-way `TxClass` split is
//! needed -- the existing `is_1d: bool` (`SymbolDecoder::read_transform_type_is_1d`) is sufficient.
//!
//! Real AV1 caps residual *scanning* to the top-left 32x32 sub-block even for a 64x64 transform
//! (`slw = min(t_dim->lw, TX_32X32)` in dav1d) -- matches this crate's existing
//! `coeff_base_eob_context`/`eob_bin_16/64/256/1024` precedent of a 4-size-class cap
//! (4x4/8x8/16x16/32x32, `tx_size_class(..).min(3)`), reused here for the same reason.

/// AV1 spec's Default_Scan_4x4 -- decode-order index -> raster position (`x*dim+y`) within the
/// transform's coefficient grid. Source: rav1d `SCAN_4X4` (`src/scan.rs`).
const SCAN_4X4: [u16; 16] = [0, 4, 1, 2, 5, 8, 12, 9, 6, 3, 7, 10, 13, 14, 11, 15];
/// Default_Scan_8x8. Source: rav1d `SCAN_8X8`.
const SCAN_8X8: [u16; 64] = [
    0, 8, 1, 2, 9, 16, 24, 17, 10, 3, 4, 11, 18, 25, 32, 40, 33, 26, 19, 12, 5, 6, 13, 20, 27, 34,
    41, 48, 56, 49, 42, 35, 28, 21, 14, 7, 15, 22, 29, 36, 43, 50, 57, 58, 51, 44, 37, 30, 23, 31,
    38, 45, 52, 59, 60, 53, 46, 39, 47, 54, 61, 62, 55, 63,
];
/// Default_Scan_16x16. Source: rav1d `SCAN_16X16`.
const SCAN_16X16: [u16; 256] = [
    0, 16, 1, 2, 17, 32, 48, 33, 18, 3, 4, 19, 34, 49, 64, 80, 65, 50, 35, 20, 5, 6, 21, 36, 51,
    66, 81, 96, 112, 97, 82, 67, 52, 37, 22, 7, 8, 23, 38, 53, 68, 83, 98, 113, 128, 144, 129, 114,
    99, 84, 69, 54, 39, 24, 9, 10, 25, 40, 55, 70, 85, 100, 115, 130, 145, 160, 176, 161, 146, 131,
    116, 101, 86, 71, 56, 41, 26, 11, 12, 27, 42, 57, 72, 87, 102, 117, 132, 147, 162, 177, 192,
    208, 193, 178, 163, 148, 133, 118, 103, 88, 73, 58, 43, 28, 13, 14, 29, 44, 59, 74, 89, 104,
    119, 134, 149, 164, 179, 194, 209, 224, 240, 225, 210, 195, 180, 165, 150, 135, 120, 105, 90,
    75, 60, 45, 30, 15, 31, 46, 61, 76, 91, 106, 121, 136, 151, 166, 181, 196, 211, 226, 241, 242,
    227, 212, 197, 182, 167, 152, 137, 122, 107, 92, 77, 62, 47, 63, 78, 93, 108, 123, 138, 153,
    168, 183, 198, 213, 228, 243, 244, 229, 214, 199, 184, 169, 154, 139, 124, 109, 94, 79, 95,
    110, 125, 140, 155, 170, 185, 200, 215, 230, 245, 246, 231, 216, 201, 186, 171, 156, 141, 126,
    111, 127, 142, 157, 172, 187, 202, 217, 232, 247, 248, 233, 218, 203, 188, 173, 158, 143, 159,
    174, 189, 204, 219, 234, 249, 250, 235, 220, 205, 190, 175, 191, 206, 221, 236, 251, 252, 237,
    222, 207, 223, 238, 253, 254, 239, 255,
];
/// Default_Scan_32x32 (also used for 64x64 transforms, capped per this module's doc). Source:
/// rav1d `SCAN_32X32`.
const SCAN_32X32: [u16; 1024] = [
    0, 32, 1, 2, 33, 64, 96, 65, 34, 3, 4, 35, 66, 97, 128, 160, 129, 98, 67, 36, 5, 6, 37, 68, 99,
    130, 161, 192, 224, 193, 162, 131, 100, 69, 38, 7, 8, 39, 70, 101, 132, 163, 194, 225, 256,
    288, 257, 226, 195, 164, 133, 102, 71, 40, 9, 10, 41, 72, 103, 134, 165, 196, 227, 258, 289,
    320, 352, 321, 290, 259, 228, 197, 166, 135, 104, 73, 42, 11, 12, 43, 74, 105, 136, 167, 198,
    229, 260, 291, 322, 353, 384, 416, 385, 354, 323, 292, 261, 230, 199, 168, 137, 106, 75, 44,
    13, 14, 45, 76, 107, 138, 169, 200, 231, 262, 293, 324, 355, 386, 417, 448, 480, 449, 418, 387,
    356, 325, 294, 263, 232, 201, 170, 139, 108, 77, 46, 15, 16, 47, 78, 109, 140, 171, 202, 233,
    264, 295, 326, 357, 388, 419, 450, 481, 512, 544, 513, 482, 451, 420, 389, 358, 327, 296, 265,
    234, 203, 172, 141, 110, 79, 48, 17, 18, 49, 80, 111, 142, 173, 204, 235, 266, 297, 328, 359,
    390, 421, 452, 483, 514, 545, 576, 608, 577, 546, 515, 484, 453, 422, 391, 360, 329, 298, 267,
    236, 205, 174, 143, 112, 81, 50, 19, 20, 51, 82, 113, 144, 175, 206, 237, 268, 299, 330, 361,
    392, 423, 454, 485, 516, 547, 578, 609, 640, 672, 641, 610, 579, 548, 517, 486, 455, 424, 393,
    362, 331, 300, 269, 238, 207, 176, 145, 114, 83, 52, 21, 22, 53, 84, 115, 146, 177, 208, 239,
    270, 301, 332, 363, 394, 425, 456, 487, 518, 549, 580, 611, 642, 673, 704, 736, 705, 674, 643,
    612, 581, 550, 519, 488, 457, 426, 395, 364, 333, 302, 271, 240, 209, 178, 147, 116, 85, 54,
    23, 24, 55, 86, 117, 148, 179, 210, 241, 272, 303, 334, 365, 396, 427, 458, 489, 520, 551, 582,
    613, 644, 675, 706, 737, 768, 800, 769, 738, 707, 676, 645, 614, 583, 552, 521, 490, 459, 428,
    397, 366, 335, 304, 273, 242, 211, 180, 149, 118, 87, 56, 25, 26, 57, 88, 119, 150, 181, 212,
    243, 274, 305, 336, 367, 398, 429, 460, 491, 522, 553, 584, 615, 646, 677, 708, 739, 770, 801,
    832, 864, 833, 802, 771, 740, 709, 678, 647, 616, 585, 554, 523, 492, 461, 430, 399, 368, 337,
    306, 275, 244, 213, 182, 151, 120, 89, 58, 27, 28, 59, 90, 121, 152, 183, 214, 245, 276, 307,
    338, 369, 400, 431, 462, 493, 524, 555, 586, 617, 648, 679, 710, 741, 772, 803, 834, 865, 896,
    928, 897, 866, 835, 804, 773, 742, 711, 680, 649, 618, 587, 556, 525, 494, 463, 432, 401, 370,
    339, 308, 277, 246, 215, 184, 153, 122, 91, 60, 29, 30, 61, 92, 123, 154, 185, 216, 247, 278,
    309, 340, 371, 402, 433, 464, 495, 526, 557, 588, 619, 650, 681, 712, 743, 774, 805, 836, 867,
    898, 929, 960, 992, 961, 930, 899, 868, 837, 806, 775, 744, 713, 682, 651, 620, 589, 558, 527,
    496, 465, 434, 403, 372, 341, 310, 279, 248, 217, 186, 155, 124, 93, 62, 31, 63, 94, 125, 156,
    187, 218, 249, 280, 311, 342, 373, 404, 435, 466, 497, 528, 559, 590, 621, 652, 683, 714, 745,
    776, 807, 838, 869, 900, 931, 962, 993, 994, 963, 932, 901, 870, 839, 808, 777, 746, 715, 684,
    653, 622, 591, 560, 529, 498, 467, 436, 405, 374, 343, 312, 281, 250, 219, 188, 157, 126, 95,
    127, 158, 189, 220, 251, 282, 313, 344, 375, 406, 437, 468, 499, 530, 561, 592, 623, 654, 685,
    716, 747, 778, 809, 840, 871, 902, 933, 964, 995, 996, 965, 934, 903, 872, 841, 810, 779, 748,
    717, 686, 655, 624, 593, 562, 531, 500, 469, 438, 407, 376, 345, 314, 283, 252, 221, 190, 159,
    191, 222, 253, 284, 315, 346, 377, 408, 439, 470, 501, 532, 563, 594, 625, 656, 687, 718, 749,
    780, 811, 842, 873, 904, 935, 966, 997, 998, 967, 936, 905, 874, 843, 812, 781, 750, 719, 688,
    657, 626, 595, 564, 533, 502, 471, 440, 409, 378, 347, 316, 285, 254, 223, 255, 286, 317, 348,
    379, 410, 441, 472, 503, 534, 565, 596, 627, 658, 689, 720, 751, 782, 813, 844, 875, 906, 937,
    968, 999, 1000, 969, 938, 907, 876, 845, 814, 783, 752, 721, 690, 659, 628, 597, 566, 535, 504,
    473, 442, 411, 380, 349, 318, 287, 319, 350, 381, 412, 443, 474, 505, 536, 567, 598, 629, 660,
    691, 722, 753, 784, 815, 846, 877, 908, 939, 970, 1001, 1002, 971, 940, 909, 878, 847, 816,
    785, 754, 723, 692, 661, 630, 599, 568, 537, 506, 475, 444, 413, 382, 351, 383, 414, 445, 476,
    507, 538, 569, 600, 631, 662, 693, 724, 755, 786, 817, 848, 879, 910, 941, 972, 1003, 1004,
    973, 942, 911, 880, 849, 818, 787, 756, 725, 694, 663, 632, 601, 570, 539, 508, 477, 446, 415,
    447, 478, 509, 540, 571, 602, 633, 664, 695, 726, 757, 788, 819, 850, 881, 912, 943, 974, 1005,
    1006, 975, 944, 913, 882, 851, 820, 789, 758, 727, 696, 665, 634, 603, 572, 541, 510, 479, 511,
    542, 573, 604, 635, 666, 697, 728, 759, 790, 821, 852, 883, 914, 945, 976, 1007, 1008, 977,
    946, 915, 884, 853, 822, 791, 760, 729, 698, 667, 636, 605, 574, 543, 575, 606, 637, 668, 699,
    730, 761, 792, 823, 854, 885, 916, 947, 978, 1009, 1010, 979, 948, 917, 886, 855, 824, 793,
    762, 731, 700, 669, 638, 607, 639, 670, 701, 732, 763, 794, 825, 856, 887, 918, 949, 980, 1011,
    1012, 981, 950, 919, 888, 857, 826, 795, 764, 733, 702, 671, 703, 734, 765, 796, 827, 858, 889,
    920, 951, 982, 1013, 1014, 983, 952, 921, 890, 859, 828, 797, 766, 735, 767, 798, 829, 860,
    891, 922, 953, 984, 1015, 1016, 985, 954, 923, 892, 861, 830, 799, 831, 862, 893, 924, 955,
    986, 1017, 1018, 987, 956, 925, 894, 863, 895, 926, 957, 988, 1019, 1020, 989, 958, 927, 959,
    990, 1021, 1022, 991, 1023,
];

/// `dav1d_lo_ctx_offsets[0]` (the `w == h` variant -- the only one this square-transform-only
/// crate ever needs, confirmed: `dav1d_lo_ctx_offsets[nonsquare_tx + (tx & nonsquare_tx)]` always
/// selects index `0` when `tx` is one of the square enum values, i.e. every tx size this crate
/// models). Indexed `[min(y,4)][min(x,4)]`. Source: rav1d `dav1d_lo_ctx_offsets` (`src/tables.c`).
const LO_CTX_OFFSETS: [[u8; 5]; 5] = [
    [0, 1, 6, 6, 21],
    [1, 6, 6, 21, 21],
    [6, 6, 21, 21, 21],
    [6, 21, 21, 21, 21],
    [21, 21, 21, 21, 21],
];

/// Coefficient-grid `(x, y)` position for decode-order index `c` (spec's scan-order traversal,
/// high-frequency to low-frequency -- matches `SymbolDecoder::read_residual_block`'s
/// `for c in (0..eob).rev()` loop directly, no off-by-one adjustment needed). `capped_class`:
/// 0..=3 (`tx_size_class(..).min(3)`, this module's doc). `is_1d`: `TX_CLASS_H`/`_V` use the
/// closed-form `x = c % dim, y = c / dim` (see this module's doc for why H and V don't need to be
/// distinguished here); `false` (2D) looks up the real scan table.
pub fn coeff_position(capped_class: usize, is_1d: bool, c: u32) -> (u32, u32) {
    let dim: u32 = 4 << capped_class;
    if is_1d {
        return (c % dim, c / dim);
    }
    let rc = match capped_class {
        0 => SCAN_4X4[c as usize],
        1 => SCAN_8X8[c as usize],
        2 => SCAN_16X16[c as usize],
        _ => SCAN_32X32[c as usize],
    } as u32;
    (rc / dim, rc % dim)
}

/// Per-transform-block coefficient-level scratch state (dav1d's `t->scratch.levels`, unpacked --
/// see this module's doc). Cleared per transform block (`LevelBuffer::new`); stores each decoded
/// position's token value scaled `tok * 65` when unextended, `tok + 192` when `coeff_br`-extended
/// -- matches dav1d's `tok * 0x41` / `tok + (3 << 6)` byte encoding exactly, since `lo_ctx`'s
/// magnitude sum treats the raw stored byte as the "how significant is this neighbor" signal, not
/// a literal level -- porting the encoding, not just the final decoded value, is required for the
/// context bucket math below to match real spec. Sized `(dim+2)^2` so neighbor reads past the
/// transform block's low-frequency edge (`x+1`/`x+2`/`y+1`/`y+2` beyond `dim-1`) read real zeros
/// without explicit bounds checks, matching dav1d's own padding.
pub struct LevelBuffer {
    data: Vec<u8>,
    dim: usize,
}

impl LevelBuffer {
    pub fn new(dim: usize) -> Self {
        Self {
            data: vec![0u8; (dim + 2) * (dim + 2)],
            dim,
        }
    }

    fn get(&self, x: u32, y: u32) -> u32 {
        let stride = self.dim + 2;
        self.data
            .get(x as usize * stride + y as usize)
            .copied()
            .unwrap_or(0) as u32
    }

    /// Record one decoded position's token, scaled per this struct's doc. `level`: the raw
    /// pre-golomb token/magnitude (`base_level`, or the post-`coeff_br`-extension value when
    /// `extended`).
    pub fn set(&mut self, x: u32, y: u32, extended: bool, level: u32) {
        let stride = self.dim + 2;
        let idx = x as usize * stride + y as usize;
        if let Some(slot) = self.data.get_mut(idx) {
            *slot = if extended {
                (level.min(63) + 192) as u8
            } else {
                (level.min(3) * 65) as u8
            };
        }
    }
}

/// `coeff_base`'s real context (return `.0`, `0..=25` for 2D / `26..=40` for H/V, matching
/// `CdfContext::base_tok`'s 41-context shape) and the 3-neighbor magnitude sum (return `.1`,
/// `coeff_br`'s `hi_mag` -- reused directly by callers computing `coeff_br`'s context, see
/// `SymbolDecoder::read_residual_block`'s doc) for the transform-block-local position `(x, y)`.
/// Per rav1d `get_lo_ctx` (`src/recon_tmpl.c`): sums the already-decoded neighbors at offsets
/// `(0,1)`, `(1,0)`, then (2D) `(1,1)` snapshotting `hi_mag` before adding `(0,2)`/`(2,0)`, or
/// (H/V) `(0,2)` snapshotting `hi_mag` before adding `(0,3)`/`(0,4)` -- these offsets are always
/// *higher*-frequency scan positions (already visited, since decode goes high-to-low frequency),
/// so causality holds without an explicit "already decoded" check.
pub fn lo_ctx(levels: &LevelBuffer, x: u32, y: u32, is_1d: bool) -> (u8, u32) {
    let mut mag = levels.get(x, y + 1) + levels.get(x + 1, y);
    let hi_mag;
    let offset;
    if is_1d {
        mag += levels.get(x, y + 2);
        hi_mag = mag;
        mag += levels.get(x, y + 3) + levels.get(x, y + 4);
        offset = 26 + if y > 1 { 10 } else { y * 5 };
    } else {
        mag += levels.get(x + 1, y + 1);
        hi_mag = mag;
        mag += levels.get(x, y + 2) + levels.get(x + 2, y);
        offset = LO_CTX_OFFSETS[y.min(4) as usize][x.min(4) as usize] as u32;
    }
    let ctx = offset + if mag > 512 { 4 } else { (mag + 64) >> 7 };
    (ctx as u8, hi_mag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coeff_position_2d_matches_known_scan_values() {
        assert_eq!(coeff_position(0, false, 0), (0, 0));
        assert_eq!(coeff_position(0, false, 1), (1, 0)); // SCAN_4X4[1] = 4 = 1*4+0
        assert_eq!(coeff_position(0, false, 4), (1, 1)); // SCAN_4X4[4] = 5 = 1*4+1
        assert_eq!(coeff_position(0, false, 15), (3, 3)); // SCAN_4X4[15] = 15 = 3*4+3
    }

    #[test]
    fn test_coeff_position_is_1d_is_row_major_identity() {
        assert_eq!(coeff_position(1, true, 0), (0, 0));
        assert_eq!(coeff_position(1, true, 1), (1, 0));
        assert_eq!(coeff_position(1, true, 8), (0, 1)); // dim=8, wraps to next row
        assert_eq!(coeff_position(1, true, 63), (7, 7));
    }

    #[test]
    fn test_coeff_position_all_positions_unique_and_in_range() {
        for capped_class in 0..4 {
            let dim = 4u32 << capped_class;
            let mut seen = std::collections::HashSet::new();
            for c in 0..(dim * dim) {
                let (x, y) = coeff_position(capped_class, false, c);
                assert!(x < dim && y < dim);
                assert!(seen.insert((x, y)), "duplicate position at c={c}");
            }
        }
    }

    #[test]
    fn test_level_buffer_defaults_to_zero() {
        let buf = LevelBuffer::new(4);
        assert_eq!(buf.get(0, 0), 0);
        assert_eq!(buf.get(3, 3), 0);
    }

    #[test]
    fn test_level_buffer_out_of_range_reads_zero() {
        let buf = LevelBuffer::new(4);
        assert_eq!(buf.get(10, 10), 0);
    }

    #[test]
    fn test_level_buffer_set_unextended_scales_by_65() {
        let mut buf = LevelBuffer::new(4);
        buf.set(1, 1, false, 2);
        assert_eq!(buf.get(1, 1), 130); // 2 * 65
    }

    #[test]
    fn test_level_buffer_set_extended_offsets_by_192() {
        let mut buf = LevelBuffer::new(4);
        buf.set(1, 1, true, 5);
        assert_eq!(buf.get(1, 1), 197); // 5 + 192
    }

    #[test]
    fn test_lo_ctx_no_neighbors_is_lowest_bucket() {
        let buf = LevelBuffer::new(8);
        let (ctx, hi_mag) = lo_ctx(&buf, 4, 4, false);
        assert_eq!(hi_mag, 0);
        // offset = LO_CTX_OFFSETS[4][4] = 21, mag bucket = (0+64)>>7 = 0
        assert_eq!(ctx, 21);
    }

    #[test]
    fn test_lo_ctx_2d_neighbors_raise_context_and_hi_mag() {
        let mut buf = LevelBuffer::new(8);
        buf.set(0, 1, false, 3); // offset (0,1) from origin
        buf.set(1, 0, false, 3); // offset (1,0) from origin
        buf.set(1, 1, false, 3); // offset (1,1) from origin
        let (ctx, hi_mag) = lo_ctx(&buf, 0, 0, false);
        assert_eq!(hi_mag, 3 * 65 * 3); // 3 neighbors, each scaled by 65
        assert!(ctx > 0);
    }

    #[test]
    fn test_lo_ctx_is_1d_uses_different_offset_formula() {
        let buf = LevelBuffer::new(16);
        let (ctx_y0, _) = lo_ctx(&buf, 0, 0, true);
        let (ctx_y1, _) = lo_ctx(&buf, 0, 1, true);
        let (ctx_y2, _) = lo_ctx(&buf, 0, 2, true);
        assert_eq!(ctx_y0, 26); // offset 26 + 0*5
        assert_eq!(ctx_y1, 31); // offset 26 + 1*5
        assert_eq!(ctx_y2, 36); // offset 26 + 10 (y > 1)
    }
}
