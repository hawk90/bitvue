//! Loop-restoration symbols: `use_wiener`, `use_sgrproj` and the switchable type.

use super::*;

impl<'a> SymbolDecoder<'a> {
    /// Read `restore_switchable` (spec 5.11.58): 0=NONE, 1=WIENER, 2=SGRPROJ.
    pub fn read_restore_switchable(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_restore_switchable_cdf_mut();
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read `use_wiener` (spec 5.11.58) -- whether a restoration unit of a WIENER frame filters.
    pub fn read_restore_wiener(&mut self) -> Result<bool> {
        let cdf = self.cdf_context.get_restore_wiener_cdf_mut();
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }

    /// Read `use_sgrproj` (spec 5.11.58) -- whether a restoration unit of a SGRPROJ frame filters.
    pub fn read_restore_sgrproj(&mut self) -> Result<bool> {
        let cdf = self.cdf_context.get_restore_sgrproj_cdf_mut();
        Ok(self.decoder.read_symbol_adaptive(cdf)? == 1)
    }
}
