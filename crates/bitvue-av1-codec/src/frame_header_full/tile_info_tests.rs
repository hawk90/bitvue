//! `read_tile_info` pinned on hand-built bit strings (uniform and explicit spacing, 64x64 and
//! 128x128 superblocks, the context-update trailer) and on a seeded sweep of random bits.

use super::super::tests::base_seq;
use super::*;

fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, b) in bits.iter().enumerate() {
        if *b != 0 {
            out[i / 8] |= 1 << (7 - (i % 8));
        }
    }
    out
}

fn put(bits: &mut Vec<u8>, value: u32, width: u8) {
    for i in (0..width).rev() {
        bits.push(((value >> i) & 1) as u8);
    }
}

fn read(seq: &SequenceHeader, w: u32, h: u32, bits: &[u8]) -> (Result<TileLayout>, u64) {
    let data = bits_to_bytes(bits);
    let mut r = BitReader::new(&data);
    let out = read_tile_info(&mut r, seq, w, h);
    (out, r.position())
}

fn seq128() -> SequenceHeader {
    let mut s = base_seq();
    s.use_128x128_superblock = true;
    s
}

#[test]
fn single_tile_uniform_has_no_trailer() {
    // 320x240, 64x64 SB: 5x4 superblocks.
    let (out, pos) = read(&base_seq(), 320, 240, &[1, 0, 0]);
    let t = out.unwrap();
    assert_eq!(t.col_starts_sb, vec![0, 5]);
    assert_eq!(t.row_starts_sb, vec![0, 4]);
    assert_eq!((t.tile_cols_log2, t.tile_rows_log2), (0, 0));
    assert_eq!((t.context_update_tile_id, t.tile_size_bytes), (0, 4));
    assert_eq!(pos, 3);
}

#[test]
fn uniform_two_columns_reads_trailer() {
    let mut bits = vec![1, 1, 0, 0]; // uniform, +1 col log2, stop, no row increment
    put(&mut bits, 1, 1); // context_update_tile_id (1 bit: rows 0 + cols 1)
    put(&mut bits, 2, 2); // tile_size_bytes_minus_1
    let (out, pos) = read(&base_seq(), 320, 240, &bits);
    let t = out.unwrap();
    // 5 SB cols at log2 1 -> width ceil(5/2)=3 -> starts 0,3 then 5.
    assert_eq!(t.col_starts_sb, vec![0, 3, 5]);
    assert_eq!(t.row_starts_sb, vec![0, 4]);
    assert_eq!((t.tile_cols_log2, t.tile_rows_log2), (1, 0));
    assert_eq!((t.context_update_tile_id, t.tile_size_bytes), (1, 3));
    assert_eq!(pos, 7);
}

