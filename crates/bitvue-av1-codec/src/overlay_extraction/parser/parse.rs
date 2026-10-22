//! `ParsedFrame::parse` and `parse_with_ref_state`: from raw OBU data to a `ParsedFrame`.

use super::{FrameDimensions, FrameTypeInfo, ObuRef, ParsedFrame};
use crate::frame_header::TxfmMode;
use crate::frame_header_full::{parse_frame_header_full, RefFrameState};
use crate::{parse_all_obus, parse_frame_header_basic, ObuType};
use bitvue_engine::BitvueError;
use std::sync::Arc;

impl ParsedFrame {
    /// Parse OBU data and cache all relevant information
    ///
    /// This is the main entry point for overlay extraction.
    /// Call this once, then use the cached data for all extractions.
    ///
    /// # Performance
    ///
    /// - O(n) where n is the OBU data size
    /// - Parses each OBU exactly once
    /// - Stores references to avoid copying payload data
    ///
    /// # Example
    ///
    /// ```ignore
    /// let parsed = ParsedFrame::parse(&obu_data)?;
    /// let qp_grid = extract_qp_grid_from_parsed(&parsed, frame_idx, base_qp)?;
    /// let mv_grid = extract_mv_grid_from_parsed(&parsed, frame_idx)?;
    /// ```
    ///
    /// **Correctness note (axis-7 fix, 2026-08-20):** this parses `obu_data` against a *fresh*
    /// `RefFrameState` (`RefFrameState::new()`), which is only correct for `obu_data`'s frame if
    /// it's frame 0 (or otherwise doesn't need real cross-frame ref-order-hint state to interpret
    /// its own header -- e.g. `skip_mode_params()`'s presence bit, spec 5.9.22). For any other
    /// frame, a real fixture's `header_size_bytes` can come out wrong under a fresh state, which
    /// silently shifts `tile_data`'s start into what's still frame-header bits -- the entropy
    /// decoder then desyncs from its very first read, and every superblock in the frame fails to
    /// parse (caught per-superblock, so the *symptom* is a silently near-empty coding-unit list,
    /// not a propagated error). Confirmed via a real fixture: frames needing `skip_mode_params`'s
    /// real state return 0 coding units this way despite non-trivial `tile_data` length. Callers
    /// that need correct `tile_data`/coding-units for anything other than frame 0 (i.e. every
    /// `bitvue-sidecar` command backing a per-frame grid) MUST use
    /// [`ParsedFrame::parse_with_ref_state`] with a `RefFrameState` threaded from frame 0, not
    /// this method.
    pub fn parse(obu_data: &[u8]) -> Result<Self, BitvueError> {
        Self::parse_with_ref_state(obu_data, &mut RefFrameState::new())
    }

    /// A frame with nothing parsed yet: what the parse returns for empty data (the default
    /// 1920x1080 scaffold the extractors fall back to), and where every other parse starts from.
    pub(super) fn scaffold(obu_data: Arc<[u8]>) -> Self {
        Self {
            obu_data,
            obus: Vec::new(),
            dimensions: FrameDimensions::default(),
            frame_type: FrameTypeInfo::default(),
            tile_data: Arc::from([]),
            delta_q_enabled: false,
            reference_select: false,
            allow_intrabc: false,
            allow_screen_content_tools: false,
            delta_lf_present: false,
            delta_lf_multi: false,
            reduced_tx_set: false,
            coded_lossless: false,
            txfm_mode: TxfmMode::default(),
            use_ref_frame_mvs: false,
            primary_ref_frame: 7,
            disable_frame_end_update_cdf: true,
            order_hint: 0,
            ref_frame_idx: None,
            refresh_frame_flags: 0,
            segmentation: crate::frame_header_full::SegmentationInfo::default(),
            mono_chrome: true,
            subsampling_x: false,
            subsampling_y: false,
            enable_filter_intra: false,
            cdef_bits: 0,
            show_existing_slot: None,
            key_frame: false,
            tiles: None,
            tile_groups: Vec::new(),
            decoded: None,
            ref_frame_sign_bias: [false; 7],
            ref_order_distance: [0; 7],
            loop_restoration: Default::default(),
            superres: false,
            skip_mode_present: false,
            skip_mode_refs: [0, 0],
            subpel_filter_switchable: false,
            switchable_motion_mode: false,
            allow_warped_motion: false,
            force_integer_mv: false,
            allow_high_precision_mv: false,
            gm_type: [0u8; 8],
            enable_interintra_compound: false,
            enable_dual_filter: false,
            enable_masked_compound: false,
            enable_jnt_comp: false,
            enable_warped_motion: false,
        }
    }

