//! Real AV1 coefficient scan order + `coeff_base`/`coeff_br` neighbor-context derivation (spec
//! 8.3.2's `get_coef_base_ctx`/`get_br_ctx`, dav1d's `get_lo_ctx`).
//!
//! Source: rav1d (`memorysafety/rav1d`, BSD-2-Clause) `src/scan.rs` (scan tables) and its C
//! predecessor `src/recon_tmpl.c`'s `get_lo_ctx`/`DECODE_COEFS_CLASS` (context formula --
//! verified by reading the C directly, not from memory, per this session's established
//! precedent). Only the **2D** (default) scan needs literal table data; `TX_CLASS_H`/`_V` compute
//! their coefficient-grid position from a closed-form row-major/column-major formula instead
//! (`x = c % dim, y = c / dim`, where `dim` is the transform's *height* -- H/V transforms only
//! ever arise for genuinely rectangular blocks in real AV1, but the position formula itself
//! doesn't depend on which axis is longer, only on `height_dim`), so no 3-way `TxClass` split is
//! needed for position -- the existing `is_1d: bool` (`SymbolDecoder::read_transform_type_is_1d`)
//! is sufficient. The **2D** neighbor-context *offset table* (`lo_ctx`'s 2D branch), unlike
//! position, does depend on aspect ratio for non-square transforms -- dav1d's real
//! `dav1d_lo_ctx_offsets[3][5][5]` has a square variant (`[0]`, this crate's original
//! `LO_CTX_OFFSETS`) plus two more for `width > height` (`[1]`) and `width < height` (`[2]`,
//! `src/tables.c`) -- both ported below and selected by `lo_ctx`'s new width/height params.
//!
//! Real AV1 caps residual *scanning* to the top-left 32x32 sub-block even for a 64x64-containing
//! transform (`slw = min(t_dim->lw, TX_32X32)` in dav1d) -- matches this crate's existing
//! `coeff_base_eob_context`/`eob_bin_16/64/256/1024` precedent of a size-class cap. For
//! rectangular transforms with a 64-sample axis (16x64/64x16/32x64/64x32), dav1d's own
//! `dav1d_scans` table (`src/scan.c`) reuses the *other* axis's already-32-or-smaller scan table
//! directly (e.g. `RTX_64X32` reuses `scan_32x32`, `RTX_16X64` reuses `scan_16x32`) rather than a
//! distinct 64-something table -- `scan_table`'s callers are expected to have already clamped
//! both dims to `<=32` (mirroring the existing `tx_size_class(..).min(3)`-style square cap) before
//! calling, so no separate 64-handling exists in this module.

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

const SCAN_4X8: [u16; 32] = [
    0, 8, 1, 16, 9, 2, 24, 17, 10, 3, 25, 18, 11, 4, 26, 19, 12, 5, 27, 20, 13, 6, 28, 21, 14, 7,
    29, 22, 15, 30, 23, 31,
];

const SCAN_8X4: [u16; 32] = [
    0, 1, 4, 2, 5, 8, 3, 6, 9, 12, 7, 10, 13, 16, 11, 14, 17, 20, 15, 18, 21, 24, 19, 22, 25, 28,
    23, 26, 29, 27, 30, 31,
];

const SCAN_4X16: [u16; 64] = [
    0, 16, 1, 32, 17, 2, 48, 33, 18, 3, 49, 34, 19, 4, 50, 35, 20, 5, 51, 36, 21, 6, 52, 37, 22, 7,
    53, 38, 23, 8, 54, 39, 24, 9, 55, 40, 25, 10, 56, 41, 26, 11, 57, 42, 27, 12, 58, 43, 28, 13,
    59, 44, 29, 14, 60, 45, 30, 15, 61, 46, 31, 62, 47, 63,
];

const SCAN_16X4: [u16; 64] = [
    0, 1, 4, 2, 5, 8, 3, 6, 9, 12, 7, 10, 13, 16, 11, 14, 17, 20, 15, 18, 21, 24, 19, 22, 25, 28,
    23, 26, 29, 32, 27, 30, 33, 36, 31, 34, 37, 40, 35, 38, 41, 44, 39, 42, 45, 48, 43, 46, 49, 52,
    47, 50, 53, 56, 51, 54, 57, 60, 55, 58, 61, 59, 62, 63,
];

