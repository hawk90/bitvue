//! Per-block residual "energy" grid extraction -- real, data-driven replacement for the
//! QP-only Efficiency Map proxy that used to live entirely in the frontend
//! (`Av1EfficiencyMapRenderer.tsx`: `bpp ≈ (64 - QP) * scale_factor`).
//!
//! `energy_bpp` per cell is `sum_abs_level / block_area` (see
//! `crate::symbol::ResidualBlockStats`'s doc) -- still not literal entropy-coded bit count
//! (this crate's residual decode uses context-independent representative CDFs, not the real
//! neighbor-context-adaptive ones, per `SymbolDecoder::read_residual_block`'s doc), but unlike
//! the QP proxy it reflects each block's actual decoded residual magnitude rather than just the
//! frame's quantization step, so "hot"/"cold" comparisons between blocks are now data-driven.
//! Reuses the exact `parse_all_coding_units`/`CuSpatialIndex` machinery `qp_extractor.rs` already
//! uses for the same grid-cell-to-CU mapping problem.

use super::cu_parser::{parse_all_coding_units, CuSpatialIndex};
use super::parser::ParsedFrame;
use crate::ivf::OVERLAY_BLOCK_SIZE;
use bitvue_engine::BitvueError;

/// Dense grid of per-block residual-energy values, same cell layout as `QPGrid`
/// (`bitvue_engine::qp_heatmap::QPGrid`) so the frontend can reuse the same grid-indexing code.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyGrid {
    pub grid_w: u32,
    pub grid_h: u32,
    pub block_w: u32,
    pub block_h: u32,
    /// Bits-per-pixel-like energy value per cell. 0.0 is a real "no residual" value (skip
    /// blocks, or no tile data to parse), not a missing-data marker -- every cell always has a
    /// value.
    pub energy_bpp: Vec<f64>,
}

/// Extract an energy grid from an already-parsed frame. Falls back to all-zero cells (rather
/// than erroring) when tile data is absent or CU parsing fails, matching
/// `extract_qp_grid_from_parsed`'s fallback-to-scaffold behavior for the same conditions.
pub fn extract_energy_grid_from_parsed(parsed: &ParsedFrame) -> Result<EnergyGrid, BitvueError> {
    let block_w = OVERLAY_BLOCK_SIZE;
    let block_h = OVERLAY_BLOCK_SIZE;
    let grid_w = parsed.dimensions.width.div_ceil(block_w);
    let grid_h = parsed.dimensions.height.div_ceil(block_h);

    let total_blocks = grid_w.checked_mul(grid_h).ok_or_else(|| {
        BitvueError::Decode(format!("Grid dimensions too large: {}x{}", grid_w, grid_h))
    })? as usize;

    if !parsed.has_tile_data() || parsed.tile_data.len() <= 10 {
        return Ok(EnergyGrid {
            grid_w,
            grid_h,
            block_w,
            block_h,
            energy_bpp: vec![0.0; total_blocks],
        });
    }

    let coding_units = match parse_all_coding_units(parsed) {
        Ok(cus) => cus,
        Err(e) => {
            tracing::warn!(
                "Failed to parse coding units for energy grid: {}, using zeros",
                e
            );
            return Ok(EnergyGrid {
                grid_w,
                grid_h,
                block_w,
                block_h,
                energy_bpp: vec![0.0; total_blocks],
            });
        }
    };

    let spatial_index = CuSpatialIndex::new(&coding_units, grid_w, grid_h, block_w, block_h);
    let mut energy_bpp = Vec::with_capacity(total_blocks);
    for grid_y in 0..grid_h {
        for grid_x in 0..grid_w {
            let val = spatial_index
                .get_cu_index(grid_x, grid_y)
                .map(|idx| {
                    let cu = &coding_units[idx];
                    let area = cu.width as f64 * cu.height as f64;
                    if area <= 0.0 {
                        0.0
                    } else {
                        cu.residual
                            .map(|r| r.sum_abs_level as f64 / area)
                            .unwrap_or(0.0)
                    }
                })
                .unwrap_or(0.0);
            energy_bpp.push(val);
        }
    }

    Ok(EnergyGrid {
        grid_w,
        grid_h,
        block_w,
        block_h,
        energy_bpp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal OBU stream (temporal delimiter + placeholder sequence/frame headers, no real
    /// tile data) -- mirrors `qp_extractor.rs`'s `create_test_obu_data` helper.
    fn create_test_obu_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x12, 0x00]); // Temporal delimiter OBU
        data.extend_from_slice(&[0x0A, 0x14]); // Sequence header OBU header
        data.extend_from_slice(&[0x00u8; 20]); // Payload placeholder
        data.extend_from_slice(&[0x1A, 0x0A]); // Frame header OBU header
        data.extend_from_slice(&[0x00u8; 10]); // Payload placeholder
        data
    }

    #[test]
    fn energy_grid_falls_back_to_zeros_without_tile_data() {
        // No real tile data in this minimal stream -- should produce an all-zero grid of the
        // scaffold dimensions rather than erroring.
        let obu_data = create_test_obu_data();
        let parsed = ParsedFrame::parse(&obu_data).expect("scaffold parse should succeed");
        let grid = extract_energy_grid_from_parsed(&parsed).unwrap();
        assert_eq!(grid.grid_w, grid.energy_bpp.len() as u32 / grid.grid_h);
        assert!(!grid.energy_bpp.is_empty());
        assert!(grid.energy_bpp.iter().all(|&v| v == 0.0));
    }
}
