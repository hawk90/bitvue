//! Motion-vector symbols: `mv_joint` and one component's difference (spec 5.11.32).

use super::*;

impl<'a> SymbolDecoder<'a> {
    /// Read `mv_joint` (AV1 spec Section 5.11.32 `read_mv()`) -- gates which axis, if any, has a
    /// coded component to read next. Returns one of the `MV_JOINT_*` values (see
    /// `CdfContext::get_mv_joint_cdf_mut`'s doc): 0=both zero, 1=horizontal only, 2=vertical
    /// only, 3=both. Real adaptation via `read_symbol_adaptive` -- MV CDFs have no above/left
    /// neighbor context in the real spec (unlike `skip`/`kfym`), just frame-lifetime adaptation.
    pub fn read_mv_joint(&mut self) -> Result<u8> {
        let cdf = self.cdf_context.get_mv_joint_cdf_mut();
        self.decoder.read_symbol_adaptive(cdf)
    }

    /// Read one MV component's difference (dav1d `read_mv_component_diff`, `src/decode.c`), in
    /// 1/8-sample units, sign applied. `component`: `0` = vertical, `1` = horizontal. `mv_prec`:
    /// `-1` when the frame uses integer MVs (no fractional bits are coded), `0` for quarter-sample
    /// precision (the high-precision bit is not coded and reads as 1), `1` for eighth-sample.
    pub fn read_mv_component_diff(&mut self, component: usize, mv_prec: i8) -> Result<i32> {
        let cdfs = self.cdf_context.get_mv_component_cdfs_mut(component);
        let sign = self.decoder.read_symbol_adaptive(&mut cdfs.sign)? == 1;
        let class = self.decoder.read_symbol_adaptive(&mut cdfs.classes)? as usize;
        let (mut fp, mut hp) = (3i32, 1i32);
        let up: i32;
        if class == 0 {
            up = self.decoder.read_symbol_adaptive(&mut cdfs.class0)? as i32;
            if mv_prec >= 0 {
                fp = self
                    .decoder
                    .read_symbol_adaptive(&mut cdfs.class0_fp[up as usize])?
                    as i32;
                if mv_prec > 0 {
                    hp = self.decoder.read_symbol_adaptive(&mut cdfs.class0_hp)? as i32;
                }
            }
        } else {
            let mut bits = 1i32 << class;
            for n in 0..class {
                bits |= (self.decoder.read_symbol_adaptive(&mut cdfs.class_n[n])? as i32) << n;
            }
            up = bits;
            if mv_prec >= 0 {
                fp = self.decoder.read_symbol_adaptive(&mut cdfs.class_n_fp)? as i32;
                if mv_prec > 0 {
                    hp = self.decoder.read_symbol_adaptive(&mut cdfs.class_n_hp)? as i32;
                }
            }
        }
        let diff = ((up << 3) | (fp << 1) | hp) + 1;
        Ok(if sign { -diff } else { diff })
    }
}
