//! Mutable per-symbol CDF accessors on `CdfContext`.

use super::*;

impl CdfContext {
    /// Get mutable `partition` CDF for `(block_size_log2, context)` -- mutable because
    /// `read_partition` adapts it in place via `update_cdf` after every read (see
    /// `partition_cdfs`'s doc: this is real context, not a representative placeholder).
    ///
    /// Block size is log2 of actual size:
    /// - 2 → 4x4
    /// - 3 → 8x8
    /// - 4 → 16x16
    /// - 5 → 32x32
    /// - 6 → 64x64
    /// - 7 → 128x128
    ///
    /// Context is 0..=3, from `crate::tile::TileContext::partition_context`.
    pub fn get_partition_cdf_mut(&mut self, block_size_log2: u8, ctx: u8) -> &mut [u16] {
        let index = (block_size_log2 as usize)
            .saturating_sub(2)
            .min(self.partition_cdfs.len() - 1);
        self.partition_cdfs[index][(ctx as usize).min(3)].as_mut_slice()
    }

    /// Get mutable skip flag CDF for the given context (0..=2, from
    /// `TileContext::skip_context`) -- mutable because `read_skip` adapts it in place via
    /// `update_cdf` after every read (see `skip_cdf`'s doc: this is real context, not a
    /// representative placeholder).
    pub fn get_skip_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.skip_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `skip_mode` CDF for the given context (0..=2, from
    /// `TileContext::skip_mode_context`).
    pub fn get_skip_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.skip_mode_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `is_inter` CDF for the given context (0..=3, from `TileContext::intra_ctx`).
    pub fn get_intra_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.intra_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable non-key-frame `y_mode` CDF for the given block-size-class context (0..=3, from
    /// `crate::tile::coding_unit::y_mode_size_context`).
    pub fn get_y_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.y_mode_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable `motion_mode` CDF for the given exact-block-size index (0..=16, from
    /// `motion_mode_size_index`).
    pub fn get_motion_mode_cdf_mut(&mut self, idx: u8) -> &mut [u16] {
        &mut self.motion_mode_cdf[(idx as usize).min(16)]
    }

    /// Get mutable `obmc` CDF for the given exact-block-size index (same indexing as
    /// `get_motion_mode_cdf_mut`).
    pub fn get_obmc_cdf_mut(&mut self, idx: u8) -> &mut [u16] {
        &mut self.obmc_cdf[(idx as usize).min(16)]
    }

    /// Get mutable `interintra` CDF for the given block-size-class context (0..=3, from
    /// `crate::tile::coding_unit::y_mode_size_context`).
    pub fn get_interintra_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.interintra_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable `interintra_mode` CDF for the given context (same indexing as
    /// `get_interintra_cdf_mut`).
    pub fn get_interintra_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.interintra_mode_cdf[(ctx as usize).min(3)]
    }

    /// Get mutable `interintra_wedge` CDF for the given wedge context (0..=6, from `wedge_ctx`
    /// capped to interintra's real 7-size-subset -- see that field's doc).
    pub fn get_interintra_wedge_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.interintra_wedge_cdf[(ctx as usize).min(6)]
    }

    /// Get mutable `wedge_comp` CDF for the given wedge context (0..=8, from `wedge_ctx`).
    pub fn get_wedge_comp_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.wedge_comp_cdf[(ctx as usize).min(8)]
    }

    /// Get mutable `wedge_idx` CDF for the given wedge context (0..=8, from `wedge_ctx`) --
    /// shared verbatim between compound wedge and `interintra_wedge`'s own index read.
    pub fn get_wedge_idx_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.wedge_idx_cdf[(ctx as usize).min(8)]
    }

