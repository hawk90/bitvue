//! Frame-header flag bundles `parse_coding_unit` takes.

/// Frame header flags `SymbolDecoder::read_transform_type_is_1d`/`read_tx_size` need, bundled to
/// avoid growing `parse_coding_unit`'s already-long parameter list further -- see `ParsedFrame`'s
/// doc for how `coded_lossless`/`reduced_tx_set`/`txfm_mode` are sourced, and
/// `read_transform_type_is_1d`'s doc for why `qidx_is_zero` is a distinct condition from
/// `coded_lossless` (the real spec shortcut checks `base_q_idx == 0` alone, without also
/// requiring zero delta-Q).
///
/// `mono_chrome`/`subsampling_x`/`subsampling_y` are threaded through here too (sourced from
/// `ParsedFrame`'s fields of the same name), gating `parse_coding_unit`'s chroma residual read
/// (`SymbolDecoder::read_chroma_residual_block`, see its doc for scope and the regression this
/// piece's first attempt hit before landing on real chroma-specific default CDF values).
#[derive(Debug, Clone, Copy)]
pub struct TxTypeFrameFlags {
    pub coded_lossless: bool,
    pub qidx_is_zero: bool,
    pub reduced_tx_set: bool,
    pub txfm_mode: crate::frame_header::TxfmMode,
    pub mono_chrome: bool,
    pub subsampling_x: bool,
    pub subsampling_y: bool,
}

/// Frame-level flags gating the real `motion_mode`/`interintra`/`compound_type`(wedge/seg)/
/// `filter` reads (spec 5.11.27-30) -- see `SymbolDecoder::read_motion_mode`/`read_interintra`/
/// `read_mask_comp`/`read_filter`'s docs for what each gates. Bundled the same way
/// `TxTypeFrameFlags` bundles its own frame-level gates, to avoid a further parameter-list
/// explosion on `parse_coding_unit`.
#[derive(Debug, Clone, Copy)]
pub struct InterModeFlags {
    pub switchable_motion_mode: bool,
    pub allow_warped_motion: bool,
    pub enable_interintra_compound: bool,
    /// Sequence header's `enable_dual_filter`: whether `interp_filter` codes one filter per axis
    /// (two symbols) or a single one shared by both.
    pub enable_dual_filter: bool,
    pub enable_masked_compound: bool,
    pub enable_jnt_comp: bool,
    pub subpel_filter_switchable: bool,
    /// `force_integer_mv`/`gm_type` (spec 5.9.2/5.9.24) -- gate `read_motion_mode`'s real
    /// `GmType[RefFrame[0]] > TRANSLATION` exclusion for `GLOBALMV`/`GLOBAL_GLOBALMV` blocks (see
    /// the call site's doc). `gm_type` indexed by `RefFrame as usize` (0=Intra unused).
    pub force_integer_mv: bool,
    /// `allow_high_precision_mv` (spec 5.9.2): with `force_integer_mv`, decides how many
    /// fractional MV bits are coded -- see [`InterModeFlags::mv_precision`].
    pub allow_high_precision_mv: bool,
    pub gm_type: [u8; 8],
}

impl InterModeFlags {
    /// dav1d's `mv_prec` (`hp - force_integer_mv`): `-1` integer MVs, `0` quarter-sample, `1`
    /// eighth-sample.
    pub fn mv_precision(&self) -> i8 {
        i8::from(self.allow_high_precision_mv) - i8::from(self.force_integer_mv)
    }
}
