/**
 * Compare/diff domain — the single-stream debug-YUV reference-file workflow (VQ Analyzer "Load
 * Reference YUV") and the dual-stream A/B compare workspace (docs/DEVELOPMENT_PHASES.md Phase
 * 7.5). Two related but distinct features sharing one file: both are "diff against a second
 * source of pixels," just with a different second source (an on-disk reference file vs. stream
 * B). See `services/bridge/core.ts`'s module doc for how this file fits into the overall bridge
 * split.
 */

import { requireBridge } from "./core";
import type { BridgeDecodedYuvFrame } from "./frameDecode";
import type {
  ResolutionInfo,
  SyncMode,
  AlignmentQuality,
  AlignmentMethod,
  AlignmentConfidence,
} from "../../types/video";

export type { SyncMode };

export type DebugYuvFormat = "i420" | "nv12" | "nv21" | "i422" | "i444";
export type DebugYuvDisplayMode =
  | "decoded"
  | "reference"
  | "diff"
  | "amplified";

export interface DebugYuvCrop {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export interface LoadDebugYuvParams {
  path: string;
  width: number;
  height: number;
  format: DebugYuvFormat;
  bitdepth: number;
  picture_offset?: number;
  crop?: DebugYuvCrop;
}

export interface LoadDebugYuvResult {
  success: boolean;
  frame_count: number;
  frame_size: number;
  error: string | null;
}

export interface YuvDiffMetricsResult {
  frame_index: number;
  psnr_y: number;
  psnr_u: number;
  psnr_v: number;
  psnr_avg: number;
  ssim_y: number;
  max_diff_y: number;
  has_mismatch: boolean;
}

export interface FindFirstDiffFrameResult {
  frame_index: number | null;
  total_checked: number;
}

/** `create_compare_workspace`'s response -- a purpose-built summary, not a raw dump of the
 *  engine's internal `CompareWorkspace` struct (see `bitvue-sidecar/src/compare.rs`'s doc).
 *  `resolution_info` includes `ResolutionInfo`'s derived getters (`is_compatible`/
 *  `mismatch_percentage`/`is_exact_match`/`scale_indicator`) explicitly computed server-side,
 *  not just the raw struct fields. */
export interface CompareAlignmentSummary {
  method: AlignmentMethod;
  confidence: AlignmentConfidence;
  gap_count: number;
  gap_percentage: number;
  total_pairs: number;
}

export interface CompareWorkspaceSummary {
  total_frames: number;
  diff_enabled: boolean;
  disable_reason: string | null;
  sync_mode: SyncMode;
  manual_offset: number;
  resolution_info: ResolutionInfo;
  alignment: CompareAlignmentSummary;
}

/** `get_aligned_frame`'s response -- `null` fields when there's no aligned B frame. */
export interface AlignedFrameResult {
  stream_b_frame_idx: number | null;
  quality: AlignmentQuality | null;
}

export type DiffMode = "abs" | "signed";

/** `bitvue_engine::diff_heatmap::DiffHeatmapData`'s wire shape (`get_diff_frame`). Half-res
 *  (`heatmap_width/height` ≈ `frame_width/height` / 2), row-major `values`. */
export interface DiffHeatmapResult {
  frame_width: number;
  frame_height: number;
  heatmap_width: number;
  heatmap_height: number;
  values: number[];
  mode: DiffMode;
  min_value: number;
  max_value: number;
}

// -- Debug YUV (VQ Analyzer "Load Reference YUV" workflow) --------------------------------------
//
// One global reference-file session, not per-StreamId -- see bitvue-sidecar's debug_yuv module
// doc. `loadDebugYuv`'s domain failure (bad path, file too small for the declared resolution) is
// NOT a thrown error -- same "RPC succeeded, check result.success" split as openStream -- but the
// rest all throw on failure (mirroring getFrameSyntax/getTimeline's "no meaningful partial result"
// reasoning): a NotFound wire error when no session is loaded yet.

export async function loadDebugYuv(
  params: LoadDebugYuvParams,
): Promise<LoadDebugYuvResult> {
  return requireBridge().loadDebugYuv(params);
}

export async function unloadDebugYuv(): Promise<void> {
  return requireBridge().unloadDebugYuv();
}

export async function setDebugYuvOffset(offset: number): Promise<void> {
  return requireBridge().setDebugYuvOffset(offset);
}

export async function setDebugYuvCrop(crop: DebugYuvCrop): Promise<void> {
  return requireBridge().setDebugYuvCrop(crop);
}

export async function getYuvDiffMetrics(
  frameIndex: number,
): Promise<YuvDiffMetricsResult> {
  return requireBridge().getYuvDiffMetrics(frameIndex);
}

export async function findFirstDiffFrame(): Promise<FindFirstDiffFrameResult> {
  return requireBridge().findFirstDiffFrame();
}

/** Decoded/reference/diff/amplified pixel data for one display-index frame -- see
 *  `debug_yuv::get_frame`'s doc for what each mode means. Requires `loadDebugYuv` to have
 *  succeeded first. */
export async function getDebugYuvFrame(
  frameIndex: number,
  mode: DebugYuvDisplayMode,
  amplify?: number,
): Promise<BridgeDecodedYuvFrame> {
  return requireBridge().getDebugYuvFrame(frameIndex, mode, amplify);
}

// -- Dual-stream A/B compare workspace -----------------------------------------------------------

/** Dual-stream compare workspace (docs/DEVELOPMENT_PHASES.md Phase 7.5) -- builds a real
 *  PTS-based alignment between whichever streams are currently open as A/B. Requires both to be
 *  open AND indexed first. */
export async function createCompareWorkspace(): Promise<CompareWorkspaceSummary> {
  return requireBridge().createCompareWorkspace();
}

export async function getAlignedFrame(
  streamAFrameIdx: number,
): Promise<AlignedFrameResult> {
  return requireBridge().getAlignedFrame(streamAFrameIdx);
}

export async function setSyncMode(
  mode: SyncMode,
): Promise<{ sync_mode: SyncMode }> {
  return requireBridge().setSyncMode(mode);
}

export async function setManualOffset(
  offset: number,
): Promise<{ manual_offset: number }> {
  return requireBridge().setManualOffset(offset);
}

export async function resetOffset(): Promise<{ manual_offset: number }> {
  return requireBridge().resetOffset();
}

export async function getDiffFrame(
  streamAFrameIdx: number,
  mode: DiffMode,
): Promise<DiffHeatmapResult> {
  return requireBridge().getDiffFrame(streamAFrameIdx, mode);
}

/** PARITY_CHECKLIST.md CMP-04 -- scans stream A frame by frame for the first real pixel
 *  difference against its aligned stream B frame. Reuses `FindFirstDiffFrameResult`'s shape
 *  (`{ frame_index, total_checked }`) from the debug-YUV single-stream sibling command -- same
 *  contract, different comparison target (stream B instead of an on-disk reference file). */
export async function findFirstDiffFrameAb(): Promise<FindFirstDiffFrameResult> {
  return requireBridge().findFirstDiffFrameAb();
}
