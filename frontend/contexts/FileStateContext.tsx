/**
 * File State Context
 *
 * Manages file loading state and operations
 * Separated from frame navigation to prevent unnecessary re-renders
 * Supports chunked frame loading for faster initial load
 *
 * `refreshFrames`/`loadMoreFrames` go through `electronBridgeService` (bitvue-sidecar's
 * `index_stream`/`get_frames_chunk`) as of the 2026-08-08 migration, not Tauri's `invoke()`.
 * `bitvue-indexer` only supports IVF/AV1 so far -- other formats/codecs surface as an error here
 * (via a `DiagnosticAdded` event from `indexStream`, or an empty `indexed: false` chunk), same as
 * any other unsupported-input case. Only stream "A" is wired here (matches
 * `useAppFileOperations.ts`'s single-primary-stream assumption) -- stream B's frames (compare
 * workspace, docs/DEVELOPMENT_PHASES.md Phase 7.5) are loaded separately by
 * `CompareContext.tsx`, which reuses this file's `unitNodeToFrameInfo` but not its chunked-
 * progressive-loading state machine (stream B doesn't need the same large-file UX as the
 * primary stream for an MVP compare view).
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useRef,
  ReactNode,
  useMemo,
} from "react";
import type { FrameInfo } from "../types/video";
import { createLogger } from "../utils/logger";
import { useFrameData } from "./FrameDataContext";
import {
  indexStream,
  getFramesChunk,
  getStreamInfo,
  getTimeline,
  extractDiagnostics,
  type BridgeUnitNode,
  type BridgeDiagnostic,
} from "../services/electronBridgeService";

const logger = createLogger("FileStateContext");

/** `bitvue_engine::UnitNode` -> `FrameInfo`. Only fields the sidecar actually provides are
 *  populated -- poc/display_order/coding_order/spatial_id/thumbnail/duration/ref_slot_info have
 *  no bitvue-indexer equivalent yet, left undefined rather than fabricated. Exported for
 *  `CompareContext.tsx`'s stream-B frame loading (docs/DEVELOPMENT_PHASES.md Phase 7.5) -- same
 *  wire shape, no need for a second copy of this mapping. */
export function unitNodeToFrameInfo(unit: BridgeUnitNode): FrameInfo {
  return {
    frame_index: unit.frame_index ?? 0,
    frame_type: unit.frame_type ?? "?",
    size: unit.size,
    offset: unit.offset,
    pts: unit.pts ?? undefined,
    temporal_id: unit.temporal_id ?? undefined,
    key_frame: unit.frame_type === "I",
    ref_frames: unit.ref_frames ?? undefined,
    ref_slots: unit.ref_slots ?? undefined,
  };
}

/** Merges freshly-indexed base frame metadata (`nextFrames`, from `unitNodeToFrameInfo`) over
 *  `prevFrames`, preserving any per-frame analysis fields (`qp_grid`/`prediction_mode_grid`/etc.)
 *  `YuvViewerPanel.loadFrameAnalysis` may have already merged into `prevFrames[i]` -- indexing
 *  runs in progressive chunks (`CHUNK_SIZE` at a time) and re-flushes the *entire* frames array
 *  on every flush, so a naive `setFrames([...allFrames])` silently wipes out any analysis grid
 *  data merged in during the window between flushes (real, reproducible bug: fetch a frame's
 *  analysis while a >100-frame stream is still progressively indexing, and the next chunk flush
 *  discards it -- `unitNodeToFrameInfo` never sets grid fields at all, so they'd otherwise just
 *  vanish from the merged object). `nextFrames`' own fields always win (indexing is the source of
 *  truth for frame_type/pts/size/etc.); only fields `nextFrames` doesn't set are preserved from
 *  `prevFrames`. */
function mergePreservingAnalysis(
  prevFrames: FrameInfo[],
  nextFrames: FrameInfo[],
): FrameInfo[] {
  return nextFrames.map((f, i) =>
    prevFrames[i] ? { ...prevFrames[i], ...f } : f,
  );
}

