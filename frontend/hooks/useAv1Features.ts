/**
 * useAv1Features
 *
 * Fetches AV1-specific feature data (CDEF, Loop Restoration, Film Grain,
 * Super Resolution) for the currently displayed frame.
 *
 * Only issues the IPC call when:
 *   1. The active codec is "AV1"
 *   2. The current visualization mode requires AV1 feature data
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Av1FeaturesData } from "../components/panels/OverlayRenderer/types";
import { createLogger } from "../utils/logger";

const logger = createLogger("useAv1Features");

/** Modes that require AV1 feature data from the backend. */
const AV1_FEATURE_MODES = new Set([
  "cdef-filter",
  "loop-restoration",
  "film-grain",
  "super-res",
  "av1-features",
]);

export function useAv1Features(
  frameIndex: number,
  activeCodec: string | null,
  currentMode: string,
): { av1Features: Av1FeaturesData | null; loading: boolean } {
  const [av1Features, setAv1Features] = useState<Av1FeaturesData | null>(null);
  const [loading, setLoading] = useState(false);

  const shouldFetch =
    activeCodec === "AV1" && AV1_FEATURE_MODES.has(currentMode);

  useEffect(() => {
    if (!shouldFetch) {
      setAv1Features(null);
      return;
    }

    let cancelled = false;

    const fetchFeatures = async () => {
      setLoading(true);
      try {
        const data = await invoke<Av1FeaturesData>("get_av1_features", {
          frameIndex,
        });
        if (!cancelled) {
          setAv1Features(data);
        }
      } catch (err) {
        if (!cancelled) {
          logger.warn(
            "Failed to fetch AV1 features for frame",
            frameIndex,
            err,
          );
          setAv1Features(null);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void fetchFeatures();

    return () => {
      cancelled = true;
    };
  }, [frameIndex, shouldFetch]);

  return { av1Features, loading };
}
