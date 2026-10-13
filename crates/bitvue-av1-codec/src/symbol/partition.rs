//! `partition` symbol reading (spec 5.11.4).

use super::*;

impl<'a> SymbolDecoder<'a> {
    /// Read a `partition` symbol (spec 5.11.4), per its real above/left context (`ctx`, 0..=3,
    /// from `crate::tile::TileContext::partition_context`) -- real per-context default CDFs
    /// (`CdfContext`'s `partition_cdfs` doc) and real adaptation via `read_symbol_adaptive`,
    /// matching `read_skip`/`read_intra_mode`'s bar.
    ///
    /// `has_rows`/`has_cols` (frame-edge legality, from `tile::partition::parse_partition_recursive`'s
    /// real `MiRows`/`MiCols` check) select which of spec 5.11.4's four branches this read takes:
    /// - both true: full alphabet, as above (real context + adaptation).
    /// - `has_cols` only: reduced binary `split_or_horz` (HORZ vs SPLIT), via
    ///   `cdf::split_or_horz_prob`'s aggregated probability -- non-adaptive (see that fn's doc for
    ///   why: rav1d's equivalent never writes back to the real `partition_cdfs` entry).
    /// - `has_rows` only: reduced binary `split_or_vert` (VERT vs SPLIT), symmetric.
    /// - neither: implicit `PARTITION_SPLIT`, **no symbol read at all** -- getting this branch
    ///   wrong (e.g. reading anything here) would desync the shared `SymbolDecoder` for the rest
    ///   of the tile, the same bug shape as this session's `residual()`/`ref_frame()`/`mv_joint`
    ///   fixes.
    ///
    /// Returns partition type (0-9) for current block context.
    pub fn read_partition(
        &mut self,
        block_size_log2: u8,
        ctx: u8,
        has_rows: bool,
        has_cols: bool,
    ) -> Result<u8> {
        if has_rows && has_cols {
            let cdf = self.cdf_context.get_partition_cdf_mut(block_size_log2, ctx);
            return self.decoder.read_symbol_adaptive(cdf);
        }
        // Raw partition symbol values (spec order, matching `tile::PartitionType`'s numeric
        // repr): NONE=0, HORZ=1, VERT=2, SPLIT=3. Not importing `PartitionType` itself here to
        // avoid a `symbol -> tile` dependency alongside `tile`'s existing `-> symbol` one.
        const HORZ: u8 = 1;
        const VERT: u8 = 2;
        const SPLIT: u8 = 3;
        if !has_rows && !has_cols {
            return Ok(SPLIT);
        }
        let cdf = self.cdf_context.get_partition_cdf_mut(block_size_log2, ctx);
        if has_cols {
            let psum = cdf::split_or_horz_prob(cdf);
            let bin_cdf = [psum, 0, 0];
            let is_split = self.decoder.read_symbol(&bin_cdf)? == 1;
            Ok(if is_split { SPLIT } else { HORZ })
        } else {
            let psum = cdf::split_or_vert_prob(cdf);
            let bin_cdf = [psum, 0, 0];
            let is_split = self.decoder.read_symbol(&bin_cdf)? == 1;
            Ok(if is_split { SPLIT } else { VERT })
        }
    }
}
