//! Palette size and colour-cache contexts.

use super::TileContext;

impl TileContext {
    /// `has_palette_y` context (spec 5.11.46, real dav1d `t->a->pal_sz[bx4] > 0` / `t->l.pal_sz[
    /// by4] > 0`, gated by real `AvailU`/`AvailL` -- this crate's established tile-local
    /// `y4>0`/`x4>0` simplification, same as `segment_id_context`'s).
    pub fn has_palette_y_context(&self, x4: u32, y4: u32) -> u8 {
        let mut ctx = 0u8;
        if y4 > 0 && self.above_pal_sz[0].get(x4 as usize).copied().unwrap_or(0) > 0 {
            ctx += 1;
        }
        if x4 > 0 && self.left_pal_sz[0].get(y4 as usize).copied().unwrap_or(0) > 0 {
            ctx += 1;
        }
        ctx
    }

    /// Real stored above palette state for one color plane's cache derivation
    /// (`SymbolDecoder::read_palette_colors`'s doc) -- `color_plane` 0=Y/1=U/2=V, `size_plane`
    /// 0=Y/1=UV (U and V always share the same size). Returns `(colors, count)`.
    pub fn pal_above(&self, color_plane: usize, size_plane: usize, x4: u32) -> ([u16; 8], u8) {
        let count = self.above_pal_sz[size_plane.min(1)]
            .get(x4 as usize)
            .copied()
            .unwrap_or(0);
        let colors = self.above_pal_colors[color_plane.min(2)]
            .get(x4 as usize)
            .copied()
            .unwrap_or([0; 8]);
        (colors, count)
    }

    /// Real stored left palette state -- see `pal_above`'s doc.
    pub fn pal_left(&self, color_plane: usize, size_plane: usize, y4: u32) -> ([u16; 8], u8) {
        let count = self.left_pal_sz[size_plane.min(1)]
            .get(y4 as usize)
            .copied()
            .unwrap_or(0);
        let colors = self.left_pal_colors[color_plane.min(2)]
            .get(y4 as usize)
            .copied()
            .unwrap_or([0; 8]);
        (colors, count)
    }

    /// Record a CU's real palette size across its 4x4-unit footprint (`size_plane` 0=Y/1=UV, real
    /// dav1d's `al_pal`/`pal_sz` write always uses the CU's *luma* footprint even for chroma --
    /// `above_pal_sz`'s doc), for future `has_palette_y_context`/`pal_above`/`pal_left` lookups.
    pub fn set_pal_size(
        &mut self,
        size_plane: usize,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        sz: u8,
    ) {
        let size_plane = size_plane.min(1);
        let x_end = (x4 + width_4x4).min(self.above_pal_sz[size_plane].len() as u32);
        for x in x4..x_end {
            self.above_pal_sz[size_plane][x as usize] = sz;
        }
        let y_end = (y4 + height_4x4).min(self.left_pal_sz[size_plane].len() as u32);
        for y in y4..y_end {
            self.left_pal_sz[size_plane][y as usize] = sz;
        }
    }

    /// Record a CU's real decoded palette colors across its 4x4-unit (luma) footprint --
    /// `color_plane` 0=Y/1=U/2=V, see `set_pal_size`'s doc.
    pub fn set_pal_colors(
        &mut self,
        color_plane: usize,
        x4: u32,
        y4: u32,
        width_4x4: u32,
        height_4x4: u32,
        colors: [u16; 8],
    ) {
        let color_plane = color_plane.min(2);
        let x_end = (x4 + width_4x4).min(self.above_pal_colors[color_plane].len() as u32);
        for x in x4..x_end {
            self.above_pal_colors[color_plane][x as usize] = colors;
        }
        let y_end = (y4 + height_4x4).min(self.left_pal_colors[color_plane].len() as u32);
        for y in y4..y_end {
            self.left_pal_colors[color_plane][y as usize] = colors;
        }
    }
}
