/**
 * Selection Types
 *
 * Type definitions for the selection system.
 * Extracted from SelectionContext for better modularity.
 */

export type StreamId = "A" | "B";

export interface SpatialBlock {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type TemporalSelectionType = "block" | "point" | "range" | "marker";

export interface TemporalSelection {
  type: TemporalSelectionType;
  frameIndex: number;
  block?: SpatialBlock;
  rangeStart?: number;
  rangeEnd?: number;
}

export interface FrameKey {
  stream: StreamId;
  frameIndex: number;
  pts?: number;
}

export interface UnitKey {
  stream: StreamId;
  unitType: string;
  offset: number;
  size: number;
}

/** Flat dotted-path string (e.g. "obu_header.obu_type"), matching the backend's actual
 *  `bitvue_engine::SyntaxNodeId = String` exactly -- this used to be a `{path, fieldType, offset}`
 *  struct that never matched the real wire shape (nothing had ever round-tripped a real one
 *  through it to notice). See types/selection.ts's SelectionState doc for the broader Tri-Sync
 *  completion this is part of. */
export type SyntaxNodeId = string;

export interface BitRange {
  startBit: number;
  endBit: number;
}

export type SelectionPanel =
  | "syntax"
  | "hex"
  | "main"
  | "timeline"
  | "filmstrip"
  | "reference-lists"
  | "keyboard"
  | "minimap"
  | "bookmarks"
  | "sync";

export interface SelectionSource {
  panel: SelectionPanel;
  timestamp: number;
}

/**
 * Invariants (axis-2 Context-ownership cleanup, see contexts/FrameSyncBridge.tsx's doc for the
 * problem this is fixing):
 *
 * 1. `frame` (+ `temporal.frameIndex` when `temporal.type === "point"`, kept equal by
 *    `applyTriSyncRules` Rule 1) is the SINGLE source of truth for "the active frame" -- the one
 *    `contexts/CurrentFrameContext.tsx` and `FrameSyncBridge` currently duplicate into a second,
 *    independently-`useState`'d `currentFrameIndex`. Read it via `useActiveFrame()`
 *    (contexts/SelectionContext.tsx), not a second piece of state.
 * 2. `unit`/`syntaxNode`/`bitRange` are independent "rich selection" facets. Setting any of them
 *    (`setUnitSelection`/`setSyntaxSelection`/`setBitRangeSelection`) must NEVER overwrite
 *    `frame` -- verified true today (`applyTriSyncRules`' Rules 2/3 only ever touch `bitRange`).
 *    Only `setFrameSelection`/`setTemporalSelection` (and Rule 1) may change `frame`.
 * 3. `streamId` is meant to track `frame.stream`, but nothing currently enforces that --
 *    `setUnitSelection` sets `streamId: unit.stream` without touching `frame.stream`, so a unit
 *    selected on a different stream than the active frame can silently desync the two. Currently
 *    low-risk only because `selection.streamId` has zero real (non-test) consumers as of this
 *    writing (grepped) -- flagged here rather than silently "fixed" because reconciling it is a
 *    product decision (should selecting a stream-B unit move the active frame to stream B?), not
 *    a pure plumbing bug. Don't add a new real consumer of `streamId` without resolving this.
 */
export interface SelectionState {
  streamId: StreamId;
  temporal: TemporalSelection | null;
  frame: FrameKey | null;
  unit: UnitKey | null;
  syntaxNode: SyntaxNodeId | null;
  bitRange: BitRange | null;
  source: SelectionSource;
}

export interface SelectionChangeEvent {
  /** null for a clearAll() -- there's no SelectionState left to report. */
  selection: SelectionState | null;
  source: SelectionSource;
}

export interface SelectionContextType {
  selection: SelectionState | null;
  setTemporalSelection: (
    selection: TemporalSelection,
    source: SelectionPanel,
  ) => void;
  setFrameSelection: (frame: FrameKey, source: SelectionPanel) => void;
  setUnitSelection: (unit: UnitKey, source: SelectionPanel) => void;
  /** `bitRange` is required (not derived/estimated) -- matches the backend's
   *  `Command::SelectSyntax { node_id, bit_range }` exactly, which needs both together. Callers
   *  (e.g. FrameSyntaxTab) already have the real bit_range from the fetched syntax tree data. */
  setSyntaxSelection: (
    node: SyntaxNodeId,
    bitRange: BitRange,
    source: SelectionPanel,
  ) => void;
  /** Also calls the real backend `select_bit_range` round trip and merges the resolved
   *  `syntaxNode` back in once it resolves -- see contexts/SelectionContext.tsx's doc. */
  setBitRangeSelection: (range: BitRange, source: SelectionPanel) => void;
  /** INT-01 (Player click→spatialBlock select): optimistically sets local `temporal` state
   *  (type "block"), then also calls the real backend `select_spatial_block` so server-side
   *  selection state stays in sync -- same "optimistic local + real bridge call" shape as
   *  `setBitRangeSelection`. */
  setSpatialBlockSelection: (
    block: SpatialBlock,
    frameIndex: number,
    source: SelectionPanel,
  ) => void;
  clearTemporal: () => void;
  clearAll: () => void;
  subscribe: (callback: (event: SelectionChangeEvent) => void) => () => void;
}

export const DEFAULT_SELECTION: SelectionState = {
  streamId: "A",
  temporal: null,
  frame: null,
  unit: null,
  syntaxNode: null,
  bitRange: null,
  source: {
    panel: "timeline",
    timestamp: Date.now(),
  },
};