#[test]
fn uniform_rows_and_columns_at_their_maxima_stop_without_a_terminator() {
    // 5 SB cols -> max_log2_cols 3; 4 SB rows -> max_log2_rows 2: three 1s then two 1s, no 0.
    let mut bits = vec![1, 1, 1, 1, 1, 1];
    put(&mut bits, 0b10101, 5); // context_update_tile_id: 3+2 bits
    put(&mut bits, 0, 2);
    let (out, pos) = read(&base_seq(), 320, 240, &bits);
    let t = out.unwrap();
    assert_eq!((t.tile_cols_log2, t.tile_rows_log2), (3, 2));
    assert_eq!(t.col_starts_sb, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(t.row_starts_sb, vec![0, 1, 2, 3, 4]);
    assert_eq!(t.context_update_tile_id, 0b10101);
    assert_eq!(t.tile_size_bytes, 1);
    assert_eq!(pos, 13);
}

#[test]
fn uniform_layout_can_have_fewer_tiles_than_log2_implies() {
    // 5 SB cols at log2 2 -> width 2 -> starts 0,2,4 (3 tiles, not 4).
    let mut bits = vec![1, 1, 1, 0, 0];
    put(&mut bits, 0, 2);
    put(&mut bits, 0, 2);
    let t = read(&base_seq(), 320, 240, &bits).0.unwrap();
    assert_eq!(t.col_starts_sb, vec![0, 2, 4, 5]);
    assert_eq!(t.tile_cols_log2, 2);
}

#[test]
fn explicit_spacing_reads_ns_coded_sizes() {
    // 320x240: 5 cols, 4 rows. Cols: ns(5)=2 (w=3,m=3,v=2<3 -> 2 bits: "10") -> size 3;
    // then remaining 2: ns(2): w=2, m=2, 1 bit v=1<2 -> 1 -> size 2.
    let mut bits = vec![0];
    put(&mut bits, 2, 2);
    put(&mut bits, 1, 1);
    // widest = 3; max_tile_area_sb2 = 20 -> max_height = 20/3 = 6 -> min(4,6)=4: ns(4) = 2 bits,
    // v=1 -> size 2, then ns(2): v=0 -> size 1, then ns(1)=0 bits -> size 1.
    put(&mut bits, 1, 2);
    put(&mut bits, 0, 1);
    // rows: 0..2, 2..3, then remaining 1 -> ns(1) reads nothing.
    put(&mut bits, 0b101, 3); // context_update_tile_id (cols log2 1 + rows log2 2 = 3 bits)
    put(&mut bits, 3, 2);
    let (out, _) = read(&base_seq(), 320, 240, &bits);
    let t = out.unwrap();
    assert_eq!(t.col_starts_sb, vec![0, 3, 5]);
    assert_eq!(t.row_starts_sb, vec![0, 2, 3, 4]);
    assert_eq!((t.tile_cols_log2, t.tile_rows_log2), (1, 2));
    assert_eq!((t.context_update_tile_id, t.tile_size_bytes), (0b101, 4));
}

#[test]
fn superblock_size_changes_the_grid() {
    // 320x240 with 128x128 SBs: 3x2 superblocks.
    let t = read(&seq128(), 320, 240, &[1, 0, 0]).0.unwrap();
    assert_eq!(t.col_starts_sb, vec![0, 3]);
    assert_eq!(t.row_starts_sb, vec![0, 2]);
}

#[test]
fn truncated_input_is_an_error() {
    assert!(read(&base_seq(), 320, 240, &[]).0.is_err());
    let data: [u8; 0] = [];
    let mut r = BitReader::new(&data);
    assert!(read_tile_info(&mut r, &base_seq(), 320, 240).is_err());
}

#[test]
fn trailer_truncation_is_an_error() {
    // Six increment bits reach 8 tile columns x 4 rows; the 5-bit tile id and 2-bit size
    // field then need 13 bits in total, and one byte only has 8.
    let data = [0b1111_1110u8];
    let mut r = BitReader::new(&data);
    assert!(read_tile_info(&mut r, &base_seq(), 320, 240).is_err());
}

fn fnv(h: &mut u64, v: u64) {
    for b in v.to_le_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

fn sweep(seed: u64, n: usize) -> u64 {
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for _ in 0..n {
        let w = 8 + (next() % 4200) as u32;
        let hh = 8 + (next() % 2400) as u32;
        let seq = if next() & 1 == 0 {
            base_seq()
        } else {
            seq128()
        };
        let data: Vec<u8> = (0..64).map(|_| next() as u8).collect();
        let mut r = BitReader::new(&data);
        match read_tile_info(&mut r, &seq, w, hh) {
            Ok(t) => {
                fnv(&mut h, 1);
                for v in t.col_starts_sb.iter().chain(&t.row_starts_sb) {
                    fnv(&mut h, *v as u64);
                }
                fnv(&mut h, t.tile_cols_log2 as u64);
                fnv(&mut h, t.tile_rows_log2 as u64);
                fnv(&mut h, t.context_update_tile_id as u64);
                fnv(&mut h, t.tile_size_bytes as u64);
            }
            Err(_) => fnv(&mut h, 0),
        }
        fnv(&mut h, r.position());
    }
    h
}

#[test]
fn seeded_sweeps_are_pinned() {
    assert_eq!(sweep(0x5eed, 400), 4478196902991484901);
    assert_eq!(sweep(0xfeed, 400), 13505916894534936984);
}