    /// Get mutable `mask_comp` CDF for the given context (0..=5, from
    /// `TileContext::mask_comp_context`).
    pub fn get_mask_comp_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.mask_comp_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `jnt_comp` CDF for the given context (0..=5, from
    /// `TileContext::jnt_comp_context`).
    pub fn get_jnt_comp_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.jnt_comp_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `filter` CDF for the given direction (0/1) and context (0..=7, from
    /// `TileContext::filter_context`).
    pub fn get_filter_cdf_mut(&mut self, dir: u8, ctx: u8) -> &mut [u16] {
        &mut self.filter_cdf[(dir as usize).min(1)][(ctx as usize).min(7)]
    }

    /// Get mutable `drl_bit` CDF for the given context (0..=2, from
    /// `crate::tile::context::get_drl_context`).
    pub fn get_drl_bit_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.drl_bit_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `seg_pred` CDF for context `ctx` (0..=2 -- `TileContext::seg_pred_context`'s
    /// doc).
    pub fn get_seg_pred_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.seg_pred_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `segment_id` CDF for context `ctx` (0..=2 -- `TileContext::
    /// segment_id_context`'s doc).
    pub fn get_seg_id_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.seg_id_cdf[(ctx as usize).min(2)]
    }

    /// Get mutable `has_palette_y` CDF (`bsize_ctx` 0..=6, `ctx` 0..=2 -- see
    /// `TileContext::has_palette_context`'s doc).
    pub fn get_pal_y_cdf_mut(&mut self, bsize_ctx: u8, ctx: u8) -> &mut [u16] {
        &mut self.pal_y_cdf[(bsize_ctx as usize).min(6)][(ctx as usize).min(2)]
    }

    /// Get mutable `has_palette_uv` CDF (`ctx` 0..=1, `PaletteSizeY > 0`).
    pub fn get_pal_uv_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.pal_uv_cdf[(ctx as usize).min(1)]
    }

    /// Get mutable `palette_size_{y,uv}_minus_2` CDF (`plane` 0=y/1=uv, `bsize_ctx` 0..=6).
    pub fn get_pal_sz_cdf_mut(&mut self, plane: usize, bsize_ctx: u8) -> &mut [u16] {
        &mut self.pal_sz_cdf[plane.min(1)][(bsize_ctx as usize).min(6)]
    }

    /// Get mutable `color_map` CDF (`plane` 0=y/1=uv, `pal_sz` 2..=8, `ctx` 0..=4 -- see
    /// `SymbolDecoder::read_palette_index_map`'s doc).
    pub fn get_color_map_cdf_mut(&mut self, plane: usize, pal_sz: u8, ctx: u8) -> &mut [u16] {
        let pal_sz_idx = (pal_sz.max(2) - 2).min(6) as usize;
        &mut self.color_map_cdf[plane.min(1)][pal_sz_idx][(ctx as usize).min(4)]
    }

    /// Get mutable `angle_delta` CDF for a directional mode (`mode_minus_vert` 0..=7, i.e.
    /// `mode - V_PRED` -- shared by both `angle_delta_y` and `angle_delta_uv`, see field doc).
    pub fn get_angle_delta_cdf_mut(&mut self, mode_minus_vert: u8) -> &mut [u16] {
        &mut self.angle_delta_cdf[(mode_minus_vert as usize).min(7)]
    }

    /// Get mutable `uv_mode` CDF for `(cfl_allowed, y_mode)` (`y_mode` 0..=12 -- see field doc for
    /// why `cfl_allowed=0`/`1` are genuinely distinct alphabets, not the same table truncated).
    pub fn get_uv_mode_cdf_mut(&mut self, cfl_allowed: bool, y_mode: u8) -> &mut [u16] {
        &mut self.uv_mode_cdf[usize::from(cfl_allowed)][(y_mode as usize).min(12)]
    }

    /// Get mutable `cfl_alpha_signs` CDF (no context, see field doc).
    pub fn get_cfl_sign_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.cfl_sign_cdf
    }

    /// Get mutable `cfl_alpha_u`/`cfl_alpha_v` CDF for the given context (0..=5, see field doc).
    pub fn get_cfl_alpha_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.cfl_alpha_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `use_filter_intra` CDF for the given `BlockSize` (see field doc for the
    /// discriminant-order mapping).
    pub fn get_use_filter_intra_cdf_mut(&mut self, bs: crate::tile::BlockSize) -> &mut [u16] {
        &mut self.use_filter_intra_cdf[(bs as usize).min(23)]
    }

    /// Get mutable `filter_intra_mode` CDF (no context, see field doc).
    pub fn get_filter_intra_mode_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.filter_intra_mode_cdf
    }

    /// Get mutable `txfm_split` CDF for `(cat, ctx)` (`cat` 0..=6, `ctx` 0..=2 -- see
    /// `read_var_tx_size`'s doc and `TileContext::var_tx_context`) -- mutable because
    /// `read_txfm_split` adapts it in place via `update_cdf` after every read.
    pub fn get_txpart_cdf_mut(&mut self, cat: u8, ctx: u8) -> &mut [u16] {
        &mut self.txpart_cdf[(cat as usize).min(6)][(ctx as usize).min(2)]
    }

    /// Get mutable key-frame `intra_mode` CDF for the given context (`above_mode_class`,
    /// `left_mode_class`, each 0..=4 -- see `crate::tile::TileContext::intra_mode_context`) --
    /// mutable because `read_intra_mode` adapts it in place via `update_cdf` after every read.
    pub fn get_kfym_cdf_mut(&mut self, above_class: u8, left_class: u8) -> &mut [u16] {
        &mut self.kfym[(above_class as usize).min(4)][(left_class as usize).min(4)]
    }

    /// Get mutable `newmv_mode` CDF for the given context (0..=5, `ctx & 7` of
    /// `crate::tile::TileContext::inter_mode_context`'s packed result).
    pub fn get_newmv_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.newmv_mode_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `globalmv_mode` CDF for the given context (0..=1, `ctx >> 3 & 1`).
    pub fn get_globalmv_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.globalmv_mode_cdf[(ctx as usize).min(1)]
    }

