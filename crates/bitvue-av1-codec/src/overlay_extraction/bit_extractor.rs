//! Per-cell entropy-coded bit cost grid: the data behind the Heat Map (bits per pixel) and the
//! Efficiency Map (symbols per bit) overlays (#458, #488).
//!
//! The costs come from the arithmetic decoder itself (`CodingUnit::bits`/`symbols`, plus the
//! partition share `partition_bits`/`partition_symbols`), so a cell's value is what the encoder
//! actually spent there, not a QP or residual-magnitude proxy.
//!
//! A cell (`OVERLAY_BLOCK_SIZE` square) usually spans many coding units, and a coding unit can span
//! several cells. Each coding unit's cost is therefore spread over the cells it overlaps in
//! proportion to the overlapped area, so the cell totals add up to the coding-unit totals exactly.
//!
//! Unit note: `symbols` counts arithmetic-decoder symbol reads (a multi-symbol read is one), not
//! libaom's binarized bools, so "symbols per bit" is comparable within this app only.

use super::cu_parser::parse_all_coding_units;
use super::parser::ParsedFrame;
use super::OVERLAY_BLOCK_SIZE;
use crate::tile::CodingUnit;
use bitvue_engine::BitvueError;

/// Dense per-cell cost grid, same layout as `EnergyGrid` / `QPGrid` (row-major).
#[derive(Debug, Clone, PartialEq)]
pub struct BitGrid {
    pub grid_w: u32,
    pub grid_h: u32,
    pub block_w: u32,
    pub block_h: u32,
    /// Entropy-coded bits per visible pixel of the cell. 0.0 is a real value (nothing was coded
    /// there, or no tile data), not a missing-data marker.
    pub bits_per_pixel: Vec<f64>,
    /// Arithmetic-decoder symbols per coded bit in the cell; 0.0 for a cell without any bits.
    pub symbols_per_bit: Vec<f64>,
}

/// Extract the bit grid from an already-parsed frame. Falls back to all-zero cells when tile data
/// is absent or coding-unit parsing fails, like `extract_energy_grid_from_parsed`.
pub fn extract_bit_grid_from_parsed(parsed: &ParsedFrame) -> Result<BitGrid, BitvueError> {
    let (width, height) = (parsed.dimensions.width, parsed.dimensions.height);
    let empty = || bit_grid_from_units(width, height, &[]);
    if !parsed.has_tile_data() {
        return empty();
    }
    match parse_all_coding_units(parsed) {
        Ok(units) => bit_grid_from_units(width, height, &units),
        Err(e) => {
            tracing::warn!("Failed to parse coding units for bit grid: {e}, using zeros");
            empty()
        }
    }
}

