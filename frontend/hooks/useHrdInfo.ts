/**
 * useHrdInfo
 *
 * Fetches the AV1 decoder model of the open stream (`get_hrd_info`) for the HRD Buffer view and
 * turns it into what `HRDBufferPanel` needs. AV1 signals no bit rate or buffer size: both come
 * from the first operating point's level, tier and profile, so the view has parameters to show
 * only when that operating point has a decoder model *and* a level the spec defines limits for.
 * Everything else (other codecs, no decoder model, an undefined level) gives no parameters and the
 * panel says there is no HRD information.
 */

import { useEffect, useState } from "react";
import {
  getHrdInfo,
  type HrdInfoWireResult,
} from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("useHrdInfo");

/** Frame rate assumed when the stream signals none (a variable display rate). */
export const DEFAULT_FRAME_RATE = 30;

export interface HrdPanelParams {
  /** Bits per second; `undefined` when the stream signals no usable HRD parameters. */
  targetBitrate?: number;
  /** CPB size in bytes. */
  bufferSize?: number;
  frameRate: number;
}

/** The panel's inputs from a decoder-model description. */
export function hrdPanelParams(info: HrdInfoWireResult | null): HrdPanelParams {
  const frameRate = info?.frame_rate ?? DEFAULT_FRAME_RATE;
  const op = info?.operating_points[0];
  if (
    !info?.present ||
    !op?.decoder_model ||
    op.max_bitrate_bps === null ||
    op.buffer_size_bits === null
  ) {
    return { frameRate };
  }
  return {
    targetBitrate: op.max_bitrate_bps,
    bufferSize: op.buffer_size_bits / 8,
    frameRate,
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
