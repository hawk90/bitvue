/**
 * Frame analysis domain — per-frame QP/MV/partition grids and codec-feature breakdowns that feed
 * the overlay renderers and analysis tabs (CodingFlowView/DeblockingView/RefListTab/
 * StatisticsTab/ResidualsView). AV1/IVF only, all throw on failure (frame out of range, stream
 * not open) -- no meaningful partial result for any of these. See `services/bridge/core.ts`'s
 * module doc for how this file fits into the overall bridge split.
 */

import { requireBridge } from "./core";
import type { FrameAnalysisData, QPGrid } from "../../types/video";

/** Wire shape of `get_frame_analysis`'s response: identical to `FrameAnalysisData` except
 *  `qp_grid.qp` (a flat per-block `Vec<i16>`, up to 129,600 values at 4K/8px-blocks) moves to a
 *  raw `qp_bytes` buffer instead of a JSON number array -- see `bitvue-sidecar`'s
 *  `frame_analysis.rs::get_frame_analysis_command` doc for why (JSON-encoding that many numbers
 *  costs real CPU + wire bytes for no benefit, the same "big flat array" reasoning
 *  `getDecodedFrameYuv`/`getHexRange` already apply to pixel/byte data). `qp_bytes` is
 *  little-endian `i16` per value, row-major -- `getFrameAnalysis` below reconstructs `qp_grid.qp`
 *  from it so every caller of the public function still sees a plain `FrameAnalysisData`. */
export type FrameAnalysisWireResult = Omit<FrameAnalysisData, "qp_grid"> & {
  qp_grid?: Omit<QPGrid, "qp">;
  qp_bytes: Uint8Array;
};

// -- get_av1_features wire shapes ----------------------------------------------------------------
//
// Snake_case, matching bitvue-sidecar's av1_features module directly (not the camelCase
// Av1FeaturesData in components/panels/OverlayRenderer/types.ts -- that's a separate,
// pre-existing consumer shape; useAv1Features.ts translates between the two at its boundary).

export interface Av1CdefBlockWire {
  x: number;
  y: number;
  size: number;
  direction: number;
  strength: number;
}

export interface Av1CdefDataWire {
  width: number;
  height: number;
  block_size: number;
  blocks: Av1CdefBlockWire[];
  damping: number;
  y_primary_strength: number;
  y_secondary_strength: number;
}

export interface Av1RestorationUnitWire {
  x: number;
  y: number;
  size: number;
  restoration_type: number;
}

export interface Av1LoopRestorationDataWire {
  width: number;
  height: number;
  unit_size: number;
  y_type: number;
  units: Av1RestorationUnitWire[];
}

export interface Av1FilmGrainDataWire {
  enabled: boolean;
  seed: number;
  scaling_shift: number;
  ar_coeff_lag: number;
  chroma_scaling_from_luma: boolean;
  overlap: boolean;
}

export interface Av1SuperResDataWire {
  enabled: boolean;
  scale_denominator: number;
  upscaled_width: number;
  upscaled_height: number;
}

export interface Av1FeaturesWireResult {
  frame_index: number;
  cdef: Av1CdefDataWire | null;
  loop_restoration: Av1LoopRestorationDataWire | null;
  film_grain: Av1FilmGrainDataWire | null;
  super_resolution: Av1SuperResDataWire | null;
}

export interface CodingFlowStageWire {
  id: string;
  label: string;
  completed: boolean;
  data_size: number | null;
}

export interface CodingFlowAnalysisWireResult {
  frame_index: number;
  stages: CodingFlowStageWire[];
  current_stage: string;
  codec_features: string[];
}

export interface DeblockingEdgeWire {
  x: number;
  y: number;
  length: number;
  orientation: "vertical" | "horizontal";
  /** The largest filter the edge allows, in samples (AV1 spec `filterSize`). */
  filter_size: 4 | 8 | 16;
  /** `applyFilter` and a non-zero `level`. */
  filtered: boolean;
  /** Filter level 0-63 after segment, reference and mode adjustments. */
  level: number;
}

/** Real AV1 loop_filter_params() fields (spec 5.9.11). `level`/`ref_deltas`/`mode_deltas` mirror
 *  bitvue_av1_codec::frame_header::LoopFilterInfo directly. */
export interface DeblockingParamsWire {
  level: [number, number, number, number];
  sharpness: number;
  delta_enabled: boolean;
  ref_deltas: number[];
  mode_deltas: number[];
}

