//! Splitting a tile group OBU into its tiles (spec 5.11.1 `tile_group_obu()`).

use crate::frame_header::TileLayout;
use bitvue_engine::BitvueError;
use std::ops::Range;

/// The tiles of one tile group: `(tile number, byte range within the payload)`, in order.
///
/// `payload` is the tile group OBU's payload (for a `Frame` OBU, the bytes after the frame
/// header). A frame with more than one tile starts it with `tile_start_and_end_present_flag`
/// and, when set, the first and last tile numbers; every tile but the last of the group is
/// preceded by `tile_size_minus_1` in `TileSizeBytes` little-endian bytes.
pub fn split_tile_group(
    payload: &[u8],
    layout: &TileLayout,
) -> Result<Vec<(u32, Range<usize>)>, BitvueError> {
    let tile_count = layout.tile_count();
    if tile_count == 0 {
        return Err(BitvueError::InvalidData("frame has no tiles".into()));
    }
    if tile_count == 1 {
        return Ok(vec![(0, 0..payload.len())]);
    }

    let eof = || BitvueError::UnexpectedEof(payload.len() as u64);
    let bits = layout.tile_cols_log2 + layout.tile_rows_log2;
    let mut reader = crate::bitreader::BitReader::new(payload);
    let (first, last) = if reader.read_bit()? {
        (reader.read_bits(bits as u8)?, reader.read_bits(bits as u8)?)
    } else {
        (0, tile_count - 1)
    };
    reader.byte_align();
    if first > last || last >= tile_count {
        return Err(BitvueError::InvalidData(format!(
            "tile group covers tiles {first}..={last} of {tile_count}"
        )));
    }

    let mut offset = reader.byte_position();
    let mut tiles = Vec::with_capacity((last - first + 1) as usize);
    for tile in first..=last {
        let size = if tile == last {
            payload.len().checked_sub(offset).ok_or_else(eof)?
        } else {
            let width = usize::from(layout.tile_size_bytes);
            let field = payload.get(offset..offset + width).ok_or_else(eof)?;
            offset += width;
            field
                .iter()
                .rev()
                .fold(0usize, |acc, &byte| (acc << 8) | usize::from(byte))
                + 1
        };
        let end = offset.checked_add(size).filter(|&end| end <= payload.len());
        let end = end.ok_or_else(eof)?;
        tiles.push((tile, offset..end));
        offset = end;
    }
    Ok(tiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(cols: u32, rows: u32, size_bytes: u8) -> TileLayout {
        TileLayout {
            col_starts_sb: (0..=cols).collect(),
            row_starts_sb: (0..=rows).collect(),
            tile_cols_log2: cols.next_power_of_two().ilog2(),
            tile_rows_log2: rows.next_power_of_two().ilog2(),
            context_update_tile_id: 0,
            tile_size_bytes: size_bytes,
        }
    }

    #[test]
    fn one_tile_is_the_whole_payload() {
        let tiles = split_tile_group(&[1, 2, 3], &layout(1, 1, 4)).unwrap();
        assert_eq!(tiles, vec![(0, 0..3)]);
    }

    /// Four tiles, no start/end: flag 0 (one byte after alignment), then sizes in front of all
    /// but the last tile.
    #[test]
    fn every_tile_but_the_last_is_preceded_by_its_size() {
        // flag byte, [size-1 = 1][2 bytes], [size-1 = 0][1 byte], [size-1 = 2][3 bytes], rest
        let payload = [
            0x00, 1, 0, 0xaa, 0xbb, 0, 0, 0xcc, 2, 0, 0xdd, 0xee, 0xff, 0x11, 0x22,
        ];
        let tiles = split_tile_group(&payload, &layout(2, 2, 2)).unwrap();
        assert_eq!(tiles, vec![(0, 3..5), (1, 7..8), (2, 10..13), (3, 13..15)]);
    }

    /// With `tile_start_and_end_present_flag` the group names its first and last tile.
    #[test]
    fn a_tile_group_may_cover_part_of_the_tiles() {
        // flag 1, first tile 2 and last tile 3 in TileColsLog2 + TileRowsLog2 = 2 bits each, then
        // byte alignment; tile 2 has a size field, tile 3 runs to the end.
        let payload = [0b1101_1000, 1, 0, 0xaa, 0xbb, 0xcc];
        let tiles = split_tile_group(&payload, &layout(2, 2, 2)).unwrap();
        assert_eq!(tiles, vec![(2, 3..5), (3, 5..6)]);
    }

    /// The widths of a tile group's first/last tile numbers are `TileColsLog2 + TileRowsLog2`
    /// bits, from the header -- a guess from the tile count gets 5x5 tiles wrong (3 + 3 = 6 bits,
    /// not 5).
    #[test]
    fn first_and_last_tile_numbers_are_as_wide_as_the_header_log2s_say() {
        let mut layout = layout(5, 5, 1);
        layout.tile_cols_log2 = 3;
        layout.tile_rows_log2 = 3;
        // flag 1, first = 0b010111 (tile 23), last = 0b011000 (tile 24), alignment; tile 23 has
        // a one-byte size field, tile 24 runs to the end.
        let payload = [0b1010_1110, 0b1100_0000, 0, 0xaa, 0xbb, 0xcc];
        let tiles = split_tile_group(&payload, &layout).unwrap();
        assert_eq!(tiles, vec![(23, 3..4), (24, 4..6)]);
    }

    #[test]
    fn sizes_past_the_payload_are_errors() {
        let payload = [0x00, 9, 0, 0xaa];
        assert!(split_tile_group(&payload, &layout(2, 2, 2)).is_err());
        assert!(split_tile_group(&[0x00], &layout(2, 2, 2)).is_err());
    }

    // ---- error injection and round trips --------------------------------------------------

    #[test]
    fn a_layout_without_tiles_is_rejected() {
        let mut empty = layout(2, 2, 2);
        empty.col_starts_sb = vec![0]; // zero tile columns
        assert_eq!(empty.tile_count(), 0);
        for payload in [&[][..], &[0x00, 1, 2, 3][..]] {
            assert!(matches!(
                split_tile_group(payload, &empty),
                Err(BitvueError::InvalidData(_))
            ));
        }
        let mut no_rows = layout(2, 2, 2);
        no_rows.row_starts_sb.clear();
        assert!(split_tile_group(&[0], &no_rows).is_err());
    }

    #[test]
    fn a_tile_range_the_group_cannot_hold_is_rejected() {
        // 2x2 tiles: two bits per tile number. flag 1, first 3, last 1 -> first > last.
        let backwards = [0b1110_1000, 0, 0xaa];
        assert!(matches!(
            split_tile_group(&backwards, &layout(2, 2, 2)),
            Err(BitvueError::InvalidData(_))
        ));
        // 3x1 tiles: TileColsLog2 = 2, so tile number 3 is codable but does not exist.
        let beyond = layout(3, 1, 2);
        assert_eq!(beyond.tile_count(), 3);
        let last_is_3 = [0b1001_1000, 0, 0xaa];
        assert!(matches!(
            split_tile_group(&last_is_3, &beyond),
            Err(BitvueError::InvalidData(_))
        ));
        // The boundaries around it are fine: last = 2 (the final tile) and first == last.
        let last_is_2 = [0b1001_0000, 0, 0, 0xaa, 0, 0, 0xbb, 0xcc];
        assert_eq!(
            split_tile_group(&last_is_2, &beyond).unwrap(),
            vec![(0, 3..4), (1, 6..7), (2, 7..8)]
        );
        let only_tile_1 = [0b1010_1000, 0xaa, 0xbb];
        assert_eq!(
            split_tile_group(&only_tile_1, &beyond).unwrap(),
            vec![(1, 1..3)]
        );
    }

    /// Builds a tile group payload: the start/end header (`range` = `Some((first, last))` sets
    /// the flag), byte alignment, then each tile preceded by its `size_bytes`-wide size except the
    /// last. Returns the payload and the byte range the splitter must report for each tile.
    fn build(
        range: Option<(u32, u32)>,
        bits: u32,
        size_bytes: usize,
        tiles: &[Vec<u8>],
    ) -> (Vec<u8>, Vec<Range<usize>>) {
        let mut header: Vec<bool> = vec![range.is_some()];
        if let Some((first, last)) = range {
            for value in [first, last] {
                for i in (0..bits).rev() {
                    header.push(value >> i & 1 == 1);
                }
            }
        }
        let mut payload: Vec<u8> = header
            .chunks(8)
            .map(|c| {
                c.iter()
                    .enumerate()
                    .fold(0u8, |b, (i, &x)| b | (u8::from(x) << (7 - i)))
            })
            .collect();
        let mut ranges = Vec::new();
        for (i, tile) in tiles.iter().enumerate() {
            if i + 1 < tiles.len() {
                let minus_1 = tile.len() - 1;
                payload.extend((0..size_bytes).map(|b| (minus_1 >> (8 * b)) as u8));
            }
            let start = payload.len();
            payload.extend(tile);
            ranges.push(start..payload.len());
        }
        (payload, ranges)
    }

    fn noise(seed: u64) -> impl FnMut() -> u64 {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        }
    }

    #[test]
    fn built_tile_groups_split_back_into_the_same_tiles() {
        let mut next = noise(1);
        let mut checked = 0;
        for (cols, rows) in [(2, 1), (1, 2), (2, 2), (3, 1), (3, 2), (4, 2), (5, 5)] {
            for size_bytes in 1..=4usize {
                let l = layout(cols, rows, size_bytes as u8);
                let count = l.tile_count();
                let bits = l.tile_cols_log2 + l.tile_rows_log2;
                for round in 0..8 {
                    // Whole frame, or a random sub-range named explicitly.
                    let (first, last) = if round % 2 == 0 {
                        (0, count - 1)
                    } else {
                        let a = (next() % u64::from(count)) as u32;
                        let b = (next() % u64::from(count)) as u32;
                        (a.min(b), a.max(b))
                    };
                    let tiles: Vec<Vec<u8>> = (first..=last)
                        .map(|_| (0..1 + next() % 40).map(|_| next() as u8).collect())
                        .collect();
                    let range = (round % 2 == 1).then_some((first, last));
                    let (payload, ranges) = build(range, bits, size_bytes, &tiles);
                    let got = split_tile_group(&payload, &l).unwrap();
                    let want: Vec<_> = (first..=last).zip(ranges).collect();
                    assert_eq!(
                        got, want,
                        "{cols}x{rows} size_bytes {size_bytes} round {round}"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 7 * 4 * 8);
    }

    #[test]
    fn every_truncation_and_bit_flip_either_errors_or_stays_inside_the_payload() {
        let mut next = noise(7);
        let l = layout(3, 2, 2);
        let bits = l.tile_cols_log2 + l.tile_rows_log2;
        let tiles: Vec<Vec<u8>> = (0..6)
            .map(|i| (0..5 + i).map(|_| next() as u8).collect())
            .collect();
        let (payload, _) = build(None, bits, 2, &tiles);
        let sane = |p: &[u8]| {
            if let Ok(found) = split_tile_group(p, &l) {
                let mut end = 0;
                for (tile, range) in found {
                    assert!(tile < 6);
                    assert!(
                        range.start <= range.end && range.end <= p.len(),
                        "{range:?}"
                    );
                    assert!(range.start >= end, "ranges are ordered and disjoint");
                    end = range.end;
                }
            }
        };
        for cut in 0..=payload.len() {
            sane(&payload[..cut]);
        }
        for byte in 0..payload.len() {
            for bit in 0..8 {
                let mut flipped = payload.clone();
                flipped[byte] ^= 1 << bit;
                sane(&flipped);
            }
        }
        // And a proper prefix of a multi-tile group can never be accepted as complete: the last
        // tile's size is implied, so only truncations inside a size field or past a tile's
        // declared end are detectable.
        assert!(split_tile_group(&payload[..2], &l).is_err());
        assert!(split_tile_group(&[], &l).is_err());
    }
}
