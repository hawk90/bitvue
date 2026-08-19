/**
 * Compare Context - Manages A/B compare workspace state
 *
 * Wired to the real `bitvue-sidecar` compare commands (docs/DEVELOPMENT_PHASES.md Phase 7.5) as
 * of this pass -- previously this called `@tauri-apps/api/core`'s `invoke()` with method names
 * (`create_compare_workspace`/`set_sync_mode`/etc.) that never existed anywhere post-Electron-
 * migration, so this context was 100% dead plumbing. `createWorkspace` no longer takes stream
 * paths: the real backend always operates on whichever streams are currently open as A/B (see
 * `bitvue-sidecar/src/compare.rs`'s doc) -- opening stream B (and indexing both streams) is the
 * caller's job before calling this, same as it already is for stream A via the normal
 * open-file flow (`FileStateContext.tsx`). See `hooks/useAppFileOperations.ts`'s
 * `handleOpenDependentFile` for the real caller.
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  ReactNode,
  useMemo,
} from "react";
import {
  createCompareWorkspace,
  getAlignedFrame as bridgeGetAlignedFrame,
  setSyncMode as bridgeSetSyncMode,
  setManualOffset as bridgeSetManualOffset,
  resetOffset as bridgeResetOffset,
  findFirstDiffFrameAb,
  getFramesChunk,
  type CompareWorkspaceSummary,
} from "../services/electronBridgeService";
import { unitNodeToFrameInfo } from "./FileStateContext";
import type { SyncMode, AlignmentQuality, FrameInfo } from "../types/video";

const FRAMES_B_CHUNK_SIZE = 200;

/** Fetches all of stream B's frame metadata in one go (no chunked-progressive-loading UX --
 *  `FileStateContext.tsx`'s doc for why stream B doesn't need that here). Stream B must already
 *  be open + indexed (`handleOpenDependentFile`'s job) by the time this runs. */
async function loadAllFramesB(): Promise<FrameInfo[]> {
  const first = await getFramesChunk("B", 0, FRAMES_B_CHUNK_SIZE);
  let allFrames = first.units.map(unitNodeToFrameInfo);
  let offset = allFrames.length;
  while (offset < first.total_count) {
    const next = await getFramesChunk("B", offset, FRAMES_B_CHUNK_SIZE);
    if (next.units.length === 0) break;
    const nextFrames = next.units.map(unitNodeToFrameInfo);
    allFrames = [...allFrames, ...nextFrames];
    offset += nextFrames.length;
  }
  return allFrames;
}

const toMessage = (err: unknown): string =>
  err instanceof Error ? err.message : String(err);

interface CompareContextType {
  // Compare workspace state
  workspace: CompareWorkspaceSummary | null;
  isLoading: boolean;
  error: string | null;

  // CMP-04: true while findFirstDiffFrame's frame-by-frame scan is in flight (can take real
  // time on a long stream -- separate from `isLoading`, which is workspace-creation-scoped).
  isScanningDiff: boolean;

  // Stream B's frame metadata (stream A's already lives in FileStateContext/FrameDataContext)
  framesB: FrameInfo[];

  // Current frames for each stream
  currentFrameA: number;
  currentFrameB: number;

  // Actions
  createWorkspace: () => Promise<void>;
  closeWorkspace: () => void;
  setFrameA: (index: number) => void;
  setFrameB: (index: number) => void;
  setSyncMode: (mode: SyncMode) => Promise<void>;
  setManualOffset: (offset: number) => Promise<void>;
  resetOffset: () => Promise<void>;
  getAlignedFrame: (
    streamAIdx: number,
  ) => Promise<{ bIdx: number | null; quality: AlignmentQuality | null }>;
  /** PARITY_CHECKLIST.md CMP-04. Returns the first stream A frame index with a real pixel
   *  difference against its aligned B frame, or `null` if none was found. */
  findFirstDiffFrame: () => Promise<{
    frameIndex: number | null;
    totalChecked: number;
  }>;
}

const CompareContext = createContext<CompareContextType | null>(null);

