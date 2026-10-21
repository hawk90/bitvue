//! `tile_info()` (spec 5.9.15).

use super::*;
use crate::frame_header::TileLayout;

const MAX_TILE_WIDTH_SB_BASE: u32 = 4096; // MAX_TILE_WIDTH
const MAX_TILE_AREA_BASE: u64 = 4096 * 2304; // MAX_TILE_AREA
const MAX_TILE_COLS: u32 = 64;
const MAX_TILE_ROWS: u32 = 64;

/// The frame-size-derived limits `tile_info()` reads against (spec 5.9.15, before the first bit).
struct TileGeometry {
    sb_cols: u32,
    sb_rows: u32,
    max_tile_width_sb: u32,
    max_log2_tile_cols: u32,
    max_log2_tile_rows: u32,
    min_log2_tile_cols: u32,
    min_log2_tiles: u32,
}

/// Tile boundaries (without the closing `sb_cols`/`sb_rows` entry) and their log2 counts.
struct TileSpacing {
    col_starts: Vec<u32>,
    row_starts: Vec<u32>,
    cols_log2: u32,
    rows_log2: u32,
}

fn tile_geometry(seq: &SequenceHeader, frame_width: u32, frame_height: u32) -> TileGeometry {
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
    TileGeometry {
        sb_cols,
        sb_rows,
        max_tile_width_sb,
        max_log2_tile_cols,
        max_log2_tile_rows,
        min_log2_tile_cols,
        min_log2_tiles,
    }
}

/// `increment_tile_{cols,rows}_log2` bits: one `1` per step up to `max`, ended by a `0`.
fn read_log2_increments(reader: &mut BitReader, min: u32, max: u32) -> Result<u32> {
    let mut log2 = min;
    while log2 < max {
        if reader.read_bit()? {
            log2 += 1;
        } else {
            break;
        }
    }
    Ok(log2)
}

fn read_uniform_spacing(reader: &mut BitReader, g: &TileGeometry) -> Result<TileSpacing> {
    let cols_log2 = read_log2_increments(reader, g.min_log2_tile_cols, g.max_log2_tile_cols)?;
    let tile_width_sb = ((g.sb_cols + (1 << cols_log2) - 1) >> cols_log2).max(1);
    let col_starts = (0..g.sb_cols).step_by(tile_width_sb as usize).collect();

    let min_log2_tile_rows = g.min_log2_tiles.saturating_sub(cols_log2);
    let rows_log2 = read_log2_increments(reader, min_log2_tile_rows, g.max_log2_tile_rows)?;
    let tile_height_sb = ((g.sb_rows + (1 << rows_log2) - 1) >> rows_log2).max(1);
    let row_starts = (0..g.sb_rows).step_by(tile_height_sb as usize).collect();
    Ok(TileSpacing {
        col_starts,
        row_starts,
        cols_log2,
        rows_log2,
    })
}

fn read_explicit_spacing(reader: &mut BitReader, g: &TileGeometry) -> Result<TileSpacing> {
    let mut col_starts = Vec::new();
    let mut widest_tile_sb = 0u32;
    let mut start_sb = 0u32;
    let mut cols_count = 0u32;
    while start_sb < g.sb_cols {
        let max_width = (g.sb_cols - start_sb).min(g.max_tile_width_sb);
        let width_in_sbs_minus_1 = read_ns(reader, max_width)?;
        let size_sb = width_in_sbs_minus_1 + 1;
        widest_tile_sb = widest_tile_sb.max(size_sb);
        col_starts.push(start_sb);
        start_sb += size_sb;
        cols_count += 1;
    }
    let cols_log2 = tile_log2(1, cols_count);

    let max_tile_area_sb2: u64 = if g.min_log2_tiles > 0 {
        (g.sb_rows as u64 * g.sb_cols as u64) >> (g.min_log2_tiles + 1)
    } else {
        g.sb_rows as u64 * g.sb_cols as u64
    };
    let max_tile_height_sb = ((max_tile_area_sb2 / widest_tile_sb.max(1) as u64).max(1)) as u32;
    let mut row_starts = Vec::new();
    let mut start_sb_row = 0u32;
    let mut rows_count = 0u32;
    while start_sb_row < g.sb_rows {
        let max_height = (g.sb_rows - start_sb_row).min(max_tile_height_sb);
        let height_in_sbs_minus_1 = read_ns(reader, max_height)?;
        let size_sb = height_in_sbs_minus_1 + 1;
        row_starts.push(start_sb_row);
        start_sb_row += size_sb;
        rows_count += 1;
    }
    let rows_log2 = tile_log2(1, rows_count);
    Ok(TileSpacing {
        col_starts,
        row_starts,
        cols_log2,
        rows_log2,
    })
}

/// `context_update_tile_id` and `tile_size_bytes_minus_1`, present only for multi-tile frames.
fn read_tile_trailer(reader: &mut BitReader, cols_log2: u32, rows_log2: u32) -> Result<(u32, u8)> {
    if cols_log2 > 0 || rows_log2 > 0 {
        let context_update_tile_id = reader.read_bits((rows_log2 + cols_log2) as u8)?;
        let tile_size_bytes = reader.read_bits(2)? as u8 + 1;
        return Ok((context_update_tile_id, tile_size_bytes));
    }
    Ok((0, 4))
}

pub(super) fn read_tile_info(
    reader: &mut BitReader,
    seq: &SequenceHeader,
    frame_width: u32,
    frame_height: u32,
) -> Result<TileLayout> {
    let geometry = tile_geometry(seq, frame_width, frame_height);
    let uniform_tile_spacing_flag = reader.read_bit()?;
    let mut spacing = if uniform_tile_spacing_flag {
        read_uniform_spacing(reader, &geometry)?
    } else {
        read_explicit_spacing(reader, &geometry)?
    };
    let (context_update_tile_id, tile_size_bytes) =
        read_tile_trailer(reader, spacing.cols_log2, spacing.rows_log2)?;
    spacing.col_starts.push(geometry.sb_cols);
    spacing.row_starts.push(geometry.sb_rows);
    Ok(TileLayout {
        col_starts_sb: spacing.col_starts,
        row_starts_sb: spacing.row_starts,
        tile_cols_log2: spacing.cols_log2,
        tile_rows_log2: spacing.rows_log2,
        context_update_tile_id,
        tile_size_bytes,
    })
}

#[cfg(test)]
#[path = "tile_info_tests.rs"]
mod tile_info_tests;