    /// Get mutable `refmv_mode` CDF for the given context (0..=5, `ctx >> 4 & 15`).
    pub fn get_refmv_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.refmv_mode_cdf[(ctx as usize).min(5)]
    }

    /// Get mutable `compound_mode` CDF for the given context (0..=7, spec 5.11.24, see
    /// `crate::tile::TileContext::compound_mode_context`).
    pub fn get_compound_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.compound_mode_cdf[(ctx as usize).min(7)]
    }

    /// Get mutable MV joint CDF (4 symbols) -- mutable because `read_mv_joint` adapts it in place
    /// via `update_cdf` after every read (no neighbor context in the real spec either -- MV
    /// adaptation is frame-lifetime-global, unlike `skip`/`kfym`'s above/left context).
    ///
    /// - MV_JOINT_ZERO (both components zero)
    /// - MV_JOINT_HNZVZ (horizontal non-zero, vertical zero)
    /// - MV_JOINT_HZVNZ (horizontal zero, vertical non-zero)
    /// - MV_JOINT_HNZVNZ (both components non-zero)
    pub fn get_mv_joint_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.mv_joint_cdf
    }

    /// Get the mutable CDFs of one MV component (`0` = vertical, `1` = horizontal).
    pub fn get_mv_component_cdfs_mut(&mut self, component: usize) -> &mut MvComponentCdfs {
        &mut self.mv_comp[component.min(1)]
    }

    /// Get mutable `delta_q` CDF (spec 5.11.38) -- adapted after every read.
    pub fn get_delta_q_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.delta_q_cdf
    }

    /// Get mutable `delta_lf` CDF for the given real spec index (see
    /// `SymbolDecoder::read_delta_lf`'s doc) -- adapted after every read.
    pub fn get_delta_lf_cdf_mut(&mut self, cdf_index: usize) -> &mut [u16] {
        &mut self.delta_lf_cdf[cdf_index.min(4)]
    }

    /// Get mutable `txb_skip` (all_zero) CDF for one transform block. `tx_size_class`: 0..=4 (see
    /// `tx_size_class`). `ctx`: 0..=6 -- see `SymbolDecoder::read_residual_block`'s doc for when
    /// callers pass a real neighbor-derived value vs. the `0` fallback.
    pub fn get_txb_skip_cdf_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.txb_skip_cdf[tx_size_class.min(4)][(ctx as usize).min(6)]
    }

    /// Get mutable `eob_bin` CDF for a transform block of `tx_size_px` pixels per side
    /// (4/8/16/32/64) and `is_1d` (`SymbolDecoder::read_transform_type_is_1d`'s result). 64x64
    /// shares the 1024-coefficient class with 32x32 (real AV1 caps the coefficient scan at the
    /// top-left 32x32 sub-area for larger transforms) -- matches `tx_size_class`'s doc. The
    /// 1024-coefficient class has no real `is_1d` axis in rav1d (see `eob_bin_1024_cdf`'s doc),
    /// so `is_1d` is ignored there.
    /// `width_dim`/`height_dim`: transform dims in samples (already 32-capped by the caller, see
    /// `scan::scan_table`'s doc) -- selection is by *total area* (`width_dim*height_dim`), not
    /// either dim alone: real spec/dav1d select `eob_bin`'s default CDF row by total coefficient
    /// count (`16/32/64/128/256/512/1024`), which is symmetric in width/height (a 4x8 and an 8x4
    /// transform share the same row) -- verified against dav1d's `default_coef_cdf`'s
    /// `eob_bin_16/32/64/128/256/512/1024` field names directly, not assumed. `eob_bin_512`/
    /// `_1024` have no real `is_1d` axis (see their field docs).
    pub fn get_eob_bin_cdf_mut(
        &mut self,
        width_dim: u32,
        height_dim: u32,
        is_1d: bool,
    ) -> &mut [u16] {
        let is_1d = usize::from(is_1d);
        match width_dim * height_dim {
            0..=16 => &mut self.eob_bin_16_cdf[is_1d],
            17..=32 => &mut self.eob_bin_32_cdf[is_1d],
            33..=64 => &mut self.eob_bin_64_cdf[is_1d],
            65..=128 => &mut self.eob_bin_128_cdf[is_1d],
            129..=256 => &mut self.eob_bin_256_cdf[is_1d],
            257..=512 => &mut self.eob_bin_512_cdf,
            _ => &mut self.eob_bin_1024_cdf,
        }
    }

    /// Get mutable `eob_hi_bit` CDF -- the single context-coded first bit of `eob`'s extra-bits
    /// suffix (see `eob_hi_bit_cdf`'s doc). `tx_size_class`: 0..=4 (4x4..64x64, see
    /// `tx_size_class`). `eob_bin`: the symbol just read from `get_eob_bin_cdf_mut`'s CDF (0..=10).
    pub fn get_eob_hi_bit_cdf_mut(&mut self, tx_size_class: usize, eob_bin: u8) -> &mut [u16] {
        &mut self.eob_hi_bit_cdf[tx_size_class.min(4)][(eob_bin as usize).min(10)]
    }

    /// Get mutable `coeff_base_eob` CDF (level 1..=3 for the highest-scan-order nonzero
    /// coefficient), real context (`tx_size_class` 0..=4, `ctx` 0..=3 -- see
    /// `SymbolDecoder::coeff_base_eob_context`'s doc).
    pub fn get_coeff_base_eob_cdf_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_base_eob_cdf[tx_size_class.min(4)][(ctx as usize).min(3)]
    }

    /// Get mutable chroma `txb_skip` CDF -- see `txb_skip_cdf_chroma`'s doc for scope
    /// (`tx_size_class` 0..=3, `ctx` 0..=5 -- see `TileContext::txb_skip_context_chroma`'s doc).
    pub fn get_txb_skip_cdf_chroma_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.txb_skip_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(5)]
    }

    /// Get mutable chroma `dc_sign` CDF -- real 3-context chroma default (see
    /// `dc_sign_cdf_chroma`'s doc), `ctx` 0..=2 (`TileContext::dc_sign_context_chroma`'s doc).
    pub fn get_dc_sign_cdf_chroma_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.dc_sign_cdf_chroma[(ctx as usize).min(2)]
    }

    /// Get mutable chroma `eob_bin` CDF for a `width_px`x`height_px` chroma transform tile (each
    /// independently `<=32`, `txb_skip_cdf_chroma`'s doc) -- selection by *total area*, same
    /// symmetric-in-width/height reasoning as `get_eob_bin_cdf_mut`'s doc. `is_1d` is always
    /// `false` for chroma in this crate's scope, so there's no axis for it (unlike
    /// `get_eob_bin_cdf_mut`).
    pub fn get_eob_bin_cdf_chroma_mut(
        &mut self,
        width_px: u32,
        height_px: u32,
        is_1d: bool,
    ) -> &mut [u16] {
        let axis = usize::from(is_1d);
        match width_px * height_px {
            0..=16 => &mut self.eob_bin_16_cdf_chroma[axis],
            17..=32 => &mut self.eob_bin_32_cdf_chroma[axis],
            33..=64 => &mut self.eob_bin_64_cdf_chroma[axis],
            65..=128 => &mut self.eob_bin_128_cdf_chroma[axis],
            129..=256 => &mut self.eob_bin_256_cdf_chroma[axis],
            257..=512 => &mut self.eob_bin_512_cdf_chroma,
            _ => &mut self.eob_bin_1024_cdf_chroma, // 1024, this crate's chroma scope max
        }
    }

    /// Get mutable chroma `eob_hi_bit` CDF. `tx_size_class`: 0..=3. `eob_bin`: same real,
    /// dynamic indexing as luma's `get_eob_hi_bit_cdf_mut`.
    pub fn get_eob_hi_bit_cdf_chroma_mut(
        &mut self,
        tx_size_class: usize,
        eob_bin: u8,
    ) -> &mut [u16] {
        &mut self.eob_hi_bit_cdf_chroma[tx_size_class.min(3)][(eob_bin as usize).min(10)]
    }

    /// Get mutable chroma `coeff_base_eob` CDF. `tx_size_class`: 0..=3. `ctx`: same real formula
    /// as luma's (`SymbolDecoder::coeff_base_eob_context`, plane-agnostic).
    pub fn get_coeff_base_eob_cdf_chroma_mut(
        &mut self,
        tx_size_class: usize,
        ctx: u8,
    ) -> &mut [u16] {
        &mut self.coeff_base_eob_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(3)]
    }

    /// Get mutable chroma `coeff_base` CDF. `tx_size_class`: 0..=3. `ctx`: 0..=40, from
    /// `symbol::scan::lo_ctx` (same real formula as luma's, plane-agnostic -- only the default
    /// CDF values differ).
    pub fn get_coeff_base_cdf_chroma_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_base_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(40)]
    }

    /// Get mutable chroma `coeff_br` CDF. `tx_size_class`: 0..=3 (matches `coeff_base_cdf_chroma`
    /// exactly here, unlike luma's `min(tx_size_class,3)` cap against a 5-bucket table -- this
    /// crate's chroma table only ever has 4 buckets, 0..=3 is already the whole range). `ctx`:
    /// 0..=20.
    pub fn get_coeff_br_cdf_chroma_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_br_cdf_chroma[tx_size_class.min(3)][(ctx as usize).min(20)]
    }

    /// Get mutable `txtp_intra2` CDF (spec 5.11.47's reduced intra `transform_type()` alphabet).
    /// `tx_size_class`: only 0..=2 (4x4/8x8/16x16) are ever reached -- see
    /// `SymbolDecoder::read_transform_type_is_1d`'s doc.
    pub fn get_txtp_intra2_cdf_mut(&mut self, tx_size_class: usize, y_mode: u8) -> &mut [u16] {
        &mut self.txtp_intra2_cdf[tx_size_class.min(2)][(y_mode as usize).min(12)]
    }

    /// Get mutable `txtp_intra1` CDF (spec 5.11.47's full intra `transform_type()` alphabet).
    /// `tx_size_class`: only 0..=1 (4x4/8x8) are ever reached.
    pub fn get_txtp_intra1_cdf_mut(&mut self, tx_size_class: usize, y_mode: u8) -> &mut [u16] {
        &mut self.txtp_intra1_cdf[tx_size_class.min(1)][(y_mode as usize).min(12)]
    }

    /// Get mutable `txtp_inter3` CDF (spec 5.11.47's reduced/32x32 inter `transform_type()`
    /// alphabet). `tx_size_class`: only 0..=3 (4x4/8x8/16x16/32x32) are ever reached.
    pub fn get_txtp_inter3_cdf_mut(&mut self, tx_size_class: usize) -> &mut [u16] {
        &mut self.txtp_inter3_cdf[tx_size_class.min(3)]
    }

    /// Get mutable `txtp_inter2` CDF (spec 5.11.47's 16x16 inter `transform_type()` alphabet --
    /// only ever reached at that one tx size, so no context axis).
    pub fn get_txtp_inter2_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.txtp_inter2_cdf
    }

    /// Get mutable `txtp_inter1` CDF (spec 5.11.47's full inter `transform_type()` alphabet).
    /// `tx_size_class`: only 0..=1 (4x4/8x8) are ever reached.
    pub fn get_txtp_inter1_cdf_mut(&mut self, tx_size_class: usize) -> &mut [u16] {
        &mut self.txtp_inter1_cdf[tx_size_class.min(1)]
    }

    /// Get mutable `tx_size()` depth CDF. `max_tx_class`: 1..=4 (4x4/class 0 never reaches this,
    /// see `SymbolDecoder::read_tx_size`'s doc). `ctx`: 0..=2, see
    /// `crate::tile::TileContext::tx_size_context`.
    pub fn get_txsz_cdf_mut(&mut self, max_tx_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.txsz_cdf[max_tx_class.clamp(1, 4) - 1][(ctx as usize).min(2)]
    }

    /// Get mutable `coeff_base` CDF (level 0..=3 for every other coefficient position).
    /// `tx_size_class`: 0..=4 (see `tx_size_class`). `ctx`: 0..=40, from `symbol::scan::lo_ctx`.
    pub fn get_coeff_base_cdf_mut(&mut self, tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_base_cdf[tx_size_class.min(4)][(ctx as usize).min(40)]
    }

    /// Get mutable `coeff_br` CDF (range-extension increment 0..=3). `capped_tx_size_class`:
    /// 0..=3 (`tx_size_class(..).min(3)`, one fewer bucket than `coeff_base_cdf` -- see this
    /// field's doc). `ctx`: 0..=20.
    pub fn get_coeff_br_cdf_mut(&mut self, capped_tx_size_class: usize, ctx: u8) -> &mut [u16] {
        &mut self.coeff_br_cdf[capped_tx_size_class.min(3)][(ctx as usize).min(20)]
    }

    /// Get mutable `dc_sign` CDF (sign of the DC coefficient). `ctx`: 0..=2 -- see
    /// `SymbolDecoder::read_residual_block`'s doc for when callers pass a real neighbor-derived
    /// value vs. the `0` fallback.
    pub fn get_dc_sign_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.dc_sign_cdf[(ctx as usize).min(2)]
    }

    /// Get `comp_mode` CDF (single-reference vs compound prediction), real context (0..=4, see
    /// `crate::tile::TileContext::comp_mode_context`) + adaptation, like `get_skip_cdf_mut`.
    pub fn get_comp_mode_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_mode_cdf[(ctx as usize).min(4)]
    }
    /// Get `single_ref_p1` CDF (forward vs backward reference group), real context (0..=2, see
    /// `crate::tile::TileContext::single_ref_p1_context`) + adaptation.
    pub fn get_single_ref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p2` CDF (BWDREF/ALTREF2 group vs ALTREF, backward branch), real context.
    pub fn get_single_ref_p2_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p2_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p3` CDF (LAST/LAST2 group vs LAST3/GOLDEN group, forward branch), real
    /// context.
    pub fn get_single_ref_p3_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p3_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p4` CDF (LAST vs LAST2), real context.
    pub fn get_single_ref_p4_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p4_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p5` CDF (LAST3 vs GOLDEN), real context.
    pub fn get_single_ref_p5_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p5_cdf[(ctx as usize).min(2)]
    }
    /// Get `single_ref_p6` CDF (BWDREF vs ALTREF2), real context.
    pub fn get_single_ref_p6_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.single_ref_p6_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref_type` CDF (unidirectional vs bidirectional compound reference), real context
    /// (0..=4, see `crate::tile::TileContext::comp_ref_type_context`).
    pub fn get_comp_ref_type_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_type_cdf[(ctx as usize).min(4)]
    }
    /// Get `uni_comp_ref` CDF ((LAST,LAST2)/(LAST,LAST3-or-GOLDEN) pair vs (BWDREF,ALTREF)), real
    /// context (reuses `single_ref_p1_context`'s formula per rav1d, see `read_ref_frames`).
    pub fn get_uni_comp_ref_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.uni_comp_ref_cdf[(ctx as usize).min(2)]
    }
    /// Get `uni_comp_ref_p1` CDF ((LAST,LAST2) vs (LAST,LAST3-or-GOLDEN)), real context (see
    /// `crate::tile::TileContext::uni_comp_ref_p1_context`).
    pub fn get_uni_comp_ref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.uni_comp_ref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `uni_comp_ref_p2` CDF (LAST3 vs GOLDEN, second slot of the unidirectional pair), real
    /// context (reuses `single_ref_p5_context`'s formula per rav1d).
    pub fn get_uni_comp_ref_p2_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.uni_comp_ref_p2_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref` CDF (forward group choice, bidirectional compound), real context (reuses
    /// `single_ref_p3_context`'s formula per rav1d).
    pub fn get_comp_ref_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref_p1` CDF (LAST vs LAST2, bidirectional compound forward ref), real context
    /// (reuses `single_ref_p4_context`'s formula per rav1d).
    pub fn get_comp_ref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_ref_p2` CDF (LAST3 vs GOLDEN, bidirectional compound forward ref), real context
    /// (reuses `single_ref_p5_context`'s formula per rav1d).
    pub fn get_comp_ref_p2_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_ref_p2_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_bwdref` CDF (backward group choice, bidirectional compound), real context
    /// (reuses `single_ref_p2_context`'s formula per rav1d).
    pub fn get_comp_bwdref_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_bwdref_cdf[(ctx as usize).min(2)]
    }
    /// Get `comp_bwdref_p1` CDF (BWDREF vs ALTREF2, bidirectional compound backward ref), real
    /// context (reuses `single_ref_p6_context`'s formula per rav1d).
    pub fn get_comp_bwdref_p1_cdf_mut(&mut self, ctx: u8) -> &mut [u16] {
        &mut self.comp_bwdref_p1_cdf[(ctx as usize).min(2)]
    }
    /// Get `use_intrabc` CDF (intra block copy flag).
    pub fn get_use_intrabc_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.use_intrabc_cdf
    }

    /// Get the `restore_switchable` CDF (3 symbols: NONE, WIENER, SGRPROJ).
    pub fn get_restore_switchable_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.restore_switchable_cdf
    }

    /// Get the `use_wiener` flag CDF.
    pub fn get_restore_wiener_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.restore_wiener_cdf
    }

    /// Get the `use_sgrproj` flag CDF.
    pub fn get_restore_sgrproj_cdf_mut(&mut self) -> &mut [u16] {
        &mut self.restore_sgrproj_cdf
    }
}