export function CompareProvider({ children }: { children: ReactNode }) {
  const [workspace, setWorkspace] = useState<CompareWorkspaceSummary | null>(
    null,
  );
  const [isLoading, setIsLoading] = useState(false);
  const [isScanningDiff, setIsScanningDiff] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [framesB, setFramesB] = useState<FrameInfo[]>([]);
  const [currentFrameA, setCurrentFrameA] = useState(0);
  const [currentFrameB, setCurrentFrameB] = useState(0);

  const createWorkspace = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [result, loadedFramesB] = await Promise.all([
        createCompareWorkspace(),
        loadAllFramesB(),
      ]);
      setWorkspace(result);
      setFramesB(loadedFramesB);
      setCurrentFrameA(0);
      setCurrentFrameB(0);
    } catch (err) {
      setError(toMessage(err));
      console.error("Failed to create compare workspace:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const closeWorkspace = useCallback(() => {
    setWorkspace(null);
    setFramesB([]);
    setCurrentFrameA(0);
    setCurrentFrameB(0);
    setError(null);
  }, []);

  const setFrameA = useCallback((index: number) => {
    setCurrentFrameA(index);
  }, []);

  const setFrameB = useCallback((index: number) => {
    setCurrentFrameB(index);
  }, []);

  const setSyncMode = useCallback(
    async (mode: SyncMode) => {
      try {
        await bridgeSetSyncMode(mode);
        if (workspace) {
          setWorkspace({ ...workspace, sync_mode: mode });
        }
      } catch (err) {
        setError(toMessage(err));
        console.error("Failed to set sync mode:", err);
      }
    },
    [workspace],
  );

  const setManualOffset = useCallback(
    async (offset: number) => {
      try {
        await bridgeSetManualOffset(offset);
        if (workspace) {
          setWorkspace({ ...workspace, manual_offset: offset });
        }
      } catch (err) {
        setError(toMessage(err));
        console.error("Failed to set manual offset:", err);
      }
    },
    [workspace],
  );

  const resetOffset = useCallback(async () => {
    try {
      await bridgeResetOffset();
      if (workspace) {
        setWorkspace({ ...workspace, manual_offset: 0 });
      }
    } catch (err) {
      setError(toMessage(err));
      console.error("Failed to reset offset:", err);
    }
  }, [workspace]);

  const getAlignedFrame = useCallback(async (streamAIdx: number) => {
    try {
      const result = await bridgeGetAlignedFrame(streamAIdx);
      return {
        bIdx: result.stream_b_frame_idx,
        quality: result.quality,
      };
    } catch (err) {
      setError(toMessage(err));
      console.error("Failed to get aligned frame:", err);
      return { bIdx: null, quality: null };
    }
  }, []);

  const findFirstDiffFrame = useCallback(async () => {
    setIsScanningDiff(true);
    setError(null);
    try {
      const result = await findFirstDiffFrameAb();
      return {
        frameIndex: result.frame_index,
        totalChecked: result.total_checked,
      };
    } catch (err) {
      setError(toMessage(err));
      console.error("Failed to find first diff frame:", err);
      return { frameIndex: null, totalChecked: 0 };
    } finally {
      setIsScanningDiff(false);
    }
  }, []);

  // Memoize context value to prevent unnecessary re-renders in consumers
  const value = useMemo<CompareContextType>(
    () => ({
      workspace,
      isLoading,
      isScanningDiff,
      error,
      framesB,
      currentFrameA,
      currentFrameB,
      createWorkspace,
      closeWorkspace,
      setFrameA,
      setFrameB,
      setSyncMode,
      setManualOffset,
      resetOffset,
      getAlignedFrame,
      findFirstDiffFrame,
    }),
    [
      workspace,
      isLoading,
      isScanningDiff,
      error,
      framesB,
      currentFrameA,
      currentFrameB,
      createWorkspace,
      closeWorkspace,
      setFrameA,
      setFrameB,
      setSyncMode,
      setManualOffset,
      resetOffset,
      getAlignedFrame,
      findFirstDiffFrame,
    ],
  );

  return (
    <CompareContext.Provider value={value}>{children}</CompareContext.Provider>
  );
}

export function useCompare(): CompareContextType {
  const context = useContext(CompareContext);
  if (!context) {
    throw new Error("useCompare must be used within CompareProvider");
  }
  return context;
}