/** Counts are in 4-sample segments (edge runs are merged along the edge). */
export interface DeblockingStatsWire {
  edge_segments: number;
  filtered_segments: number;
  unfiltered_segments: number;
  filtered_16: number;
  filtered_8: number;
  filtered_4: number;
}

export interface DeblockingAnalysisWireResult {
  frame_index: number;
  width: number;
  height: number;
  edges: DeblockingEdgeWire[];
  params: DeblockingParamsWire;
  stats: DeblockingStatsWire;
}

/** AV1 has no long-term marking or weighted-prediction syntax -- `long_term` is always `false`
 *  and `weight`/`offset` are always `null` on the wire (see bitvue-sidecar::codec_extended_info's
 *  module doc). */
export interface RefEntryWire {
  list_idx: number;
  slot: number;
  poc: number;
  frame_index: number;
  frame_type: string;
  long_term: boolean;
  weight: number | null;
  offset: number | null;
}

export interface QpHistogramBucketWire {
  qp: number;
  count: number;
}

export interface CodecExtendedInfoWireResult {
  frame_index: number;
  l0_refs: RefEntryWire[];
  l1_refs: RefEntryWire[];
  qp_histogram: QpHistogramBucketWire[];
}

/** `energy` is `sum_abs_level` (sum of absolute coefficient levels), not a spatial-domain energy
 *  metric -- bitvue-sidecar has no inverse-transform stage to produce one. See
 *  bitvue-sidecar::residual_analysis's module doc. */
export interface CoefficientStatsWire {
  min: number;
  max: number;
  mean: number;
  variance: number;
  energy: number;
  zero_count: number;
  non_zero_count: number;
}

export interface BlockResidualWire {
  x: number;
  y: number;
  width: number;
  height: number;
  energy: number;
  max_coeff: number;
  non_zeros: number;
}

export interface ResidualAnalysisWireResult {
  frame_index: number;
  width: number;
  height: number;
  coefficient_stats: CoefficientStatsWire;
  block_residuals: BlockResidualWire[];
}

/** Decodes `FrameAnalysisWireResult.qp_bytes` (little-endian `i16` per value) back into the
 *  plain `number[]` `QPGrid.qp` expects. Uses `DataView.getInt16` rather than an `Int16Array`
 *  view over the same buffer -- structured-clone IPC doesn't guarantee `qp_bytes.byteOffset` is
 *  even, and an unaligned `Int16Array` view would throw. */
function decodeQpBytes(bytes: Uint8Array): number[] {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const qp: number[] = new Array(bytes.byteLength / 2);
  for (let i = 0; i < qp.length; i++) {
    qp[i] = view.getInt16(i * 2, true);
  }
  return qp;
}

/** QP/MV/partition/prediction-mode/transform-size grids for one frame of stream A -- feeds the
 *  main viewer's overlay renderers. AV1/IVF only. Throws on failure (frame out of range, stream
 *  not open) -- no meaningful partial result, same reasoning as getFrameSyntax/getTimeline. */
export async function getFrameAnalysis(
  frameIndex: number,
): Promise<FrameAnalysisData> {
  const wire = await requireBridge().getFrameAnalysis(frameIndex);
  const { qp_bytes, qp_grid, ...rest } = wire;
  return {
    ...rest,
    qp_grid: qp_grid && { ...qp_grid, qp: decodeQpBytes(qp_bytes) },
  };
}

/** What `get_hrd_info` returns (`bitvue-sidecar`'s `hrd_info.rs`): the AV1 decoder model of stream A
 *  and the buffer limits its operating points' levels imply. AV1 signals no bit rate or buffer
 *  size -- both follow from level, tier and profile (spec Annex E.2/A.3), so `max_bitrate_bps` and
 *  `buffer_size_bits` are `null` for a level the spec defines no limits for. */
export interface HrdOperatingPointWire {
  idc: number;
  seq_level_idx: number;
  level: string | null;
  tier: "main" | "high";
  max_bitrate_bps: number | null;
  buffer_size_bits: number | null;
  /** Present exactly when the operating point has a decoder model. */
  decoder_model: {
    decoder_buffer_delay: number;
    encoder_buffer_delay: number;
    low_delay_mode: boolean;
  } | null;
}

/** One decodable frame group (a decoded frame) of the decoder model run (spec Annex E.4); times in
 *  seconds. */
export interface HrdModelFrameWire {
  dfg: number;
  /** The IVF packet the frame is in -- the index of the frame list. */
  packet: number;
  frame_type: string;
  show_frame: boolean;
  coded_bits: number;
  buffer_removal_time: number | null;
  frame_presentation_time: number | null;
  first_bit_arrival: number;
  last_bit_arrival: number;
  scheduled_removal: number;
  removal: number;
}