    /// Takes the dimensions, chroma format and tool switches from the sequence header.
    pub(super) fn apply_sequence_header(&mut self, seq_hdr: &crate::SequenceHeader) {
        self.dimensions = FrameDimensions {
            width: seq_hdr.max_frame_width,
            height: seq_hdr.max_frame_height,
            sb_size: if seq_hdr.use_128x128_superblock {
                128
            } else {
                64
            },
            sb_cols: 0,
            sb_rows: 0,
        };
        self.mono_chrome = seq_hdr.color_config.mono_chrome;
        self.subsampling_x = seq_hdr.color_config.subsampling_x;
        self.subsampling_y = seq_hdr.color_config.subsampling_y;
        self.enable_filter_intra = seq_hdr.enable_filter_intra;
        self.enable_interintra_compound = seq_hdr.enable_interintra_compound;
        self.enable_dual_filter = seq_hdr.enable_dual_filter;
        self.enable_masked_compound = seq_hdr.enable_masked_compound;
        self.enable_jnt_comp = seq_hdr.enable_jnt_comp;
        self.enable_warped_motion = seq_hdr.enable_warped_motion;
    }

    /// Takes the three values the approximate [`parse_frame_header_basic`] supplies (intra flag,
    /// base QP, delta-q flag). They stand until [`ParsedFrame::apply_full_header`] replaces them,
    /// and are the only ones there are while no sequence header has been seen.
    pub(super) fn apply_basic_header(&mut self, frame_hdr: &crate::frame_header::FrameHeader) {
        // `FrameTypeInfo::is_intra_only`'s doc says "key/intra-only" -- i.e. spec
        // 5.9.2's `FrameIsIntra` (`frame_type == KEY_FRAME || frame_type ==
        // INTRA_ONLY_FRAME`), which is `FrameType::is_intra()`, NOT
        // `FrameType::is_intra_only()` (`bitvue-engine`'s codec-agnostic type --
        // that method only matches AV1's literal, rare INTRA_ONLY_FRAME type,
        // excluding ordinary KEY_FRAME). Using the narrower method meant every
        // real KEY_FRAME in every fixture this session ever parsed was silently
        // routed through `parse_coding_unit`'s INTER branch instead of its INTRA
        // one -- found while verifying the new key-frame `intra_mode`/`kfym`
        // context work (`SymbolDecoder::read_intra_mode`) had zero real frames to
        // exercise it on.
        self.frame_type.is_intra_only = frame_hdr.frame_type.is_intra();
        self.frame_type.base_qp = frame_hdr.base_q_idx;
        self.delta_q_enabled = frame_hdr.delta_q_present;
    }