const SCAN_8X16: [u16; 128] = [
    0, 16, 1, 32, 17, 2, 48, 33, 18, 3, 64, 49, 34, 19, 4, 80, 65, 50, 35, 20, 5, 96, 81, 66, 51,
    36, 21, 6, 112, 97, 82, 67, 52, 37, 22, 7, 113, 98, 83, 68, 53, 38, 23, 8, 114, 99, 84, 69, 54,
    39, 24, 9, 115, 100, 85, 70, 55, 40, 25, 10, 116, 101, 86, 71, 56, 41, 26, 11, 117, 102, 87,
    72, 57, 42, 27, 12, 118, 103, 88, 73, 58, 43, 28, 13, 119, 104, 89, 74, 59, 44, 29, 14, 120,
    105, 90, 75, 60, 45, 30, 15, 121, 106, 91, 76, 61, 46, 31, 122, 107, 92, 77, 62, 47, 123, 108,
    93, 78, 63, 124, 109, 94, 79, 125, 110, 95, 126, 111, 127,
];

const SCAN_16X8: [u16; 128] = [
    0, 1, 8, 2, 9, 16, 3, 10, 17, 24, 4, 11, 18, 25, 32, 5, 12, 19, 26, 33, 40, 6, 13, 20, 27, 34,
    41, 48, 7, 14, 21, 28, 35, 42, 49, 56, 15, 22, 29, 36, 43, 50, 57, 64, 23, 30, 37, 44, 51, 58,
    65, 72, 31, 38, 45, 52, 59, 66, 73, 80, 39, 46, 53, 60, 67, 74, 81, 88, 47, 54, 61, 68, 75, 82,
    89, 96, 55, 62, 69, 76, 83, 90, 97, 104, 63, 70, 77, 84, 91, 98, 105, 112, 71, 78, 85, 92, 99,
    106, 113, 120, 79, 86, 93, 100, 107, 114, 121, 87, 94, 101, 108, 115, 122, 95, 102, 109, 116,
    123, 103, 110, 117, 124, 111, 118, 125, 119, 126, 127,
];

const SCAN_8X32: [u16; 256] = [
    0, 32, 1, 64, 33, 2, 96, 65, 34, 3, 128, 97, 66, 35, 4, 160, 129, 98, 67, 36, 5, 192, 161, 130,
    99, 68, 37, 6, 224, 193, 162, 131, 100, 69, 38, 7, 225, 194, 163, 132, 101, 70, 39, 8, 226,
    195, 164, 133, 102, 71, 40, 9, 227, 196, 165, 134, 103, 72, 41, 10, 228, 197, 166, 135, 104,
    73, 42, 11, 229, 198, 167, 136, 105, 74, 43, 12, 230, 199, 168, 137, 106, 75, 44, 13, 231, 200,
    169, 138, 107, 76, 45, 14, 232, 201, 170, 139, 108, 77, 46, 15, 233, 202, 171, 140, 109, 78,
    47, 16, 234, 203, 172, 141, 110, 79, 48, 17, 235, 204, 173, 142, 111, 80, 49, 18, 236, 205,
    174, 143, 112, 81, 50, 19, 237, 206, 175, 144, 113, 82, 51, 20, 238, 207, 176, 145, 114, 83,
    52, 21, 239, 208, 177, 146, 115, 84, 53, 22, 240, 209, 178, 147, 116, 85, 54, 23, 241, 210,
    179, 148, 117, 86, 55, 24, 242, 211, 180, 149, 118, 87, 56, 25, 243, 212, 181, 150, 119, 88,
    57, 26, 244, 213, 182, 151, 120, 89, 58, 27, 245, 214, 183, 152, 121, 90, 59, 28, 246, 215,
    184, 153, 122, 91, 60, 29, 247, 216, 185, 154, 123, 92, 61, 30, 248, 217, 186, 155, 124, 93,
    62, 31, 249, 218, 187, 156, 125, 94, 63, 250, 219, 188, 157, 126, 95, 251, 220, 189, 158, 127,
    252, 221, 190, 159, 253, 222, 191, 254, 223, 255,
];

