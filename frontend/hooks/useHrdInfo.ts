/**
 * useHrdInfo
 *
 * Fetches the AV1 decoder model of the open stream (`get_hrd_info`) for the HRD Buffer view. The
 * sidecar runs the model of spec Annex E over the whole stream (arrival and removal times, the
 * smoothing buffer's occupancy, the conformance violations) when the stream signals timing it can
 * run on; every other stream (other codecs, no decoder model, an undefined level) has no model and
 * the panel says there is no HRD information.
 */

import { useEffect, useState } from "react";
import {
  getHrdInfo,
  type HrdInfoWireResult,
  type HrdModelWire,
} from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("useHrdInfo");

export interface HrdPanelModel {
  /** `null` when the stream has no decoder model to run. */
  model: HrdModelWire | null;
  /** Why there is none, when the sidecar said. */
  unavailableReason: string | null;
}

/** The panel's inputs from a decoder-model description. */
export function hrdPanelModel(info: HrdInfoWireResult | null): HrdPanelModel {
  return {
    model: info?.model ?? null,
    unavailableReason: info?.model_unavailable ?? null,
  };
}

/**
 * `enabled` gates the request (only the HRD view needs it); `streamKey` changes when another
 * stream is open so the answer is fetched again.
 */
export function useHrdInfo(
  enabled: boolean,
  streamKey: string | number,
): HrdInfoWireResult | null {
  const [info, setInfo] = useState<HrdInfoWireResult | null>(null);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    getHrdInfo()
      .then((result) => {
        if (!cancelled) setInfo(result);
      })
      .catch((error: unknown) => {
        // not AV1, or no stream: the view shows "no HRD information"
        logger.debug("no HRD info:", error);
        if (!cancelled) setInfo(null);
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, streamKey]);

  return info;
}