    /// Takes every value of a fully parsed frame header; `prev_ref_order_hint` is the reference
    /// slots' order hints from before this frame's own header.
    pub(super) fn apply_full_header(
        &mut self,
        full_hdr: &crate::frame_header::FrameHeader,
        seq: &crate::SequenceHeader,
        prev_ref_order_hint: &[u32; 8],
    ) {
        // The exact header parse wins over `parse_frame_header_basic`'s
        // approximation for the two values it also supplied.
        self.delta_q_enabled = full_hdr.delta_q_present;
        if full_hdr.base_q_idx.is_some() {
            self.frame_type.base_qp = full_hdr.base_q_idx;
        }
        self.reference_select = full_hdr.reference_select;
        self.allow_intrabc = full_hdr.allow_intrabc;
        self.allow_screen_content_tools = full_hdr.allow_screen_content_tools;
        self.delta_lf_present = full_hdr.delta_lf_present;
        self.delta_lf_multi = full_hdr.delta_lf_multi;
        self.reduced_tx_set = full_hdr.reduced_tx_set;
        self.coded_lossless = full_hdr.base_q_idx == Some(0)
            && full_hdr.y_dc_delta_q.unwrap_or(0) == 0
            && full_hdr.uv_dc_delta_q.unwrap_or(0) == 0;
        self.txfm_mode = full_hdr.txfm_mode;
        self.use_ref_frame_mvs = full_hdr.use_ref_frame_mvs;
        self.primary_ref_frame = full_hdr.primary_ref_frame;
        self.disable_frame_end_update_cdf = full_hdr.disable_frame_end_update_cdf;
        self.order_hint = full_hdr.order_hint;
        self.ref_frame_idx = full_hdr.ref_frame_idx;
        self.refresh_frame_flags = full_hdr.refresh_frame_flags.unwrap_or(0);
        self.segmentation = full_hdr.segmentation;
        self.cdef_bits = full_hdr.cdef_damping.bits;
        self.show_existing_slot = full_hdr.frame_to_show_map_idx;
        self.tiles = full_hdr.tiles.clone();
        self.key_frame = !full_hdr.show_existing_frame
            && full_hdr.frame_type.is_intra()
            && !full_hdr.frame_type.is_intra_only();
        self.ref_frame_sign_bias = crate::frame_header_full::ref_frame_sign_bias(
            prev_ref_order_hint,
            full_hdr.ref_frame_idx.as_ref(),
            full_hdr.order_hint,
            seq,
        );
        self.ref_order_distance = crate::frame_header_full::ref_order_distance(
            prev_ref_order_hint,
            full_hdr.ref_frame_idx.as_ref(),
            full_hdr.order_hint,
            seq,
        );
        self.loop_restoration = full_hdr.loop_restoration.clone();
        self.superres = full_hdr.super_resolution.enabled;
        self.skip_mode_present = full_hdr.skip_mode_present;
        self.skip_mode_refs = full_hdr.skip_mode_refs;
        self.subpel_filter_switchable = full_hdr.subpel_filter_switchable;
        self.switchable_motion_mode = full_hdr.switchable_motion_mode;
        self.allow_warped_motion = full_hdr.allow_warped_motion;
        self.force_integer_mv = full_hdr.force_integer_mv;
        self.allow_high_precision_mv = full_hdr.allow_high_precision_mv;
        self.gm_type = full_hdr.gm_type;
    }