const SCAN_32X8: [u16; 256] = [
    0, 1, 8, 2, 9, 16, 3, 10, 17, 24, 4, 11, 18, 25, 32, 5, 12, 19, 26, 33, 40, 6, 13, 20, 27, 34,
    41, 48, 7, 14, 21, 28, 35, 42, 49, 56, 15, 22, 29, 36, 43, 50, 57, 64, 23, 30, 37, 44, 51, 58,
    65, 72, 31, 38, 45, 52, 59, 66, 73, 80, 39, 46, 53, 60, 67, 74, 81, 88, 47, 54, 61, 68, 75, 82,
    89, 96, 55, 62, 69, 76, 83, 90, 97, 104, 63, 70, 77, 84, 91, 98, 105, 112, 71, 78, 85, 92, 99,
    106, 113, 120, 79, 86, 93, 100, 107, 114, 121, 128, 87, 94, 101, 108, 115, 122, 129, 136, 95,
    102, 109, 116, 123, 130, 137, 144, 103, 110, 117, 124, 131, 138, 145, 152, 111, 118, 125, 132,
    139, 146, 153, 160, 119, 126, 133, 140, 147, 154, 161, 168, 127, 134, 141, 148, 155, 162, 169,
    176, 135, 142, 149, 156, 163, 170, 177, 184, 143, 150, 157, 164, 171, 178, 185, 192, 151, 158,
    165, 172, 179, 186, 193, 200, 159, 166, 173, 180, 187, 194, 201, 208, 167, 174, 181, 188, 195,
    202, 209, 216, 175, 182, 189, 196, 203, 210, 217, 224, 183, 190, 197, 204, 211, 218, 225, 232,
    191, 198, 205, 212, 219, 226, 233, 240, 199, 206, 213, 220, 227, 234, 241, 248, 207, 214, 221,
    228, 235, 242, 249, 215, 222, 229, 236, 243, 250, 223, 230, 237, 244, 251, 231, 238, 245, 252,
    239, 246, 253, 247, 254, 255,
];

