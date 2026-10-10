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
}
