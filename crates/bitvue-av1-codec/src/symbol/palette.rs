//! Palette symbols: `has_palette_y/uv`, `palette_size` and colour-map indices.

use super::*;

impl<'a> SymbolDecoder<'a> {
    /// Read `has_palette_y` (spec 5.11.46) -- real per-`(bsize_ctx, ctx)` CDF + adaptation.
    /// `bsize_ctx`/`ctx`: `crate::tile::coding_unit::read_palette_mode_info`'s doc.
    pub fn read_has_palette_y(&mut self, bsize_ctx: u8, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_pal_y_cdf_mut(bsize_ctx, ctx);
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `has_palette_uv` -- real per-`ctx` CDF + adaptation (`ctx`: `PaletteSizeY > 0`).
    pub fn read_has_palette_uv(&mut self, ctx: u8) -> Result<bool> {
        let cdf = self.cdf_context.get_pal_uv_cdf_mut(ctx);
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `palette_size_{y,uv}_minus_2` -- real per-`(plane, bsize_ctx)` CDF + adaptation.
    /// `plane`: 0=y/1=uv. Returns the real palette size (`symbol + 2`, range 2..=8).
    pub fn read_pal_size(&mut self, plane: usize, bsize_ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_pal_sz_cdf_mut(plane, bsize_ctx);
        Ok(self.decoder.read_symbol_adaptive(cdf)? + 2)
    }

    /// Read one `color_map` (palette pixel index) symbol -- real per-`(plane, pal_sz, ctx)` CDF +
    /// adaptation (`SymbolDecoder::read_palette_index_map`'s doc for `ctx`'s real derivation).
    /// `plane`: 0=y/1=uv. Alphabet size `pal_sz - 1` (real spec: `pal_sz` symbols, 0..=`pal_sz-1`).
    pub fn read_color_map_index(&mut self, plane: usize, pal_sz: u8, ctx: u8) -> Result<u8> {
        let cdf = self.cdf_context.get_color_map_cdf_mut(plane, pal_sz, ctx);
        self.decoder.read_symbol_adaptive(cdf)
    }
}