/** Populates `display_order`/`coding_order` on already-loaded decode-order `frames` by joining
 *  against `get_timeline`'s real PTS-sorted display index (`bitvue_engine::frame_identity::
 *  FrameIndexMap`, CTX-02 fix). These two fields were always left `undefined` by
 *  `unitNodeToFrameInfo` above ("no bitvue-indexer equivalent yet") even though several real
 *  consumers already render them when present (DetailsPanel/StatisticsTab/FrameSyntaxTab/
 *  ThumbnailsView/VirtualizedThumbnailsView/DebugPanel/dataExport CSV+JSON). Joins by `pts`, not
 *  `decode_idx` -- `FrameIndexMap`'s decode_idx is intentionally internal-only per
 *  `frame_identity/timeline_extractor.rs`'s module doc ("decode_idx is internal only and must not
 *  be exposed"), and `get_timeline`'s response already carries real per-entry `pts`, so no wire
 *  change is needed to do this join. PTS is assumed unique per frame in a well-formed stream;
 *  frames whose PTS is missing or duplicated in the timeline response (i.e. `PtsQuality::Bad`/
 *  `Warn`) are left with `display_order`/`coding_order` undefined rather than a fabricated guess
 *  -- same "don't fabricate" convention as `unitNodeToFrameInfo`.
 *
 *  Scope note: this fixes the *data* only. `coding_order` here is just the frame's existing
 *  `frame_index` (decode order) -- every backend frame-fetch call (`getDecodedFrameYuv`,
 *  `getFrameAnalysis`, ...) still keys on and must keep keying on that same decode-order array
 *  position. `Timeline.tsx`'s bars still render/scrub/key by array position, not by this new
 *  `display_order` -- physically reordering the visual timeline to match display order is a
 *  separate, larger, riskier UI change (arrow-key nav, hit-testing, and every
 *  `setFrameSelection` call currently assume array position === decode-order `frame_index`),
 *  deliberately deferred rather than attempted here. */
interface DisplayOrderResult {
  frames: FrameInfo[];
  /** EDGE-03: stream-wide PTS quality from the same `getTimeline` call -- `null` if the call
   *  failed (same non-fatal fallback as `display_order`/`coding_order` below). */
  ptsQuality: "Ok" | "Warn" | "Bad" | null;
}

async function applyDisplayOrder(
  frames: FrameInfo[],
): Promise<DisplayOrderResult> {
  try {
    const timeline = await getTimeline("A");
    const displayIdxByPts = new Map<number, number>();
    const ambiguousPts = new Set<number>();
    for (const entry of timeline.frames) {
      if (entry.pts === null) continue;
      if (displayIdxByPts.has(entry.pts)) {
        ambiguousPts.add(entry.pts);
        continue;
      }
      displayIdxByPts.set(entry.pts, entry.display_idx);
    }
    for (const pts of ambiguousPts) displayIdxByPts.delete(pts);

    const withDisplayOrder = frames.map((f) => {
      if (f.pts === undefined) return f;
      const display_order = displayIdxByPts.get(f.pts);
      if (display_order === undefined) return f;
      return { ...f, display_order, coding_order: f.frame_index };
    });
    return { frames: withDisplayOrder, ptsQuality: timeline.pts_quality };
  } catch (err) {
    // Non-fatal, same reasoning as getStreamInfo above -- decode-order frame data is the
    // primary path, display_order/ptsQuality are supplementary (fall back to existing "N/A"
    // rendering / null).
    logger.error("Failed to load display-order timeline:", err);
    return { frames, ptsQuality: null };
  }
}

interface FileStateContextType {
  filePath: string | null;
  loading: boolean;
  error: string | null;
  /** Real `DiagnosticAdded` events surfaced by the most recent `indexStream` call (e.g.
   *  unsupported codec/container, a mismatched IVF fourcc, a per-frame parse failure) -- see
   *  `DiagnosticsPanel.tsx`, which renders these instead of fabricated data. */
  diagnostics: BridgeDiagnostic[];
  setFilePath: (path: string | null) => void;
  refreshFrames: () => Promise<FrameInfo[]>;
  loadMoreFrames: () => Promise<FrameInfo[]>;
  hasMoreFrames: boolean;
  totalFrames: number;
  clearData: () => void;
}

