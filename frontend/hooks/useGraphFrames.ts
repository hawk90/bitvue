/**
 * useGraphFrames
 *
 * The open stream's whole frame list for the graph panels, with a `reload` for their Reload/Retry
 * buttons. Empty (`[]`, no error) when nothing is indexed yet.
 *
 * Two sources. With `provided` (the mounted Bitrate panel passes the frames `FrameDataContext`
 * already holds) nothing is fetched: no loading, no error, and `reload` does nothing because the
 * context owns the data -- the panel hides Reload then (`canReload`). Without it the whole list is
 * walked from the sidecar (`getAllGraphFrames`).
 */

import { useCallback, useState } from "react";
import { useBridgeQuery } from "./useBridgeQuery";
import {
  getAllGraphFrames,
  type GraphFrame,
} from "../services/electronBridgeService";

export interface UseGraphFramesResult {
  frames: GraphFrame[];
  loading: boolean;
  error: string | null;
  reload: () => void;
  /** `false` when `provided` frames are in use (nothing for Reload/Retry to re-fetch). */
  canReload: boolean;
}

export function useGraphFrames(provided?: GraphFrame[]): UseGraphFramesResult {
  const [reloadKey, setReloadKey] = useState(0);
  const { data, loading, error } = useBridgeQuery(
    (signal) => getAllGraphFrames("A", signal),
    [reloadKey],
    { enabled: provided === undefined },
  );
  const reload = useCallback(() => setReloadKey((k) => k + 1), []);

  if (provided !== undefined) {
    return {
      frames: provided,
      loading: false,
      error: null,
      reload,
      canReload: false,
    };
  }

  return {
    frames: data ?? [],
    loading,
    error:
      error === null
        ? null
        : error instanceof Error
          ? error.message
          : String(error),
    reload,
    canReload: true,
  };
}
