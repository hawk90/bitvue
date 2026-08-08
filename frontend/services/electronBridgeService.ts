/**
 * Electron sidecar bridge — the replacement for the old `@tauri-apps/api` `invoke()`-based
 * command layer (`tauriCommandService.ts`, and the various files that called `invoke()`
 * directly). Thin wrappers around `window.bitvue.*`, exposed by
 * `bitvue-desktop/electron/preload.cjs` via `contextBridge`.
 *
 * IMPORTANT — the old Tauri command surface (~40 commands, `src-tauri/src/commands/*.rs`,
 * deleted 2026-08-08 -- though several *frontend* files still call `@tauri-apps/api` directly
 * and haven't been migrated to this bridge yet, e.g. the compare workspace / quality panels /
 * system menu / filmstrip thumbnails; grep the frontend tree before assuming a panel is covered)
 * and the new `bitvue-sidecar` surface (15 commands, see `docs/DEVELOPMENT_PHASES.md` §
 * "제품 아키텍처 확정") are NOT the same API — different names, different param/result shapes.
 * This file wraps all 15 real sidecar commands: open/close a stream, select a frame
 * (selection-sync only), the four structural multi-sync selections
 * (`selectUnit`/`selectSyntax`/`selectBitRange`/`selectSpatialBlock` — no live UI consumer as of
 * 2026-08-08, wired because the sidecar-side capability is real, not because a feature needs them
 * yet), a raw hex byte range, decoded YUV pixel planes for one frame (`getDecodedFrameYuv` —
 * AV1/IVF only, re-decodes from the stream start every call, no session caching yet), the native
 * open-file dialog, and metadata indexing (`indexStream`/`getStreamInfo`/`getFramesChunk` —
 * container + per-frame metadata, IVF/AV1 only so far) plus lazy per-unit syntax trees
 * (`getFrameSyntax`) and a display-order timeline (`getTimeline`, also no live UI consumer yet —
 * see its own doc), both AV1 only; see `bitvue-indexer`'s module doc. Don't add wrappers here for
 * capabilities the sidecar doesn't have; that would silently promise something broken.
 */

export type StreamId = "A" | "B";

/** Shape of the JSON-mapped `bitvue_engine::Event` values the sidecar sends back — see
 *  `bitvue-sidecar/src/main.rs`'s `event_to_json` for the authoritative field set per type. */
export interface BridgeEvent {
  type: string;
  stream?: string;
  [key: string]: unknown;
}

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

/** Mirrors `bitvue-sidecar`'s `syntax_node_to_json` -- a nested tree built from
 *  `bitvue_engine::SyntaxModel`'s flat node map. `value` is a plain string (or null for
 *  container/non-leaf fields), not a discriminated union -- unlike `bitvue_engine::UnitNode`,
 *  `SyntaxNode`'s `value` field is already just a display string in the Rust type. */