const SCAN_16X32: [u16; 512] = [
    0, 32, 1, 64, 33, 2, 96, 65, 34, 3, 128, 97, 66, 35, 4, 160, 129, 98, 67, 36, 5, 192, 161, 130,
    99, 68, 37, 6, 224, 193, 162, 131, 100, 69, 38, 7, 256, 225, 194, 163, 132, 101, 70, 39, 8,
    288, 257, 226, 195, 164, 133, 102, 71, 40, 9, 320, 289, 258, 227, 196, 165, 134, 103, 72, 41,
    10, 352, 321, 290, 259, 228, 197, 166, 135, 104, 73, 42, 11, 384, 353, 322, 291, 260, 229, 198,
    167, 136, 105, 74, 43, 12, 416, 385, 354, 323, 292, 261, 230, 199, 168, 137, 106, 75, 44, 13,
    448, 417, 386, 355, 324, 293, 262, 231, 200, 169, 138, 107, 76, 45, 14, 480, 449, 418, 387,
    356, 325, 294, 263, 232, 201, 170, 139, 108, 77, 46, 15, 481, 450, 419, 388, 357, 326, 295,
    264, 233, 202, 171, 140, 109, 78, 47, 16, 482, 451, 420, 389, 358, 327, 296, 265, 234, 203,
    172, 141, 110, 79, 48, 17, 483, 452, 421, 390, 359, 328, 297, 266, 235, 204, 173, 142, 111, 80,
    49, 18, 484, 453, 422, 391, 360, 329, 298, 267, 236, 205, 174, 143, 112, 81, 50, 19, 485, 454,
    423, 392, 361, 330, 299, 268, 237, 206, 175, 144, 113, 82, 51, 20, 486, 455, 424, 393, 362,
    331, 300, 269, 238, 207, 176, 145, 114, 83, 52, 21, 487, 456, 425, 394, 363, 332, 301, 270,
    239, 208, 177, 146, 115, 84, 53, 22, 488, 457, 426, 395, 364, 333, 302, 271, 240, 209, 178,
    147, 116, 85, 54, 23, 489, 458, 427, 396, 365, 334, 303, 272, 241, 210, 179, 148, 117, 86, 55,
    24, 490, 459, 428, 397, 366, 335, 304, 273, 242, 211, 180, 149, 118, 87, 56, 25, 491, 460, 429,
    398, 367, 336, 305, 274, 243, 212, 181, 150, 119, 88, 57, 26, 492, 461, 430, 399, 368, 337,
    306, 275, 244, 213, 182, 151, 120, 89, 58, 27, 493, 462, 431, 400, 369, 338, 307, 276, 245,
    214, 183, 152, 121, 90, 59, 28, 494, 463, 432, 401, 370, 339, 308, 277, 246, 215, 184, 153,
    122, 91, 60, 29, 495, 464, 433, 402, 371, 340, 309, 278, 247, 216, 185, 154, 123, 92, 61, 30,
    496, 465, 434, 403, 372, 341, 310, 279, 248, 217, 186, 155, 124, 93, 62, 31, 497, 466, 435,
    404, 373, 342, 311, 280, 249, 218, 187, 156, 125, 94, 63, 498, 467, 436, 405, 374, 343, 312,
    281, 250, 219, 188, 157, 126, 95, 499, 468, 437, 406, 375, 344, 313, 282, 251, 220, 189, 158,
    127, 500, 469, 438, 407, 376, 345, 314, 283, 252, 221, 190, 159, 501, 470, 439, 408, 377, 346,
    315, 284, 253, 222, 191, 502, 471, 440, 409, 378, 347, 316, 285, 254, 223, 503, 472, 441, 410,
    379, 348, 317, 286, 255, 504, 473, 442, 411, 380, 349, 318, 287, 505, 474, 443, 412, 381, 350,
    319, 506, 475, 444, 413, 382, 351, 507, 476, 445, 414, 383, 508, 477, 446, 415, 509, 478, 447,
    510, 479, 511,
];

const SCAN_32X16: [u16; 512] = [
    0, 1, 16, 2, 17, 32, 3, 18, 33, 48, 4, 19, 34, 49, 64, 5, 20, 35, 50, 65, 80, 6, 21, 36, 51,
    66, 81, 96, 7, 22, 37, 52, 67, 82, 97, 112, 8, 23, 38, 53, 68, 83, 98, 113, 128, 9, 24, 39, 54,
    69, 84, 99, 114, 129, 144, 10, 25, 40, 55, 70, 85, 100, 115, 130, 145, 160, 11, 26, 41, 56, 71,
    86, 101, 116, 131, 146, 161, 176, 12, 27, 42, 57, 72, 87, 102, 117, 132, 147, 162, 177, 192,
    13, 28, 43, 58, 73, 88, 103, 118, 133, 148, 163, 178, 193, 208, 14, 29, 44, 59, 74, 89, 104,
    119, 134, 149, 164, 179, 194, 209, 224, 15, 30, 45, 60, 75, 90, 105, 120, 135, 150, 165, 180,
    195, 210, 225, 240, 31, 46, 61, 76, 91, 106, 121, 136, 151, 166, 181, 196, 211, 226, 241, 256,
    47, 62, 77, 92, 107, 122, 137, 152, 167, 182, 197, 212, 227, 242, 257, 272, 63, 78, 93, 108,
    123, 138, 153, 168, 183, 198, 213, 228, 243, 258, 273, 288, 79, 94, 109, 124, 139, 154, 169,
    184, 199, 214, 229, 244, 259, 274, 289, 304, 95, 110, 125, 140, 155, 170, 185, 200, 215, 230,
    245, 260, 275, 290, 305, 320, 111, 126, 141, 156, 171, 186, 201, 216, 231, 246, 261, 276, 291,
    306, 321, 336, 127, 142, 157, 172, 187, 202, 217, 232, 247, 262, 277, 292, 307, 322, 337, 352,
    143, 158, 173, 188, 203, 218, 233, 248, 263, 278, 293, 308, 323, 338, 353, 368, 159, 174, 189,
    204, 219, 234, 249, 264, 279, 294, 309, 324, 339, 354, 369, 384, 175, 190, 205, 220, 235, 250,
    265, 280, 295, 310, 325, 340, 355, 370, 385, 400, 191, 206, 221, 236, 251, 266, 281, 296, 311,
    326, 341, 356, 371, 386, 401, 416, 207, 222, 237, 252, 267, 282, 297, 312, 327, 342, 357, 372,
    387, 402, 417, 432, 223, 238, 253, 268, 283, 298, 313, 328, 343, 358, 373, 388, 403, 418, 433,
    448, 239, 254, 269, 284, 299, 314, 329, 344, 359, 374, 389, 404, 419, 434, 449, 464, 255, 270,
    285, 300, 315, 330, 345, 360, 375, 390, 405, 420, 435, 450, 465, 480, 271, 286, 301, 316, 331,
    346, 361, 376, 391, 406, 421, 436, 451, 466, 481, 496, 287, 302, 317, 332, 347, 362, 377, 392,
    407, 422, 437, 452, 467, 482, 497, 303, 318, 333, 348, 363, 378, 393, 408, 423, 438, 453, 468,
    483, 498, 319, 334, 349, 364, 379, 394, 409, 424, 439, 454, 469, 484, 499, 335, 350, 365, 380,
    395, 410, 425, 440, 455, 470, 485, 500, 351, 366, 381, 396, 411, 426, 441, 456, 471, 486, 501,
    367, 382, 397, 412, 427, 442, 457, 472, 487, 502, 383, 398, 413, 428, 443, 458, 473, 488, 503,
    399, 414, 429, 444, 459, 474, 489, 504, 415, 430, 445, 460, 475, 490, 505, 431, 446, 461, 476,
    491, 506, 447, 462, 477, 492, 507, 463, 478, 493, 508, 479, 494, 509, 495, 510, 511,
];

