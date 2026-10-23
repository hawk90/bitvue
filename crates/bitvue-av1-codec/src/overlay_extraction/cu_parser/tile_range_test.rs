//! `tile_superblock_ranges`: which superblocks a tile covers.

use super::tile_superblock_ranges;
use crate::frame_header::TileLayout;

fn layout(cols: &[u32], rows: &[u32]) -> TileLayout {
    TileLayout {
        col_starts_sb: cols.to_vec(),
        row_starts_sb: rows.to_vec(),
        tile_cols_log2: 0,
        tile_rows_log2: 0,
        context_update_tile_id: 0,
        tile_size_bytes: 4,
    }
}

#[test]
fn no_layout_covers_the_whole_frame() {
    assert_eq!(tile_superblock_ranges(None, 0, 7, 3), (0..7, 0..3));
}

#[test]
fn tiles_number_in_raster_order_with_unequal_sizes() {
    let l = layout(&[0, 3, 5], &[0, 4, 6]);
    let at = |n| tile_superblock_ranges(Some(&l), n, 5, 6);
    assert_eq!(at(0), (0..3, 0..4));
    assert_eq!(at(1), (3..5, 0..4));
    assert_eq!(at(2), (0..3, 4..6));
    assert_eq!(at(3), (3..5, 4..6));
}

#[test]
fn a_single_column_layout_steps_down_a_row_per_tile() {
    let l = layout(&[0, 5], &[0, 2, 4, 6]);
    assert_eq!(tile_superblock_ranges(Some(&l), 2, 5, 6), (0..5, 4..6));
}

#[test]
fn the_update_tile_takes_the_final_cdf_and_the_first_tile_is_the_fallback() {
    use super::takes_final_cdf;
    assert!(takes_final_cdf(0, 0, false));
    assert!(takes_final_cdf(2, 2, true));
    assert!(takes_final_cdf(0, 2, false)); // the update tile is absent from the data
    assert!(!takes_final_cdf(1, 2, true));
    assert!(!takes_final_cdf(3, 2, true));
}
