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

import { useBridgeQuery } from "./useBridgeQuery";
import type { Av1FeaturesData } from "../components/panels/OverlayRenderer/types";
import { MAIN_MODES_NEEDING_AV1_FEATURES } from "../components/panels/OverlayRenderer/registry";
import {
  getAv1Features as bridgeGetAv1Features,
  type Av1FeaturesWireResult,
} from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("useAv1Features");

/** Translates the bridge's snake_case wire shape (matching bitvue-sidecar's own field naming
 *  directly) into this hook's camelCase `Av1FeaturesData` -- the two shapes predate each other by
 *  design (see electronBridgeService.ts's wire-type doc comment), so this is the boundary where
 *  they meet. */
function wireToAv1FeaturesData(data: Av1FeaturesWireResult): Av1FeaturesData {
  return {
    frameIndex: data.frame_index,
    cdef: data.cdef
      ? {
          width: data.cdef.width,
          height: data.cdef.height,
          blockSize: data.cdef.block_size,
          blocks: data.cdef.blocks,
          damping: data.cdef.damping,
          yPrimaryStrength: data.cdef.y_primary_strength,
          ySecondaryStrength: data.cdef.y_secondary_strength,
        }
      : undefined,
    loopRestoration: data.loop_restoration
      ? {
          width: data.loop_restoration.width,
          height: data.loop_restoration.height,
          unitSize: data.loop_restoration.unit_size,
          yType: data.loop_restoration.y_type,
          units: data.loop_restoration.units.map((u) => ({
            x: u.x,
            y: u.y,
            size: u.size,
            restorationType: u.restoration_type,
          })),
        }
      : undefined,
    filmGrain: data.film_grain
      ? {
          enabled: data.film_grain.enabled,
          seed: data.film_grain.seed,
          scalingShift: data.film_grain.scaling_shift,
          arCoeffLag: data.film_grain.ar_coeff_lag,
          chromaScalingFromLuma: data.film_grain.chroma_scaling_from_luma,
          overlap: data.film_grain.overlap,
        }
      : undefined,
    superResolution: data.super_resolution
      ? {
          enabled: data.super_resolution.enabled,
          scaleDenominator: data.super_resolution.scale_denominator,
          upscaledWidth: data.super_resolution.upscaled_width,
          upscaledHeight: data.super_resolution.upscaled_height,
        }
      : undefined,
  };
}

/**
 * Modes that require AV1 feature data from the backend: the registry's `needsAv1Features` modes
 * plus the legacy "av1-features" catch-all, which has no renderer of its own.
 */
export const AV1_FEATURE_MODES: ReadonlySet<string> = new Set([
  ...MAIN_MODES_NEEDING_AV1_FEATURES,
  "av1-features",
]);

export function useAv1Features(
  frameIndex: number,
  activeCodec: string | null,
  currentMode: string,
): { av1Features: Av1FeaturesData | null; loading: boolean } {
  // `activeCodec` comes straight from the sidecar's `get_stream_info` (real value is lowercase
  // "av1", not "AV1") -- codecModeRegistry.ts already normalizes via toUpperCase() before
  // comparing; this needs the same normalization, or shouldFetch is always false and this hook's
  // fetch never runs, leaving every AV1-advanced-feature overlay (CDEF/loop-restoration/film-grain
  // /super-res) permanently without data (confirmed via a diagnostic probe: activeCodec was "av1"
  // at every render, so `activeCodec === "AV1"` never matched).
  const shouldFetch =
    activeCodec?.toUpperCase() === "AV1" && AV1_FEATURE_MODES.has(currentMode);

  const { data: av1Features, loading } = useBridgeQuery(
    async () => wireToAv1FeaturesData(await bridgeGetAv1Features(frameIndex)),
    [frameIndex],
    {
      enabled: shouldFetch,
      onError: (err) =>
        logger.warn("Failed to fetch AV1 features for frame", frameIndex, err),
    },
  );

  return { av1Features, loading };
}