    /// Same as [`ParsedFrame::parse`], but threads a caller-supplied `RefFrameState` into the
    /// frame-header parse used to locate `tile_data`'s real start (`header_size_bytes`), instead
    /// of assuming a fresh/default state. `ref_state` should reflect every frame from 0 up to
    /// (but not including) `obu_data`'s own frame -- see [`RefFrameState`]'s doc and this
    /// method's callers (`bitvue-sidecar`'s `frame_analysis`/`deblocking`/`residual_analysis`/
    /// `codec_extended_info`/`coding_flow` modules) for the standard sequential-scan pattern that
    /// builds it. Mutates `ref_state` in place with this frame's own contribution as a side
    /// effect (matching [`crate::frame_header_full::parse_frame_header_full`]'s own contract),
    /// so callers that need the pre-this-frame state again afterward should clone it first.
    pub fn parse_with_ref_state(
        obu_data: &[u8],
        ref_state: &mut RefFrameState,
    ) -> Result<Self, BitvueError> {
        let obu_data: Arc<[u8]> = Arc::from(obu_data);
        // Order hints of the reference slots as they were before this frame's own header.
        let prev_ref_order_hint = *ref_state.ref_order_hint();

        // Handle empty data: return a default ParsedFrame immediately.
        // Callers that extract grids from empty data receive the default 1920x1080
        // scaffold, which is the documented behaviour for this module.
        if obu_data.is_empty() {
            return Ok(Self::scaffold(obu_data));
        }

        let obus_vec = collect_obus_resilient(&obu_data);

        // Build lightweight OBU references
        let mut frame = Self::scaffold(Arc::clone(&obu_data));
        frame.obus.reserve(obus_vec.len());
        let mut offset = 0;
        // The tile bytes of the frame parsed last; moved into `frame` once the loop is done.
        let mut tile_data = Vec::new();
        // Retained across the loop so the frame-header OBU (which comes after the sequence
        // header in every real stream) can use it -- see reference_select/allow_intrabc's doc.
        let mut seq_header: Option<crate::SequenceHeader> = None;

        for obu in &obus_vec {
            let payload_start = offset + obu.header.header_size;
            let payload_end = payload_start + obu.payload.len();

            frame.obus.push(ObuRef {
                obu_type: obu.header.obu_type,
                payload_start,
                payload_end: payload_end.min(obu_data.len()),
            });

            // Extract information based on OBU type
            match obu.header.obu_type {
                ObuType::SequenceHeader => {
                    if let Ok(seq_hdr) = crate::parse_sequence_header(&obu.payload) {
                        frame.apply_sequence_header(&seq_hdr);
                        seq_header = Some(seq_hdr);
                    }
                }
                ObuType::Frame => frame.absorb_frame_obu(
                    &obu.payload,
                    seq_header.as_ref(),
                    ref_state,
                    &prev_ref_order_hint,
                    &mut tile_data,
                ),
                ObuType::FrameHeader => frame.absorb_frame_header_obu(
                    &obu.payload,
                    seq_header.as_ref(),
                    ref_state,
                    &prev_ref_order_hint,
                    &mut tile_data,
                ),
                ObuType::TileGroup => frame.absorb_tile_group_obu(&obu.payload, &mut tile_data),
                _ => {}
            }

            offset = payload_end;
        }

        frame.finish_superblock_grid();

        frame.tile_data = Arc::from(tile_data);
        Ok(frame)
    }

    /// An `OBU_FRAME`: the header and the first tile group in one payload.
    fn absorb_frame_obu(
        &mut self,
        payload: &[u8],
        seq: Option<&crate::SequenceHeader>,
        ref_state: &mut RefFrameState,
        prev_ref_order_hint: &[u32; 8],
        tile_data: &mut Vec<u8>,
    ) {
        // OBU_FRAME packs frame_header() + byte_alignment() + tile_group() into one
        // payload (spec 5.10) -- unlike a standalone FrameHeader OBU, the tile bytes
        // live right here, after header_size_bytes. Most real encoders (including
        // this crate's own IVF test fixture) emit this combined form rather than
        // separate FrameHeader+TileGroup OBUs, so without this split, tile_data stays
        // empty and every real-CU-parsing consumer (QP/MV/partition/prediction-mode/
        // transform grids, deblocking boundary strength) silently falls back to
        // scaffold data -- found via `get_deblocking_analysis` returning a decode
        // error on real fixture frames that should have real tile data.
        //
        // `header_size_bytes` for the tile_data cut MUST come from `full_hdr`
        // (`parse_frame_header_full`), not `parse_frame_header_basic` -- confirmed via
        // a real dav1d oracle build (`DEBUG_BLOCK_INFO`) that `basic`'s value is
        // exactly what its own doc admits: an approximation for non-KEY frames that
        // skips `frame_size()`/`tile_info()`/`segmentation_params()`/
        // `loop_filter_params()`/`cdef_params()`/`lr_params()`/
        // `global_motion_params()`/etc. entirely. On the real fixture's frame 13 this
        // undershoots by 11 bytes (`basic`=8, `full`=19, oracle's independently-derived
        // true offset=19) -- every inter frame's `tile_data` has silently started 11+
        // bytes into what's still frame-header bits, handing the symbol decoder
        // garbage from its very first read. This was the actual root cause behind the
        // "compounding entropy desync" investigated over many sessions as a
        // coeff_base/coeff_br context-precision suspicion -- that suspicion was never
        // reached because the decoder was never even starting from real tile bytes for
        // inter frames. `basic` is kept only for `is_intra_only`/`base_qp`/
        // `delta_q_enabled` when `seq_header` isn't available yet (a Frame OBU can't
        // spec-legally precede a Sequence Header, so this fallback path is defensive,
        // not a real-stream case).
        // A temporal unit can hold several frames (a hidden ARF followed by the shown
        // frame): every field below describes the frame parsed last, so the tile
        // bytes of an earlier one must not stay in front of them.
        tile_data.clear();
        self.tile_groups.clear();
        if let Ok(frame_hdr) = parse_frame_header_basic(payload) {
            self.apply_basic_header(&frame_hdr);
            if seq.is_none() && frame_hdr.header_size_bytes < payload.len() {
                tile_data.extend_from_slice(&payload[frame_hdr.header_size_bytes..]);
            }
        }
        if let Some(seq) = seq {
            if let Ok(full_hdr) = parse_frame_header_full(payload, seq, ref_state) {
                self.apply_full_header(&full_hdr, seq, prev_ref_order_hint);
                if full_hdr.header_size_bytes < payload.len() {
                    let start = tile_data.len();
                    tile_data.extend_from_slice(&payload[full_hdr.header_size_bytes..]);
                    self.tile_groups.push(start..tile_data.len());
                }
            }
        }
    }

