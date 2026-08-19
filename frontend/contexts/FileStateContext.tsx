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
  type BridgeUnitNode,
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

interface FileStateContextType {
  filePath: string | null;
  loading: boolean;
  error: string | null;
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

    try {
      logger.info(
        "refreshFrames: indexing stream, then calling get_frames_chunk...",
      );
      const startTime = performance.now();

      await indexStream("A");

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
      setFrames([...allFrames]);

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
          setFrames([...allFrames]);
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

      setHasMoreFrames(false);
      return allFrames;
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
  }, [setFrames, setStreamInfo]);

  const contextValue = useMemo<FileStateContextType>(
    () => ({
      filePath,
      loading,
      error,
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