/// `dav1d_lo_ctx_offsets[0]` (the `w == h` square variant). Indexed `[min(y,4)][min(x,4)]`.
/// Source: rav1d `dav1d_lo_ctx_offsets` (`src/tables.c`).
const LO_CTX_OFFSETS_SQUARE: [[u8; 5]; 5] = [
    [0, 1, 6, 6, 21],
    [1, 6, 6, 21, 21],
    [6, 6, 21, 21, 21],
    [6, 21, 21, 21, 21],
    [21, 21, 21, 21, 21],
];
/// `dav1d_lo_ctx_offsets[1]` (`width > height`). Source: rav1d `dav1d_lo_ctx_offsets`
/// (`src/tables.c`) -- real per-aspect-ratio context table for genuinely rectangular transforms
/// (see this module's doc; `nonsquare_tx + (tx & nonsquare_tx)` selects index `1` for every
/// wider-than-tall `RectTxfmSize`, verified against the real enum ordering).
const LO_CTX_OFFSETS_WIDE: [[u8; 5]; 5] = [
    [0, 16, 6, 6, 21],
    [16, 16, 6, 21, 21],
    [16, 16, 21, 21, 21],
    [16, 16, 21, 21, 21],
    [16, 16, 21, 21, 21],
];
/// `dav1d_lo_ctx_offsets[2]` (`width < height`). Source: rav1d `dav1d_lo_ctx_offsets`
/// (`src/tables.c`).
const LO_CTX_OFFSETS_TALL: [[u8; 5]; 5] = [
    [0, 11, 11, 11, 11],
    [11, 11, 11, 11, 11],
    [6, 6, 21, 21, 21],
    [6, 21, 21, 21, 21],
    [21, 21, 21, 21, 21],
];

