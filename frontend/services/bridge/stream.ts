/**
 * Stream domain — open/close/select a stream, metadata indexing, filmstrip thumbnails, the
 * display-order timeline. See `services/bridge/core.ts`'s module doc for how this file fits into
 * the overall bridge split.
 */

import { requireBridge, type StreamId, type BridgeEvent } from "./core";

export interface OpenStreamResult {
  /** False if the only event back was a `DiagnosticAdded` (severity Error) — see
   *  `Core::handle_command`'s design note: failures aren't a wire-level error, they're an event,
   *  same as the UI would eventually see them. */
  success: boolean;
  path: string;
  events: BridgeEvent[];
  /** Present only when `success` is false. */
  error?: string;
}

/** Mirrors `bitvue_engine::UnitNode`'s JSON shape (it's one of the few `bitvue-engine` types that
 *  derives `Serialize` — see `bitvue-sidecar`'s module doc). One entry per parsed frame/unit. */
export interface BridgeUnitNode {
  key: unknown;
  unit_type: string;
  offset: number;
  size: number;
  frame_index: number | null;
  frame_type: string | null;
  pts: number | null;
  dts: number | null;
  display_name: string;
  children: BridgeUnitNode[];
  qp_avg: number | null;
  mv_grid: unknown;
  temporal_id: number | null;
  ref_frames: number[] | null;
  ref_slots: number[] | null;
}

/** Mirrors `get_stream_info`'s hand-mapped JSON (`bitvue-sidecar::container_model_to_json`). */
export interface BridgeContainerModel {
  format: string;
  codec: string;
  track_count: number;
  duration_ms: number | null;
  bitrate_bps: number | null;
  width: number | null;
  height: number | null;
  bit_depth: number | null;
}

export interface StreamInfoResult {
  indexed: boolean;
  container: BridgeContainerModel | null;
}

export interface FramesChunkResult {
  indexed: boolean;
  units: BridgeUnitNode[];
  total_count: number;
}

/** Mirrors `bitvue_engine::timeline::TimelineFrame` -- it (and `BridgeTimeline` below) derive
 *  `Serialize` directly, unlike most `bitvue-engine` types, so this is a straight field mirror,
 *  no hand-mapped JSON on the Rust side to keep in sync. */
export interface BridgeTimelineFrame {
  display_idx: number;
  size_bytes: number;
  frame_type: string;
  marker: "None" | "Key" | "Error" | "Bookmark";
  pts: number | null;
  dts: number | null;
  is_selected: boolean;
}

/** Mirrors `bitvue_engine::timeline::TimelineBase`. `pts_quality` is a stream-wide verdict
 *  (not per-frame) -- see `frame_identity::PtsQuality`'s doc for exactly what triggers
 *  Warn/Bad (duplicate PTS or >50% missing -> Bad; some missing or VFR -> Warn; decode-order
 *  non-monotonicity from B-frames is expected and NOT a quality issue). */
export interface BridgeTimeline {
  stream_id: string;
  frames: BridgeTimelineFrame[];
  current_frame: number | null;
  scrub_mode: "Idle" | "Active";
  viewport: [number, number];
  vertical_viewport: [number, number];
  pts_quality: "Ok" | "Warn" | "Bad";
}

/** Mirrors `get_thumbnails`'s JSON array response (`bitvue-sidecar`'s `decode_bridge` module).
 *  `thumbnail_data` is a real `data:image/png;base64,...` URL -- feeds directly into an
 *  `<img src>`, no frontend-side decoding needed. */
export interface BridgeThumbnailResult {
  frame_index: number;
  thumbnail_data: string;
  width: number;
  height: number;
  success: boolean;
}

export async function openStream(
  stream: StreamId,
  path: string,
): Promise<OpenStreamResult> {
  const { events } = await requireBridge().openStream(stream, path);
  const diagnostic = events.find((e) => e.type === "DiagnosticAdded");
  return {
    success: !diagnostic,
    path,
    events,
    error: diagnostic ? String(diagnostic.diagnostic) : undefined,
  };
}