/// Spread every coding unit's cost over the cells it overlaps. A unit is clipped to the 8-aligned
/// area the decoder codes (blocks may extend past the picture edge), which always lies inside the
/// cell canvas, so no cost is dropped.
pub fn bit_grid_from_units(
    width: u32,
    height: u32,
    units: &[CodingUnit],
) -> Result<BitGrid, BitvueError> {
    let (block_w, block_h) = (OVERLAY_BLOCK_SIZE, OVERLAY_BLOCK_SIZE);
    let grid_w = width.div_ceil(block_w);
    let grid_h = height.div_ceil(block_h);
    let total = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {grid_w}x{grid_h}"))
    })? as usize;
    let (coded_w, coded_h) = (width.div_ceil(8) * 8, height.div_ceil(8) * 8);

    let mut bits = vec![0.0f64; total];
    let mut symbols = vec![0.0f64; total];
    for cu in units {
        let (x0, y0) = (cu.x.min(coded_w), cu.y.min(coded_h));
        let x1 = cu.x.saturating_add(cu.width).min(coded_w);
        let y1 = cu.y.saturating_add(cu.height).min(coded_h);
        let area = f64::from(x1 - x0) * f64::from(y1 - y0);
        if area <= 0.0 {
            continue;
        }
        let cu_bits = cu.bits + cu.partition_bits;
        let cu_symbols = f64::from(cu.symbols) + cu.partition_symbols;
        for gy in y0 / block_h..=(y1 - 1) / block_h {
            for gx in x0 / block_w..=(x1 - 1) / block_w {
                let ox = x1.min((gx + 1) * block_w) - x0.max(gx * block_w);
                let oy = y1.min((gy + 1) * block_h) - y0.max(gy * block_h);
                let share = f64::from(ox) * f64::from(oy) / area;
                let i = (gy * grid_w + gx) as usize;
                bits[i] += cu_bits * share;
                symbols[i] += cu_symbols * share;
            }
        }
    }

    let mut bits_per_pixel = Vec::with_capacity(total);
    let mut symbols_per_bit = Vec::with_capacity(total);
    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let i = (gy * grid_w + gx) as usize;
            let visible = f64::from(width.min((gx + 1) * block_w) - gx * block_w)
                * f64::from(height.min((gy + 1) * block_h) - gy * block_h);
            bits_per_pixel.push(bits[i] / visible);
            symbols_per_bit.push(if bits[i] > 0.0 {
                symbols[i] / bits[i]
            } else {
                0.0
            });
        }
    }
    Ok(BitGrid {
        grid_w,
        grid_h,
        block_w,
        block_h,
        bits_per_pixel,
        symbols_per_bit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(x: u32, y: u32, w: u32, h: u32, bits: f64, symbols: u32) -> CodingUnit {
        let mut cu = CodingUnit::new(x, y, w, h);
        cu.bits = bits;
        cu.symbols = symbols;
        cu
    }

    #[test]
    fn no_units_gives_a_zero_grid_of_the_right_shape() {
        let g = bit_grid_from_units(130, 70, &[]).unwrap();
        assert_eq!((g.grid_w, g.grid_h), (3, 2));
        assert_eq!(g.bits_per_pixel, vec![0.0; 6]);
        assert_eq!(g.symbols_per_bit, vec![0.0; 6]);
    }

    #[test]
    fn a_unit_inside_one_cell_gives_bits_over_the_visible_cell_area() {
        let g = bit_grid_from_units(128, 64, &[unit(0, 0, 32, 32, 4096.0, 512)]).unwrap();
        assert_eq!(g.bits_per_pixel, vec![4096.0 / 4096.0, 0.0]);
        assert_eq!(g.symbols_per_bit, vec![0.125, 0.0]);
    }

    #[test]
    fn partition_cost_is_included() {
        let mut cu = unit(0, 0, 64, 64, 1000.0, 100);
        cu.partition_bits = 24.0;
        cu.partition_symbols = 4.0;
        let g = bit_grid_from_units(64, 64, &[cu]).unwrap();
        assert_eq!(g.bits_per_pixel[0], 1024.0 / 4096.0);
        assert_eq!(g.symbols_per_bit[0], 104.0 / 1024.0);
    }

    /// A 128x128 block over four cells splits its cost evenly, and nothing is lost.
    #[test]
    fn a_unit_spanning_cells_is_split_by_overlapped_area() {
        let g = bit_grid_from_units(128, 128, &[unit(0, 0, 128, 128, 8000.0, 800)]).unwrap();
        assert_eq!(g.bits_per_pixel, vec![2000.0 / 4096.0; 4]);
        assert_eq!(g.symbols_per_bit, vec![0.1; 4]);
    }

    /// Cells at the frame edge divide by their visible area; a block reaching past the picture
    /// into the 8-aligned coded area still has all of its cost counted.
    #[test]
    fn edge_cells_and_blocks_past_the_picture_conserve_the_total() {
        let (w, h) = (66, 66);
        let units = [
            unit(0, 0, 64, 64, 1000.0, 10),
            unit(64, 0, 64, 64, 300.0, 3),
            unit(0, 64, 64, 64, 200.0, 2),
            unit(64, 64, 64, 64, 100.0, 1),
        ];
        let g = bit_grid_from_units(w, h, &units).unwrap();
        let visible = [4096.0, 2.0 * 64.0, 64.0 * 2.0, 4.0];
        let total: f64 = g
            .bits_per_pixel
            .iter()
            .zip(visible)
            .map(|(bpp, area)| bpp * area)
            .sum();
        assert!((total - 1600.0).abs() < 1e-9, "{total}");
        assert!(g.bits_per_pixel.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn a_unit_entirely_outside_the_coded_area_is_ignored() {
        let g = bit_grid_from_units(64, 64, &[unit(128, 128, 64, 64, 99.0, 9)]).unwrap();
        assert_eq!(g.bits_per_pixel, vec![0.0]);
    }
}
