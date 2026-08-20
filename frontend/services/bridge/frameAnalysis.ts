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
  boundary_strength: number;
  filtered: boolean;
  strength: number;
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

export interface DeblockingStatsWire {
  total_edges: number;
  filtered_edges: number;
  strong_edges: number;
  weak_edges: number;
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