export async function closeStream(stream: StreamId): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().closeStream(stream);
  return events;
}

export async function selectFrame(
  stream: StreamId,
  frameIndex: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectFrame(stream, frameIndex);
  return events;
}

/** Runs `bitvue-indexer`'s metadata indexing (container + units) for the given stream. IVF/AV1
 *  only so far — other formats come back as a `DiagnosticAdded` event, not an exception. */
export async function indexStream(stream: StreamId): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().indexStream(stream);
  return events;
}

/** Structured shape of a `DiagnosticAdded` event's `diagnostic` field -- mirrors
 *  `bitvue_engine::event::Diagnostic`'s hand-mapped JSON (`command_support.rs`'s
 *  `event_to_json`). `severity`/`category` are the Rust enum variant names verbatim (`Debug`
 *  formatted), not lowercased -- callers map them to whatever local vocabulary they need
 *  (see `DiagnosticsPanel.tsx`'s severity mapping). */
export interface BridgeDiagnostic {
  id: number;
  severity: "Info" | "Warn" | "Error" | "Fatal";
  message: string;
  category: "Container" | "Bitstream" | "Decode" | "Metric" | "IO" | "Worker";
  offset_bytes: number;
  timestamp_ms: number;
  frame_index: number | null;
  count: number;
  impact_score: number;
}

/** Pulls the real, structured diagnostics out of a `BridgeEvent[]` (e.g. `indexStream`'s
 *  return) -- skips every other event type. `BridgeEvent`'s `diagnostic` field is typed
 *  `unknown` at that layer since `BridgeEvent` is generic across all event types; this is the
 *  one place that narrows it back to `BridgeDiagnostic` for real `DiagnosticAdded` events. */
export function extractDiagnostics(events: BridgeEvent[]): BridgeDiagnostic[] {
  return events
    .filter((e) => e.type === "DiagnosticAdded")
    .map((e) => e.diagnostic as BridgeDiagnostic)
    .filter((d): d is BridgeDiagnostic => d != null && typeof d === "object");
}

/** Container metadata populated by `indexStream`. `indexed: false` (not a thrown error) when
 *  nothing's been indexed yet -- that's a normal, expected state. */
export async function getStreamInfo(
  stream: StreamId,
): Promise<StreamInfoResult> {
  return requireBridge().getStreamInfo(stream);
}

/** Paginated slice of the unit/frame list populated by `indexStream`. */
export async function getFramesChunk(
  stream: StreamId,
  offset: number,
  limit: number,
): Promise<FramesChunkResult> {
  return requireBridge().getFramesChunk(stream, offset, limit);
}

/** Display-order timeline (frame types/sizes/markers/pts_quality), built from already-indexed
 *  units via `bitvue_engine::frame_identity::TimelineMapper` (AV1 only so far -- see
 *  `bitvue-indexer`'s `get_timeline` doc). As of 2026-08-23, `FileStateContext.tsx`'s
 *  `applyDisplayOrder` calls this once per `refreshFrames` to join real `display_order`/
 *  `coding_order` (CTX-02) and `pts_quality` (EDGE-03) into the frames it already has. Throws on
 *  failure (no units indexed yet, wrong codec) -- same reasoning as `getFrameSyntax`. */
export async function getTimeline(stream: StreamId): Promise<BridgeTimeline> {
  return requireBridge().getTimeline(stream);
}

/** Batch thumbnail generation -- decodes once per call, capturing every requested index along
 *  the way (see `bitvue-sidecar`'s `decode_bridge::get_thumbnails` doc), not once per index.
 *  `targetWidth` defaults to 120px server-side (matches `THUMBNAIL_SIZE.WIDTH`) if omitted. */
export async function getThumbnails(
  stream: StreamId,
  frameIndices: number[],
  targetWidth?: number,
): Promise<BridgeThumbnailResult[]> {
  return requireBridge().getThumbnails(stream, frameIndices, targetWidth);
}