/// Real per-`(width, height)` scan table lookup -- `width_dim`/`height_dim` are the transform's
/// dimensions in samples, `4/8/16/32` only (callers must have already clamped any 64-sample axis
/// to `32`, matching dav1d's own `dav1d_scans` table reusing the 32-capped scan for those sizes --
/// see this module's doc). Panics (via the table match) on any other input -- an internal
/// programming error, not a real-data condition, since every caller derives these dims from this
/// crate's own tx-size representation.
fn scan_table(width_dim: u32, height_dim: u32) -> &'static [u16] {
    match (width_dim, height_dim) {
        (4, 4) => &SCAN_4X4,
        (8, 8) => &SCAN_8X8,
        (16, 16) => &SCAN_16X16,
        (32, 32) => &SCAN_32X32,
        (4, 8) => &SCAN_4X8,
        (8, 4) => &SCAN_8X4,
        (4, 16) => &SCAN_4X16,
        (16, 4) => &SCAN_16X4,
        (8, 16) => &SCAN_8X16,
        (16, 8) => &SCAN_16X8,
        (8, 32) => &SCAN_8X32,
        (32, 8) => &SCAN_32X8,
        (16, 32) => &SCAN_16X32,
        (32, 16) => &SCAN_32X16,
        _ => panic!("scan_table: unsupported (width_dim={width_dim}, height_dim={height_dim})"),
    }
}

/// Coefficient-grid `(x, y)` position for decode-order index `c` (spec's scan-order traversal,
/// high-frequency to low-frequency -- matches `SymbolDecoder::read_residual_block`'s
/// `for c in (0..eob).rev()` loop directly, no off-by-one adjustment needed). `width_dim`/
/// `height_dim`: transform dimensions in samples (`4/8/16/32`, already 32-capped by the caller --
/// see `scan_table`'s doc).
///
/// `is_1d`/`is_vertical` (the latter meaningless when `!is_1d`): `TX_CLASS_H` and `TX_CLASS_V`
/// both use a closed-form `x = c % dim, y = c / dim`, but -- verified against rav1d's
/// `DECODE_COEFS_CLASS` macro (`src/recon_tmpl.c`) directly, not assumed -- **`dim` differs by
/// class**: `TX_CLASS_H` uses `height_dim`, `TX_CLASS_V` uses `width_dim`. For a square transform
/// these are identical (the only case this crate supported before rectangular var-tx), which is
/// why this distinction was previously invisible; for a genuinely rectangular transform, using the
/// wrong axis would feed `lo_ctx` different (still self-consistent, but wrong) neighbor-context
/// positions than a real encoder assumes, silently drifting the shared adaptive CDF -- the same
/// failure shape as every other context-selection bug this session found. `false` (2D) looks up
/// the real scan table instead.
pub fn coeff_position(
    width_dim: u32,
    height_dim: u32,
    is_1d: bool,
    is_vertical: bool,
    c: u32,
) -> (u32, u32) {
    if is_1d {
        let dim = if is_vertical { width_dim } else { height_dim };
        return (c % dim, c / dim);
    }
    let rc = scan_table(width_dim, height_dim)[c as usize] as u32;
    (rc / height_dim, rc % height_dim)
}

/// Per-transform-block coefficient-level scratch state (dav1d's `t->scratch.levels`, unpacked --
/// see this module's doc). Cleared per transform block (`LevelBuffer::new`); stores each decoded
/// position's token value scaled `tok * 65` when unextended, `tok + 192` when `coeff_br`-extended
/// -- matches dav1d's `tok * 0x41` / `tok + (3 << 6)` byte encoding exactly, since `lo_ctx`'s
/// magnitude sum treats the raw stored byte as the "how significant is this neighbor" signal, not
/// a literal level -- porting the encoding, not just the final decoded value, is required for the
/// context bucket math below to match real spec. Sized `(width+2)*(height+2)` (generalized from
/// the original square-only `(dim+2)^2` for rectangular transforms -- `x` ranges `0..width`, `y`
/// ranges `0..height`, matching `coeff_position`'s convention) so neighbor reads past the
/// transform block's low-frequency edge (`x+1`/`x+2`/`y+1`/`y+2` beyond bounds) read real zeros
/// without explicit bounds checks, matching dav1d's own padding.
pub struct LevelBuffer {
    data: Vec<u8>,
    height: usize,
}

