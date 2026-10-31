#![no_main]

use bitvue_av1_codec::frame_header::TileLayout;
use bitvue_av1_codec::tile::layout::split_tile_group;
use libfuzzer_sys::fuzz_target;

// The tile-group layer of a multi-tile frame (spec 5.11.1 `tile_group_obu()`), driven directly:
// the first four bytes choose a `TileLayout` (tile columns, rows, the `TileColsLog2`/`TileRowsLog2`
// that size the first/last tile fields, and `TileSizeBytes`), the rest is the tile group payload.
// Mutating a real stream (`stream_analysis`) rarely changes those layout fields together with a
// consistent payload, so this target reaches the size-field and tile-range arithmetic that a
// separate-tile-group OBU relies on.
//
// Beyond "no panic" it checks what a successful split promises: consecutive tile numbers inside
// the frame, and byte ranges that are ordered, non-overlapping and inside the payload.
fuzz_target!(|data: &[u8]| {
    let Some((head, payload)) = data.split_first_chunk::<4>() else {
        return;
    };
    let cols = 1 + u32::from(head[0] % 16);
    let rows = 1 + u32::from(head[1] % 16);
    let layout = TileLayout {
        col_starts_sb: (0..=cols).collect(),
        row_starts_sb: (0..=rows).collect(),
        // independent of the counts on purpose: a uniform layout may have fewer tiles than 1 << log2
        tile_cols_log2: u32::from(head[2] >> 4) % 7,
        tile_rows_log2: u32::from(head[2] & 0x0f) % 7,
        context_update_tile_id: 0,
        tile_size_bytes: 1 + head[3] % 4,
    };

    let Ok(tiles) = split_tile_group(payload, &layout) else {
        return;
    };
    assert!(!tiles.is_empty(), "an Ok split holds at least one tile");
    let mut previous_end = 0usize;
    let mut previous_tile: Option<u32> = None;
    for (tile, range) in &tiles {
        assert!(*tile < layout.tile_count(), "tile {tile} outside the frame");
        if let Some(prev) = previous_tile {
            assert_eq!(*tile, prev + 1, "tile numbers must be consecutive");
        }
        assert!(range.start <= range.end && range.end <= payload.len());
        assert!(range.start >= previous_end, "tile ranges overlap");
        previous_end = range.end;
        previous_tile = Some(*tile);
    }
});