const FileStateContext = createContext<FileStateContextType | undefined>(
  undefined,
);

// Chunk size for progressive loading
const CHUNK_SIZE = 100;
// Threshold for using chunked loading (200+ frames)
const CHUNKED_LOADING_THRESHOLD = 200;

export function FileStateProvider({ children }: { children: ReactNode }) {
  const { setFrames, setStreamInfo } = useFrameData();
  const [filePath, setFilePath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMoreFrames, setHasMoreFrames] = useState(false);
  const [totalFrames, setTotalFrames] = useState(0);
  const [diagnostics, setDiagnostics] = useState<BridgeDiagnostic[]>([]);

  const currentOffsetRef = useRef(0);
  const isLoadingMoreRef = useRef(false);

  // Load frames from backend (with chunking support)
  const refreshFrames = useCallback(async () => {
    setLoading(true);
    setError(null);
    currentOffsetRef.current = 0;
    setHasMoreFrames(false);
    setTotalFrames(0);
    setFrames([]);
    setStreamInfo(null);
    setDiagnostics([]);

    try {
      logger.info(
        "refreshFrames: indexing stream, then calling get_frames_chunk...",
      );
      const startTime = performance.now();

      const indexEvents = await indexStream("A");
      setDiagnostics(extractDiagnostics(indexEvents));

      // Container-level width/height/codec (`get_stream_info`, populated by `index_stream` above)
      // -- real data for `SelectionInfoPanel`'s "Video Properties" section, which used to have no
      // source at all and silently rendered hardcoded 1920x1080/AV1 placeholders for every file.
      try {
        const info = await getStreamInfo("A");
        if (info.indexed && info.container) {
          setStreamInfo({
            width: info.container.width ?? 0,
            height: info.container.height ?? 0,
            codec: info.container.codec,
            bitDepth: info.container.bit_depth,
            // Filled in below once applyDisplayOrder's getTimeline call resolves -- a separate
            // sidecar call, not part of get_stream_info's response.
            ptsQuality: null,
          });
        }
      } catch (streamInfoErr) {
        // Non-fatal: frame loading below is the primary data path, stream info is supplementary.
        logger.error("Failed to load stream info:", streamInfoErr);
      }

      const firstChunk = await getFramesChunk("A", 0, CHUNK_SIZE);
      const allFrames = firstChunk.units.map(unitNodeToFrameInfo);
      currentOffsetRef.current = allFrames.length;
      setHasMoreFrames(currentOffsetRef.current < firstChunk.total_count);
      setTotalFrames(firstChunk.total_count);
      setFrames((prev) => mergePreservingAnalysis(prev, allFrames));

      // Flushing every chunk to React state (100 frames at a time) made `setFrames` re-render
      // Timeline's unvirtualized per-frame DOM list once per chunk -- for a long video (many
      // thousands of frames -> hundreds of chunks), that's hundreds of increasingly expensive
      // re-renders of an ever-growing list, turning a sub-3-second backend load (measured: 60k
      // frames / 600 chunks in ~2.4s end to end) into a many-times-slower "looks frozen" UI
      // experience. Batching the state flush every FLUSH_EVERY_N_CHUNKS chunks instead cuts the
      // re-render count proportionally while still showing real progressive-load feedback (not
      // one big flush at the very end) -- `allFrames.push` (mutate in place) also avoids
      // `[...allFrames, ...nextFrames]`'s per-iteration full-array copy, which was separately
      // O(total frames) per chunk on its own regardless of React.
      const FLUSH_EVERY_N_CHUNKS = 10;
      let chunksSinceFlush = 0;
      while (currentOffsetRef.current < firstChunk.total_count) {
        const nextChunk = await getFramesChunk(
          "A",
          currentOffsetRef.current,
          CHUNK_SIZE,
        );
        if (nextChunk.units.length === 0) break;
        const nextFrames = nextChunk.units.map(unitNodeToFrameInfo);
        allFrames.push(...nextFrames);
        currentOffsetRef.current += nextFrames.length;
        setHasMoreFrames(currentOffsetRef.current < nextChunk.total_count);
        setTotalFrames(nextChunk.total_count);
        chunksSinceFlush++;
        if (
          chunksSinceFlush >= FLUSH_EVERY_N_CHUNKS ||
          currentOffsetRef.current >= nextChunk.total_count
        ) {
          setFrames((prev) => mergePreservingAnalysis(prev, allFrames));
          chunksSinceFlush = 0;
        }
      }

      const elapsed = performance.now() - startTime;
      logger.info(
        `refreshFrames: Loaded ${allFrames.length} frames in ${elapsed.toFixed(2)}ms`,
      );

      if (allFrames.length >= CHUNKED_LOADING_THRESHOLD) {
        logger.info(
          `refreshFrames: Loaded large file progressively (${allFrames.length} frames)`,
        );
      }

      // CTX-02/EDGE-03 fix: populate real display_order/coding_order and pts_quality (see
      // applyDisplayOrder's doc). Done once over the full stream (not per-chunk) since
      // FrameIndexMap's PTS sort needs the whole stream to be correct -- a per-chunk sort could
      // split a reordered GOP across a chunk boundary.
      const { frames: framesWithDisplayOrder, ptsQuality } =
        await applyDisplayOrder(allFrames);
      setFrames((prev) =>
        mergePreservingAnalysis(prev, framesWithDisplayOrder),
      );
      setStreamInfo((prev) => (prev ? { ...prev, ptsQuality } : prev));

      setHasMoreFrames(false);
      return framesWithDisplayOrder;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
      logger.error("Failed to load frames:", errorMsg);
      setFrames([]);
      setStreamInfo(null);
      return [];
    } finally {
      setLoading(false);
    }
  }, [setFrames, setStreamInfo]);

  // Load more frames using chunked loading
  const loadMoreFrames = useCallback(async () => {
    // Prevent concurrent loading
    if (isLoadingMoreRef.current || !hasMoreFrames) {
      return [];
    }

    isLoadingMoreRef.current = true;

    try {
      logger.info(
        `loadMoreFrames: Loading chunk at offset ${currentOffsetRef.current}`,
      );

      const result = await getFramesChunk(
        "A",
        currentOffsetRef.current,
        CHUNK_SIZE,
      );
      const frames = result.units.map(unitNodeToFrameInfo);

      logger.info(
        `loadMoreFrames: Got ${frames.length} frames, total: ${result.total_count}`,
      );

      // Update state for next chunk
      currentOffsetRef.current += frames.length;
      setHasMoreFrames(currentOffsetRef.current < result.total_count);
      setTotalFrames(result.total_count);

      return frames;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      logger.error("Failed to load frames chunk:", errorMsg);
      setError(errorMsg);
      return [];
    } finally {
      isLoadingMoreRef.current = false;
    }
  }, [hasMoreFrames]);

  // Clear all data (used when closing file)
  const clearData = useCallback(() => {
    setFilePath(null);
    setError(null);
    setLoading(false);
    setHasMoreFrames(false);
    setTotalFrames(0);
    currentOffsetRef.current = 0;
    isLoadingMoreRef.current = false;
    setFrames([]);
    setStreamInfo(null);
    setDiagnostics([]);
  }, [setFrames, setStreamInfo]);

  const contextValue = useMemo<FileStateContextType>(
    () => ({
      filePath,
      loading,
      error,
      diagnostics,
      setFilePath,
      refreshFrames,
      loadMoreFrames,
      hasMoreFrames,
      totalFrames,
      clearData,
    }),
    [
      filePath,
      loading,
      error,
      diagnostics,
      refreshFrames,
      loadMoreFrames,
      hasMoreFrames,
      totalFrames,
      clearData,
    ],
  );

  return (
    <FileStateContext.Provider value={contextValue}>
      {children}
    </FileStateContext.Provider>
  );
}

export function useFileState(): FileStateContextType {
  const context = useContext(FileStateContext);
  if (!context) {
    throw new Error("useFileState must be used within a FileStateProvider");
  }
  return context;
}

export type { FileStateContextType };