impl LevelBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let stride = height + 2;
        Self {
            data: vec![0u8; (width + 2) * stride],
            height,
        }
    }

    fn get(&self, x: u32, y: u32) -> u32 {
        let stride = self.height + 2;
        self.data
            .get(x as usize * stride + y as usize)
            .copied()
            .unwrap_or(0) as u32
    }

    /// Record one decoded position's token, scaled per this struct's doc. `level`: the raw
    /// pre-golomb token/magnitude (`base_level`, or the post-`coeff_br`-extension value when
    /// `extended`).
    pub fn set(&mut self, x: u32, y: u32, extended: bool, level: u32) {
        let stride = self.height + 2;
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
///
/// `width_dim`/`height_dim` (only used by the 2D branch's offset-table selection): real dav1d
/// selects a different 5x5 offset table for square vs `width > height` vs `width < height`
/// transforms (`LO_CTX_OFFSETS_SQUARE`/`_WIDE`/`_TALL`'s doc) -- for a square transform these
/// dims are equal and the square table is always selected, matching this function's pre-rect
/// behavior exactly.
pub fn lo_ctx(
    levels: &LevelBuffer,
    x: u32,
    y: u32,
    is_1d: bool,
    width_dim: u32,
    height_dim: u32,
) -> (u8, u32) {
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
        let table = match width_dim.cmp(&height_dim) {
            std::cmp::Ordering::Equal => &LO_CTX_OFFSETS_SQUARE,
            std::cmp::Ordering::Greater => &LO_CTX_OFFSETS_WIDE,
            std::cmp::Ordering::Less => &LO_CTX_OFFSETS_TALL,
        };
        offset = table[y.min(4) as usize][x.min(4) as usize] as u32;
    }
    let ctx = offset + if mag > 512 { 4 } else { (mag + 64) >> 7 };
    (ctx as u8, hi_mag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coeff_position_2d_matches_known_scan_values() {
        assert_eq!(coeff_position(4, 4, false, false, 0), (0, 0));
        assert_eq!(coeff_position(4, 4, false, false, 1), (1, 0)); // SCAN_4X4[1] = 4 = 1*4+0
        assert_eq!(coeff_position(4, 4, false, false, 4), (1, 1)); // SCAN_4X4[4] = 5 = 1*4+1
        assert_eq!(coeff_position(4, 4, false, false, 15), (3, 3)); // SCAN_4X4[15] = 15 = 3*4+3
    }

    #[test]
    fn test_coeff_position_is_1d_horizontal_uses_height_dim() {
        // TX_CLASS_H: dim = height_dim (8x4 transform, height_dim=4).
        assert_eq!(coeff_position(8, 4, true, false, 0), (0, 0));
        assert_eq!(coeff_position(8, 4, true, false, 1), (1, 0));
        assert_eq!(coeff_position(8, 4, true, false, 4), (0, 1)); // wraps at height_dim=4
        assert_eq!(coeff_position(8, 4, true, false, 31), (3, 7)); // 31%4=3, 31/4=7
    }

    #[test]
    fn test_coeff_position_is_1d_vertical_uses_width_dim() {
        // TX_CLASS_V: dim = width_dim -- different axis than horizontal for a rect transform.
        assert_eq!(coeff_position(8, 4, true, true, 0), (0, 0));
        assert_eq!(coeff_position(8, 4, true, true, 1), (1, 0));
        assert_eq!(coeff_position(8, 4, true, true, 8), (0, 1)); // wraps at width_dim=8
        assert_eq!(coeff_position(8, 4, true, true, 31), (7, 3));
    }

    #[test]
    fn test_coeff_position_all_positions_unique_and_in_range() {
        let square_dims = [(4, 4), (8, 8), (16, 16), (32, 32)];
        let rect_dims = [
            (4, 8),
            (8, 4),
            (4, 16),
            (16, 4),
            (8, 16),
            (16, 8),
            (8, 32),
            (32, 8),
            (16, 32),
            (32, 16),
        ];
        for (w, h) in square_dims.into_iter().chain(rect_dims) {
            let mut seen = std::collections::HashSet::new();
            for c in 0..(w * h) {
                let (x, y) = coeff_position(w, h, false, false, c);
                assert!(x < w && y < h, "({w}x{h}) c={c} -> ({x},{y}) out of range");
                assert!(seen.insert((x, y)), "({w}x{h}) duplicate position at c={c}");
            }
        }
    }

    #[test]
    fn test_level_buffer_defaults_to_zero() {
        let buf = LevelBuffer::new(4, 4);
        assert_eq!(buf.get(0, 0), 0);
        assert_eq!(buf.get(3, 3), 0);
    }

    #[test]
    fn test_level_buffer_out_of_range_reads_zero() {
        let buf = LevelBuffer::new(4, 4);
        assert_eq!(buf.get(10, 10), 0);
    }

    #[test]
    fn test_level_buffer_set_unextended_scales_by_65() {
        let mut buf = LevelBuffer::new(4, 4);
        buf.set(1, 1, false, 2);
        assert_eq!(buf.get(1, 1), 130); // 2 * 65
    }

    #[test]
    fn test_level_buffer_set_extended_offsets_by_192() {
        let mut buf = LevelBuffer::new(4, 4);
        buf.set(1, 1, true, 5);
        assert_eq!(buf.get(1, 1), 197); // 5 + 192
    }

    #[test]
    fn test_level_buffer_rectangular_dims_independent_axes() {
        // width=8, height=4: x can reach 7 (padded to 9 slots), y only 3 (padded to 5 slots).
        let mut buf = LevelBuffer::new(8, 4);
        buf.set(7, 3, false, 2);
        assert_eq!(buf.get(7, 3), 130);
        assert_eq!(buf.get(0, 0), 0);
    }

    #[test]
    fn test_lo_ctx_no_neighbors_is_lowest_bucket() {
        let buf = LevelBuffer::new(8, 8);
        let (ctx, hi_mag) = lo_ctx(&buf, 4, 4, false, 8, 8);
        assert_eq!(hi_mag, 0);
        // offset = LO_CTX_OFFSETS_SQUARE[4][4] = 21, mag bucket = (0+64)>>7 = 0
        assert_eq!(ctx, 21);
    }

    #[test]
    fn test_lo_ctx_2d_neighbors_raise_context_and_hi_mag() {
        let mut buf = LevelBuffer::new(8, 8);
        buf.set(0, 1, false, 3); // offset (0,1) from origin
        buf.set(1, 0, false, 3); // offset (1,0) from origin
        buf.set(1, 1, false, 3); // offset (1,1) from origin
        let (ctx, hi_mag) = lo_ctx(&buf, 0, 0, false, 8, 8);
        assert_eq!(hi_mag, 3 * 65 * 3); // 3 neighbors, each scaled by 65
        assert!(ctx > 0);
    }

    #[test]
    fn test_lo_ctx_is_1d_uses_different_offset_formula() {
        let buf = LevelBuffer::new(16, 16);
        let (ctx_y0, _) = lo_ctx(&buf, 0, 0, true, 16, 16);
        let (ctx_y1, _) = lo_ctx(&buf, 0, 1, true, 16, 16);
        let (ctx_y2, _) = lo_ctx(&buf, 0, 2, true, 16, 16);
        assert_eq!(ctx_y0, 26); // offset 26 + 0*5
        assert_eq!(ctx_y1, 31); // offset 26 + 1*5
        assert_eq!(ctx_y2, 36); // offset 26 + 10 (y > 1)
    }

    #[test]
    fn test_lo_ctx_2d_selects_wide_vs_tall_offset_table() {
        // LO_CTX_OFFSETS_SQUARE[0][1]=1, _WIDE[0][1]=16, _TALL[0][1]=11 -- a position where all
        // three tables disagree, so this proves the right one gets selected per aspect ratio.
        let buf_wide = LevelBuffer::new(16, 8);
        let (ctx_wide, _) = lo_ctx(&buf_wide, 1, 0, false, 16, 8);
        assert_eq!(ctx_wide, 16);
        let buf_tall = LevelBuffer::new(8, 16);
        let (ctx_tall, _) = lo_ctx(&buf_tall, 1, 0, false, 8, 16);
        assert_eq!(ctx_tall, 11);
        let buf_square = LevelBuffer::new(8, 8);
        let (ctx_square, _) = lo_ctx(&buf_square, 1, 0, false, 8, 8);
        assert_eq!(ctx_square, 1);
    }
}
