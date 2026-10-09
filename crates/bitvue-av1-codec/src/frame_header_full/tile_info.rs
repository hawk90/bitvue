//! `tile_info()` (spec 5.9.15).

use super::*;
use crate::frame_header::TileLayout;

const MAX_TILE_WIDTH_SB_BASE: u32 = 4096; // MAX_TILE_WIDTH
const MAX_TILE_AREA_BASE: u64 = 4096 * 2304; // MAX_TILE_AREA
const MAX_TILE_COLS: u32 = 64;
const MAX_TILE_ROWS: u32 = 64;
pub(super) fn read_tile_info(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    frame_width: u32,
    frame_height: u32,
) -> Result<TileLayout> {
    let mi_cols = 2 * ((frame_width + 7) >> 3);
    let mi_rows = 2 * ((frame_height + 7) >> 3);
    let sb_shift: u32 = if seq.use_128x128_superblock { 5 } else { 4 };
    let sb_cols = if seq.use_128x128_superblock {
        (mi_cols + 31) >> 5
    } else {
        (mi_cols + 15) >> 4
    };
    let sb_rows = if seq.use_128x128_superblock {
        (mi_rows + 31) >> 5
    } else {
        (mi_rows + 15) >> 4
    };
    let sb_size = sb_shift + 2;
    let max_tile_width_sb = MAX_TILE_WIDTH_SB_BASE >> sb_size;
    let max_tile_area_sb = MAX_TILE_AREA_BASE >> (2 * sb_size);
    let min_log2_tile_cols = tile_log2(max_tile_width_sb, sb_cols);
    let max_log2_tile_cols = tile_log2(1, sb_cols.min(MAX_TILE_COLS));
    let max_log2_tile_rows = tile_log2(1, sb_rows.min(MAX_TILE_ROWS));
    let min_log2_tiles = min_log2_tile_cols.max(tile_log2(
        max_tile_area_sb.min(u32::MAX as u64) as u32,
        sb_rows * sb_cols,
    ));

    let mut col_starts: Vec<u32> = Vec::new();
    let mut row_starts: Vec<u32> = Vec::new();
    let uniform_tile_spacing_flag = reader.read_bit()?;
    let (tile_cols_log2, tile_rows_log2);
    if uniform_tile_spacing_flag {
        let mut cols_log2 = min_log2_tile_cols;
        while cols_log2 < max_log2_tile_cols {
            if reader.read_bit()? {
                cols_log2 += 1;
            } else {
                break;
            }
        }
        let tile_width_sb = ((sb_cols + (1 << cols_log2) - 1) >> cols_log2).max(1);
        col_starts = (0..sb_cols).step_by(tile_width_sb as usize).collect();

        let min_log2_tile_rows = min_log2_tiles.saturating_sub(cols_log2);
        let mut rows_log2 = min_log2_tile_rows;
        while rows_log2 < max_log2_tile_rows {
            if reader.read_bit()? {
                rows_log2 += 1;
            } else {
                break;
            }
        }
        let tile_height_sb = ((sb_rows + (1 << rows_log2) - 1) >> rows_log2).max(1);
        row_starts = (0..sb_rows).step_by(tile_height_sb as usize).collect();
        tile_cols_log2 = cols_log2;
        tile_rows_log2 = rows_log2;
    } else {
        let mut widest_tile_sb = 0u32;
        let mut start_sb = 0u32;
        let mut cols_count = 0u32;
        while start_sb < sb_cols {
            let max_width = (sb_cols - start_sb).min(max_tile_width_sb);
            let width_in_sbs_minus_1 = read_ns(reader, max_width)?;
            let size_sb = width_in_sbs_minus_1 + 1;
            widest_tile_sb = widest_tile_sb.max(size_sb);
            col_starts.push(start_sb);
            start_sb += size_sb;
            cols_count += 1;
        }
        tile_cols_log2 = tile_log2(1, cols_count);

        let max_tile_area_sb2: u64 = if min_log2_tiles > 0 {
            (sb_rows as u64 * sb_cols as u64) >> (min_log2_tiles + 1)
        } else {
            sb_rows as u64 * sb_cols as u64
        };
        let max_tile_height_sb = ((max_tile_area_sb2 / widest_tile_sb.max(1) as u64).max(1)) as u32;
        let mut start_sb_row = 0u32;
        let mut rows_count = 0u32;
        while start_sb_row < sb_rows {
            let max_height = (sb_rows - start_sb_row).min(max_tile_height_sb);
            let height_in_sbs_minus_1 = read_ns(reader, max_height)?;
            let size_sb = height_in_sbs_minus_1 + 1;
            row_starts.push(start_sb_row);
            start_sb_row += size_sb;
            rows_count += 1;
        }
        tile_rows_log2 = tile_log2(1, rows_count);
    }

    let (mut context_update_tile_id, mut tile_size_bytes) = (0, 4);
    if tile_cols_log2 > 0 || tile_rows_log2 > 0 {
        context_update_tile_id = reader.read_bits((tile_rows_log2 + tile_cols_log2) as u8)?;
        tile_size_bytes = reader.read_bits(2)? as u8 + 1;
    }
    col_starts.push(sb_cols);
    row_starts.push(sb_rows);
    Ok(TileLayout {
        col_starts_sb: col_starts,
        row_starts_sb: row_starts,
        tile_cols_log2,
        tile_rows_log2,
        context_update_tile_id,
        tile_size_bytes,
    })
}
