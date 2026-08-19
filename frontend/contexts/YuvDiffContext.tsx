/**
 * YuvDiff Context
 *
 * State management for the YUVDiff (debug YUV) comparison mode.
 * Mirrors the VQ Analyzer "Load Debug YUV" workflow:
 *   1. User opens a raw YUV reference file via the Debug menu.
 *   2. Context stores the loaded state + display mode choice.
 *   3. VideoCanvas fetches frames via get_debug_yuv_frame when active.
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from "react";
import {
  loadDebugYuv,
  unloadDebugYuv,
  setDebugYuvOffset as bridgeSetDebugYuvOffset,
  getYuvDiffMetrics,
  findFirstDiffFrame,
} from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("YuvDiffContext");

// ─── Types ────────────────────────────────────────────────────────────────────

export type YuvFormat = "i420" | "nv12" | "nv21" | "i422" | "i444";

export type YuvDiffDisplayMode = "decoded" | "reference" | "diff" | "amplified";

export interface YuvDiffMetrics {
  frame_index: number;
  psnr_y: number;
  psnr_u: number;
  psnr_v: number;
  psnr_avg: number;
  ssim_y: number;
  max_diff_y: number;
  has_mismatch: boolean;
}

export interface YuvDiffLoadParams {
  path: string;
  width: number;
  height: number;
  format: YuvFormat;
  bitdepth: number;
  picture_offset?: number;
  crop?: { left: number; right: number; top: number; bottom: number };
}

export interface YuvDiffState {
  /** Whether a reference YUV file is currently loaded. */
  isLoaded: boolean;
  /** Path to the loaded reference file. */
  path: string | null;
  /** Total frames available in the reference file. */
  frameCount: number;
  /** Current display mode. */
  displayMode: YuvDiffDisplayMode;
  /** Amplification factor used when displayMode === "amplified". */
  amplifyFactor: number;
  /** Frame dimensions from the loaded file. */
  width: number;
  height: number;
  /** Bit depth (8 or 10). */
  bitdepth: number;
  /** Frame offset between display index and reference index. */
  pictureOffset: number;
  /** Per-frame quality metrics (populated on demand). */
  metrics: YuvDiffMetrics | null;
  /** True while a backend operation is in flight. */
  loading: boolean;
  /** Last error message from backend operations. */
  error: string | null;
}

interface YuvDiffContextType extends YuvDiffState {
  loadFile: (params: YuvDiffLoadParams) => Promise<void>;
  unloadFile: () => void;
  setDisplayMode: (mode: YuvDiffDisplayMode) => void;
  setAmplifyFactor: (factor: number) => void;
  setPictureOffset: (offset: number) => void;
  fetchMetrics: (frameIndex: number) => Promise<YuvDiffMetrics | null>;
  findFirstDiff: () => Promise<number | null>;
}

// ─── Context ──────────────────────────────────────────────────────────────────

const YuvDiffContext = createContext<YuvDiffContextType | null>(null);

const INITIAL_STATE: YuvDiffState = {
  isLoaded: false,
  path: null,
  frameCount: 0,
  displayMode: "decoded",
  amplifyFactor: 8,
  width: 0,
  height: 0,
  bitdepth: 8,
  pictureOffset: 0,
  metrics: null,
  loading: false,
  error: null,
};

// ─── Provider ─────────────────────────────────────────────────────────────────

export function YuvDiffProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<YuvDiffState>(INITIAL_STATE);

  const loadFile = useCallback(async (params: YuvDiffLoadParams) => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const result = await loadDebugYuv(params);

      if (!result.success) {
        setState((s) => ({
          ...s,
          loading: false,
          error: result.error ?? "Failed to load YUV file",
        }));
        return;
      }

      setState((s) => ({
        ...s,
        isLoaded: true,
        path: params.path,
        frameCount: result.frame_count,
        width: params.width,
        height: params.height,
        bitdepth: params.bitdepth,
        pictureOffset: params.picture_offset ?? 0,
        metrics: null,
        loading: false,
        error: null,
      }));

      logger.info(
        `Loaded debug YUV: ${params.path} (${result.frame_count} frames)`,
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      logger.error("loadFile failed:", msg);
      setState((s) => ({ ...s, loading: false, error: msg }));
    }
  }, []);

  const unloadFile = useCallback(() => {
    unloadDebugYuv().catch((e) => logger.warn("unloadDebugYuv:", e));
    setState(INITIAL_STATE);
  }, []);

  const setDisplayMode = useCallback((mode: YuvDiffDisplayMode) => {
    setState((s) => ({ ...s, displayMode: mode }));
  }, []);

  const setAmplifyFactor = useCallback((factor: number) => {
    setState((s) => ({ ...s, amplifyFactor: factor }));
  }, []);

  const setPictureOffset = useCallback((offset: number) => {
    setState((s) => ({ ...s, pictureOffset: offset }));
    bridgeSetDebugYuvOffset(offset).catch((e) =>
      logger.warn("setDebugYuvOffset:", e),
    );
  }, []);

  const fetchMetrics = useCallback(
    async (frameIndex: number): Promise<YuvDiffMetrics | null> => {
      if (!state.isLoaded) return null;
      try {
        const metrics = await getYuvDiffMetrics(frameIndex);
        setState((s) => ({ ...s, metrics }));
        return metrics;
      } catch (err) {
        logger.warn("fetchMetrics:", err);
        return null;
      }
    },
    [state.isLoaded],
  );

  const findFirstDiff = useCallback(async (): Promise<number | null> => {
    if (!state.isLoaded) return null;
    setState((s) => ({ ...s, loading: true }));
    try {
      const result = await findFirstDiffFrame();
      setState((s) => ({ ...s, loading: false }));
      return result.frame_index ?? null;
    } catch (err) {
      logger.warn("findFirstDiff:", err);
      setState((s) => ({ ...s, loading: false }));
      return null;
    }
  }, [state.isLoaded]);

  return (
    <YuvDiffContext.Provider
      value={{
        ...state,
        loadFile,
        unloadFile,
        setDisplayMode,
        setAmplifyFactor,
        setPictureOffset,
        fetchMetrics,
        findFirstDiff,
      }}
    >
      {children}
    </YuvDiffContext.Provider>
  );
}

// ─── Hook ─────────────────────────────────────────────────────────────────────

export function useYuvDiff(): YuvDiffContextType {
  const ctx = useContext(YuvDiffContext);
  if (!ctx) throw new Error("useYuvDiff must be used inside YuvDiffProvider");
  return ctx;
}
