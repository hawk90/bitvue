/**
 * Electron bridge core — shared plumbing for every domain module under `services/bridge/`.
 *
 * Owns the one consolidated `Window.bitvue` IPC contract (every channel `preload.cjs` exposes,
 * see `bitvue-desktop/electron/preload.cjs`), `requireBridge()` (the guard every domain wrapper
 * calls through), and the two cross-cutting helpers (`hasElectronBridge`/`setHasOpenFile`) that
 * don't belong to any single feature domain.
 *
 * Domain modules (`stream.ts`, `frameDecode.ts`, `frameAnalysis.ts`, `syntaxHex.ts`,
 * `compareDiff.ts`, `evidenceExport.ts`, `windowLifecycle.ts`) each own their own result/param
 * wire types and import only `requireBridge`/`StreamId`/`BridgeEvent` from here (plus
 * `BridgeDecodedYuvFrame` from `frameDecode.ts` where a domain returns decoded pixel data).
 * `electronBridgeService.ts` re-exports every domain module as a single stable import path so no
 * consumer needed to change when this file was split (2026-08-20 Tauri-leftover/axis-1 cleanup —
 * see docs/DEVELOPMENT_PHASES.md's session log for the "why split" rationale: this was previously
 * one 1006-line file mixing type declarations, IPC contract, and wrapper functions for every
 * feature area).
 */

import type { Av1FeaturesWireResult } from "./frameAnalysis";
import type { CodingFlowAnalysisWireResult } from "./frameAnalysis";
import type { DeblockingAnalysisWireResult } from "./frameAnalysis";
import type { CodecExtendedInfoWireResult } from "./frameAnalysis";
import type { ResidualAnalysisWireResult } from "./frameAnalysis";
import type { FrameAnalysisWireResult } from "./frameAnalysis";
import type { BridgeDecodedYuvFrame } from "./frameDecode";
import type {
  StreamInfoResult,
  FramesChunkResult,
  BridgeTimeline,
  BridgeThumbnailResult,
} from "./stream";
import type { BridgeSyntaxNode } from "./syntaxHex";
import type {
  LoadDebugYuvParams,
  LoadDebugYuvResult,
  DebugYuvCrop,
  YuvDiffMetricsResult,
  FindFirstDiffFrameResult,
  CompareWorkspaceSummary,
  AlignedFrameResult,
  SyncMode,
  DiffMode,
  DiffHeatmapResult,
  DebugYuvDisplayMode,
} from "./compareDiff";
import type {
  ContextMenuScopeWire,
  ContextMenuItemWire,
  EvidenceBundleExportResultWire,
  ExportEvidenceBundleParams,
} from "./evidenceExport";
import type { OpenFileDialogFilter } from "./windowLifecycle";

export type StreamId = "A" | "B";

/** Shape of the JSON-mapped `bitvue_engine::Event` values the sidecar sends back — see
 *  `bitvue-sidecar/src/main.rs`'s `event_to_json` for the authoritative field set per type. */
