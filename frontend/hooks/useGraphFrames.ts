/**
 * useGraphFrames
 *
 * The open stream's whole frame list for the graph panels (`getAllGraphFrames`), with a `reload`
 * for their Reload/Retry buttons. Empty (`[]`, no error) when nothing is indexed yet.
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
}

export function useGraphFrames(): UseGraphFramesResult {
  const [reloadKey, setReloadKey] = useState(0);
  const { data, loading, error } = useBridgeQuery(
    (signal) => getAllGraphFrames("A", signal),
    [reloadKey],
  );
  const reload = useCallback(() => setReloadKey((k) => k + 1), []);

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
  };
}