export interface BridgeSyntaxNode {
  type: string;
  name: string;
  value: string | null;
  bit_range: { start_bit: number; end_bit: number };
  children: BridgeSyntaxNode[];
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

/** Mirrors `bitvue_engine::timeline::TimelineBase`. */
export interface BridgeTimeline {
  stream_id: string;
  frames: BridgeTimelineFrame[];
  current_frame: number | null;
  scrub_mode: "Idle" | "Active";
  viewport: [number, number];
  vertical_viewport: [number, number];
}

/** Mirrors `get_decoded_frame_yuv`'s Control-frame metadata (`bitvue-sidecar`'s `decode_bridge`
 *  module) -- the `Data` frame that follows is the concatenated Y+U+V raw bytes, sliced using
 *  `y_len`/`u_len`/`v_len` below. Same no-base64, no-JSON-array approach as `getHexRange`. */
export interface BridgeDecodedYuvFrame {
  width: number;
  height: number;
  bitDepth: number;
  chromaSubsampling: "420" | "422" | "444";
  yStride: number;
  uStride: number;
  vStride: number;
  yLen: number;
  uLen: number;
  vLen: number;
  bytes: Uint8Array;
}

declare global {
  interface Window {
    bitvue?: {
      hello: (
        clientVersion?: string,
      ) => Promise<{ protocol_version: string; capabilities: string[] }>;
      openStream: (
        stream: StreamId,
        filePath: string,
      ) => Promise<{ events: BridgeEvent[] }>;
      closeStream: (stream: StreamId) => Promise<{ events: BridgeEvent[] }>;
      selectFrame: (
        stream: StreamId,
        frameIndex: number,
      ) => Promise<{ events: BridgeEvent[] }>;
      getHexRange: (
        stream: StreamId,
        offset: number,
        len: number,
      ) => Promise<{ offset: number; len: number; bytes: Uint8Array }>;
      getDecodedFrameYuv: (
        stream: StreamId,
        frameIndex: number,
      ) => Promise<BridgeDecodedYuvFrame>;
      showOpenDialog: (
        filters?: Array<{ name: string; extensions: string[] }>,
      ) => Promise<string | null>;
      indexStream: (stream: StreamId) => Promise<{ events: BridgeEvent[] }>;
      getStreamInfo: (stream: StreamId) => Promise<StreamInfoResult>;
      getFramesChunk: (
        stream: StreamId,
        offset: number,
        limit: number,
      ) => Promise<FramesChunkResult>;
      getFrameSyntax: (
        stream: StreamId,
        frameIndex: number,
      ) => Promise<BridgeSyntaxNode>;
      getTimeline: (stream: StreamId) => Promise<BridgeTimeline>;
      selectUnit: (
        stream: StreamId,
        unitType: string,
        offset: number,
        size: number,
      ) => Promise<{ events: BridgeEvent[] }>;
      selectSyntax: (
        stream: StreamId,
        nodeId: string,
        startBit: number,
        endBit: number,
      ) => Promise<{ events: BridgeEvent[] }>;
      selectBitRange: (
        stream: StreamId,
        startBit: number,
        endBit: number,
      ) => Promise<{ events: BridgeEvent[] }>;
      selectSpatialBlock: (
        stream: StreamId,
        x: number,
        y: number,
        w: number,
        h: number,
      ) => Promise<{ events: BridgeEvent[] }>;
      onSidecarRestarted: (callback: () => void) => () => void;
    };
  }
}

function requireBridge(): NonNullable<Window["bitvue"]> {
  if (!window.bitvue) {
    throw new Error(
      "window.bitvue is unavailable — this page isn't running inside the bitvue-desktop Electron " +
        "shell (preload didn't run), or it's a plain browser tab.",
    );
  }
  return window.bitvue;
}

/** Whether the Electron bridge is available at all — for callers that want to branch/skip. */
export function hasElectronBridge(): boolean {
  return typeof window !== "undefined" && Boolean(window.bitvue);
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

export async function getHexRange(
  stream: StreamId,
  offset: number,
  len: number,
): Promise<{ offset: number; len: number; bytes: Uint8Array }> {
  return requireBridge().getHexRange(stream, offset, len);
}

/** Decodes (dav1d, via `bitvue-decode`) the stream from its start up to and including
 *  `frameIndex`, returning that frame's real YUV planes. AV1/IVF only, no session caching yet --
 *  see `bitvue-sidecar`'s `decode_bridge` module doc for the current O(frameIndex) perf caveat. */
export async function getDecodedFrameYuv(
  stream: StreamId,
  frameIndex: number,
): Promise<BridgeDecodedYuvFrame> {
  return requireBridge().getDecodedFrameYuv(stream, frameIndex);
}

export interface OpenFileDialogFilter {
  name: string;
  extensions: string[];
}

/** Native "open file" dialog via the main process. Returns the selected path, or null if cancelled. */
export async function showOpenDialog(
  filters?: OpenFileDialogFilter[],
): Promise<string | null> {
  return requireBridge().showOpenDialog(filters);
}

/** Runs `bitvue-indexer`'s metadata indexing (container + units) for the given stream. IVF/AV1
 *  only so far — other formats come back as a `DiagnosticAdded` event, not an exception. */
export async function indexStream(stream: StreamId): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().indexStream(stream);
  return events;
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

/** Lazy, per-unit syntax tree (AV1 only so far -- see `bitvue-indexer`'s module doc). Unlike
 *  `getStreamInfo`/`getFramesChunk`, this throws on failure (no unit for that frame_index, index
 *  hasn't run yet, wrong codec) rather than returning an `{indexed: false}`-style payload -- the
 *  sidecar reports it as a real wire error since there's no meaningful partial syntax tree. */
export async function getFrameSyntax(
  stream: StreamId,
  frameIndex: number,
): Promise<BridgeSyntaxNode> {
  return requireBridge().getFrameSyntax(stream, frameIndex);
}

/** Display-order timeline (frame types/sizes/markers), built from already-indexed units via
 *  `bitvue_engine::frame_identity::TimelineMapper` (AV1 only so far -- see `bitvue-indexer`'s
 *  `get_timeline` doc). No component consumes this yet as of 2026-08-08 (grepped for an existing
 *  dead call site the way `getFramesChunk`/`getFrameSyntax` had -- none found); wired because the
 *  backend capability is real and tested, not because a specific UI feature needs it. Throws on
 *  failure (no units indexed yet, wrong codec) -- same reasoning as `getFrameSyntax`. */
export async function getTimeline(stream: StreamId): Promise<BridgeTimeline> {
  return requireBridge().getTimeline(stream);
}

// -- Structural (multi-sync) selection ---------------------------------------------------------
//
// These four map onto bitvue_engine::Command's "Tri-sync" selection variants -- independent of
// selectFrame's temporal cursor (a unit/syntax-node/bit-range/spatial-block selection can exist
// without a frame selection, and vice versa). No frontend component calls these yet as of
// 2026-08-08 -- they're wired here because the sidecar-side capability is real and tested
// (bitvue-sidecar's select_unit/select_syntax/select_bit_range/select_spatial_block, wired
// earlier this migration), not because a specific UI interaction needs them right now. Don't
// treat their existence as proof any cross-view sync feature is wired up end to end.

export async function selectUnit(
  stream: StreamId,
  unitType: string,
  offset: number,
  size: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectUnit(
    stream,
    unitType,
    offset,
    size,
  );
  return events;
}

export async function selectSyntax(
  stream: StreamId,
  nodeId: string,
  startBit: number,
  endBit: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectSyntax(
    stream,
    nodeId,
    startBit,
    endBit,
  );
  return events;
}

export async function selectBitRange(
  stream: StreamId,
  startBit: number,
  endBit: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectBitRange(
    stream,
    startBit,
    endBit,
  );
  return events;
}

export async function selectSpatialBlock(
  stream: StreamId,
  x: number,
  y: number,
  w: number,
  h: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectSpatialBlock(
    stream,
    x,
    y,
    w,
    h,
  );
  return events;
}