export type HrdViolationKind =
  | "smoothing_buffer_underflow"
  | "smoothing_buffer_overflow"
  | "decoder_buffer_delay_consistency"
  | "decoder_buffer_delay_level"
  | "decode_buffer_available_late"
  | "decode_frame_buf_unavailable"
  | "decode_existing_frame_buf_empty"
  | "display_frame_late";

export interface HrdViolationWire {
  kind: HrdViolationKind;
  /** The decodable frame group it is found at; `null` for a stream-wide requirement or a
   *  `show_existing_frame`. */
  dfg: number | null;
  packet: number | null;
}

/** The decoder model (spec Annex E) run over the whole stream for the first operating point. */
export interface HrdModelWire {
  mode: "decoding_schedule" | "resource_availability";
  bit_rate_bps: number;
  buffer_size_bits: number;
  decoder_buffer_delay: number;
  encoder_buffer_delay: number;
  low_delay_mode: boolean;
  /** `DecCT` in seconds; decoding schedule mode only. */
  dec_ct_seconds: number | null;
  peak_bits: number;
  frames: HrdModelFrameWire[];
  /** `[seconds, bits]` breakpoints of the smoothing buffer's piecewise linear occupancy; a
   *  removal is two points at one time (the level before and after it). */
  curve: [number, number][];
  violations: HrdViolationWire[];
}

export interface HrdInfoWireResult {
  /** The decoder model ran: the stream has timing the HRD view can show. */
  present: boolean;
  /** The decoder model, `null` when the stream signals none (see `model_unavailable`). */
  model: HrdModelWire | null;
  model_unavailable: string | null;
  profile: string;
  /** From `timing_info` with an equal picture interval; `null` for a variable display rate. */
  frame_rate: number | null;
  timing: {
    num_units_in_display_tick: number;
    time_scale: number;
    equal_picture_interval: boolean;
    num_ticks_per_picture: number | null;
  } | null;
  decoder_model: {
    buffer_delay_length: number;
    num_units_in_decoding_tick: number;
    buffer_removal_time_length: number;
    frame_presentation_time_length: number;
  } | null;
  operating_points: HrdOperatingPointWire[];
}

/** The decoder model of stream A. AV1/IVF only. Throws when no stream is open or its sequence
 *  header cannot be read. */
export async function getHrdInfo(): Promise<HrdInfoWireResult> {
  return requireBridge().getHrdInfo();
}

/** CDEF/loop-restoration/film-grain/super-resolution data for one frame of stream A. AV1/IVF
 *  only. Throws on failure (frame out of range, stream not open, or a stream using short
 *  reference-frame signaling -- see bitvue_av1_codec::frame_header_full's module doc). */
export async function getAv1Features(
  frameIndex: number,
): Promise<Av1FeaturesWireResult> {
  return requireBridge().getAv1Features(frameIndex);
}

/** Encoder/decoder pipeline stage completion + real sequence-header codec features for one frame
 *  of stream A -- feeds CodingFlowView. AV1/IVF only. Throws on failure (frame out of range,
 *  stream not open). */
export async function getCodingFlowAnalysis(
  frameIndex: number,
): Promise<CodingFlowAnalysisWireResult> {
  return requireBridge().getCodingFlowAnalysis(frameIndex);
}

/** AV1 loop-filter boundary strength per coding-unit edge + real loop-filter parameters for one
 *  frame of stream A -- feeds DeblockingView. AV1/IVF only. Throws on failure (frame out of
 *  range, stream not open). */
export async function getDeblockingAnalysis(
  frameIndex: number,
): Promise<DeblockingAnalysisWireResult> {
  return requireBridge().getDeblockingAnalysis(frameIndex);
}

/** AV1 reference-frame lists (L0/L1) + QP histogram for one frame of stream A -- feeds
 *  RefListTab/StatisticsTab. AV1/IVF only. Throws on failure (frame out of range, stream not
 *  open). */
export async function getCodecExtendedInfo(
  frameIndex: number,
): Promise<CodecExtendedInfoWireResult> {
  return requireBridge().getCodecExtendedInfo(frameIndex);
}

/** Per-block residual coefficient magnitude statistics for one frame of stream A -- feeds
 *  ResidualsView. AV1/IVF only. Throws on failure (frame out of range, stream not open). */
export async function getResidualAnalysis(
  frameIndex: number,
): Promise<ResidualAnalysisWireResult> {
  return requireBridge().getResidualAnalysis(frameIndex);
}