    /// A standalone `OBU_FRAME_HEADER`; its tile groups follow as `OBU_TILE_GROUP`s.
    fn absorb_frame_header_obu(
        &mut self,
        payload: &[u8],
        seq: Option<&crate::SequenceHeader>,
        ref_state: &mut RefFrameState,
        prev_ref_order_hint: &[u32; 8],
        tile_data: &mut Vec<u8>,
    ) {
        // Same reset as for `ObuType::Frame`: this header starts a new frame, so the
        // tile groups gathered for an earlier frame of the packet are not its own.
        tile_data.clear();
        self.tile_groups.clear();
        if let Ok(frame_hdr) = parse_frame_header_basic(payload) {
            self.apply_basic_header(&frame_hdr);
        }
        if let Some(seq) = seq {
            if let Ok(full_hdr) = parse_frame_header_full(payload, seq, ref_state) {
                self.apply_full_header(&full_hdr, seq, prev_ref_order_hint);
            }
        }
    }

    /// An `OBU_TILE_GROUP`: its payload is appended to `tile_data`, with a range of its own.
    fn absorb_tile_group_obu(&mut self, payload: &[u8], tile_data: &mut Vec<u8>) {
        let start = tile_data.len();
        tile_data.extend_from_slice(payload);
        self.tile_groups.push(start..tile_data.len());
    }

    /// The frame's size in superblocks, once the sequence header has given its size.
    fn finish_superblock_grid(&mut self) {
        if self.dimensions.width > 0 && self.dimensions.sb_size > 0 {
            self.dimensions.sb_cols = self.dimensions.width.div_ceil(self.dimensions.sb_size);
            self.dimensions.sb_rows = self.dimensions.height.div_ceil(self.dimensions.sb_size);
        }
    }
}

/// Every OBU of `obu_data` that parses; a failing one is logged, not propagated.
pub(super) fn collect_obus_resilient(obu_data: &[u8]) -> Vec<crate::Obu> {
    // Use resilient parsing: collect whatever OBUs parse successfully and
    // log (but do not propagate) any individual OBU parse errors.
    // This allows overlay extraction to work on truncated or minimal test
    // data without failing the whole operation.
    match parse_all_obus(obu_data) {
        Ok(obus) => obus,
        Err(e) => {
            tracing::warn!(
                "parse_all_obus failed ({}), falling back to resilient OBU iteration",
                e
            );
            // Collect successfully-parsed OBUs, silently dropping errors
            use crate::ObuIterator;
            ObuIterator::new(obu_data)
                .filter_map(|result| result.ok())
                .collect::<Vec<_>>()
        }
    }
}