export interface BridgeEvent {
  type: string;
  stream?: string;
  [key: string]: unknown;
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
      getDecodedFrameYuvCancellable: (
        requestId: string,
        stream: StreamId,
        frameIndex: number,
      ) => Promise<BridgeDecodedYuvFrame>;
      cancelDecodedFrameYuv: (requestId: string) => Promise<void>;
      loadDebugYuv: (params: LoadDebugYuvParams) => Promise<LoadDebugYuvResult>;
      unloadDebugYuv: () => Promise<void>;
      setDebugYuvOffset: (offset: number) => Promise<void>;
      setDebugYuvCrop: (crop: DebugYuvCrop) => Promise<void>;
      getYuvDiffMetrics: (frameIndex: number) => Promise<YuvDiffMetricsResult>;
      findFirstDiffFrame: () => Promise<FindFirstDiffFrameResult>;
      createCompareWorkspace: () => Promise<CompareWorkspaceSummary>;
      getAlignedFrame: (streamAFrameIdx: number) => Promise<AlignedFrameResult>;
      setSyncMode: (mode: SyncMode) => Promise<{ sync_mode: SyncMode }>;
      setManualOffset: (offset: number) => Promise<{ manual_offset: number }>;
      resetOffset: () => Promise<{ manual_offset: number }>;
      getDiffFrame: (
        streamAFrameIdx: number,
        mode: DiffMode,
      ) => Promise<DiffHeatmapResult>;
      findFirstDiffFrameAb: () => Promise<FindFirstDiffFrameResult>;
      getDebugYuvFrame: (
        frameIndex: number,
        mode: DebugYuvDisplayMode,
        amplify?: number,
      ) => Promise<BridgeDecodedYuvFrame>;
      getFrameAnalysis: (
        frameIndex: number,
      ) => Promise<FrameAnalysisWireResult>;
      getAv1Features: (frameIndex: number) => Promise<Av1FeaturesWireResult>;
      getCodingFlowAnalysis: (
        frameIndex: number,
      ) => Promise<CodingFlowAnalysisWireResult>;
      getDeblockingAnalysis: (
        frameIndex: number,
      ) => Promise<DeblockingAnalysisWireResult>;
      getCodecExtendedInfo: (
        frameIndex: number,
      ) => Promise<CodecExtendedInfoWireResult>;
      getResidualAnalysis: (
        frameIndex: number,
      ) => Promise<ResidualAnalysisWireResult>;
      getContextMenuItems: (
        scope: ContextMenuScopeWire,
        hasSelection: boolean,
        hasByteRange: boolean,
      ) => Promise<{ items: ContextMenuItemWire[] }>;
      exportEvidenceBundle: (
        params: ExportEvidenceBundleParams,
      ) => Promise<EvidenceBundleExportResultWire>;
      captureScreenshot: () => Promise<string | null>;
      showOpenDialog: (
        filters?: OpenFileDialogFilter[],
      ) => Promise<string | null>;
      showDirectoryDialog: () => Promise<string | null>;
      pathExists: (path: string) => Promise<boolean>;
      getSamplePath: (filename: string) => Promise<string>;
      closeWindow: () => Promise<void>;
      minimizeWindow: () => Promise<void>;
      toggleMaximizeWindow: () => Promise<void>;
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
      getThumbnails: (
        stream: StreamId,
        frameIndices: number[],
        targetWidth?: number,
      ) => Promise<BridgeThumbnailResult[]>;
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
    /** Set by App.tsx whenever `fileInfo?.success && frames.length > 0` changes -- the same
     *  condition that already gates rendering the main content (see App.tsx's `mainContent`).
     *  Read synchronously by the main process (`bitvue-desktop/electron/main.ts`'s
     *  `requestQuit`/`hasOpenFileInRenderer`, via `executeJavaScript`) to decide whether closing
     *  the window is worth a confirmation dialog -- there's no tracked "export in progress" or
     *  "unsaved compare workspace" signal anywhere in the frontend/sidecar yet (CompareWorkspace
     *  is real UI as of docs/DEVELOPMENT_PHASES.md Phase 7.5, but still has no "close without
     *  saving" concept of its own -- it's just two open streams, not a persisted session), so
     *  this is deliberately the one *real* signal available today rather than a fabricated one. */
    __BITVUE_HAS_OPEN_FILE__?: boolean;
  }
}

export function requireBridge(): NonNullable<Window["bitvue"]> {
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

/** Records whether there's a real open bitstream right now, for the main process's
 *  quit-confirmation gate (`bitvue-desktop/electron/main.ts`'s `requestQuit`) to read via
 *  `executeJavaScript` before closing the window. See `window.__BITVUE_HAS_OPEN_FILE__`'s doc
 *  above for why this is the signal used instead of a fabricated "unsaved work" flag. Safe to
 *  call outside Electron (plain browser tab / tests) -- just a no-op `window` property write. */
export function setHasOpenFile(hasOpenFile: boolean): void {
  if (typeof window === "undefined") return;
  window.__BITVUE_HAS_OPEN_FILE__ = hasOpenFile;
}
