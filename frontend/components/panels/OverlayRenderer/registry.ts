/**
 * Overlay renderer registry
 *
 * One table per dispatch pass maps a `VisualizationMode` to the renderer that draws it, and says
 * explicitly whether the dispatcher wraps that renderer in `ctx.save()` / `ctx.restore()`.
 * `index.tsx` is only the lookup; adding a mode means adding one row here.
 *
 * `saveRestore` is not cosmetic: a renderer registered with `saveRestore: false` leaves whatever
 * it set (font, fillStyle, strokeStyle, ...) on the shared context, and the modes that were
 * dispatched that way before the registry still are (pinned by
 * tests/components/OverlayModeDispatch.test.ts). Flip a flag only on purpose.
 *
 * Two tables, not one, because `qp-map` and `mv-field` exist in both passes and are drawn
 * differently in each: the main pass calls the renderer bare, while the info pass wraps every
 * mode in one outer save + `globalAlpha` + restore (see `renderInfoOverlay`).
 */

import type { VisualizationMode } from "../../../contexts/ModeContext";
import type { Av1FeaturesData, OverlayRendererProps } from "./types";
import { getFrameExtra } from "./utils/helpers";
import { CodingFlowOverlay } from "./renderers/CodingFlowRenderer";
import { PredictionOverlay } from "./renderers/PredictionRenderer";
import { TransformOverlay } from "./renderers/TransformRenderer";
import { QPMapOverlay } from "./renderers/QPMapRenderer";
import { MVFieldOverlay } from "./renderers/MVFieldRenderer";
import { ReferenceOverlay } from "./renderers/ReferenceRenderer";
import { Av1CdefRenderer } from "./renderers/Av1CdefRenderer";
import { Av1LoopRestorationRenderer } from "./renderers/Av1LoopRestorationRenderer";
import { Av1SuperResRenderer } from "./renderers/Av1SuperResRenderer";
import { Av1FilmGrainRenderer } from "./renderers/Av1FilmGrainRenderer";
import { VvcDualTreeRenderer } from "./renderers/VvcDualTreeRenderer";
import { VvcAdaptiveFilterRenderer } from "./renderers/VvcAdaptiveFilterRenderer";
import { VvcInverseMapRenderer } from "./renderers/VvcInverseMapRenderer";
import { Av1HeatMapOverlay } from "./renderers/Av1HeatMapRenderer";
import { Av1EfficiencyMapOverlay } from "./renderers/Av1EfficiencyMapRenderer";
import { Av1BlockTypeOverlay } from "./renderers/Av1BlockTypeRenderer";
import { Avs3EsaoRenderer } from "./renderers/Avs3EsaoRenderer";
import { Avs3CcsaoRenderer } from "./renderers/Avs3CcsaoRenderer";
import { JpegXsPrecinctRenderer } from "./renderers/JpegXsPrecinctRenderer";
import { JpegXsDequantRenderer } from "./renderers/JpegXsDequantRenderer";
import { JpegXsTransformRenderer } from "./renderers/JpegXsTransformRenderer";
import { JpegXsMctRenderer } from "./renderers/JpegXsMctRenderer";
import { JpegXsNltRenderer } from "./renderers/JpegXsNltRenderer";
import { Vc3SegmentRenderer } from "./renderers/Vc3SegmentRenderer";
import { AvcMbTypeOverlay } from "./renderers/AvcMbTypeRenderer";
import { AvcRefIdxOverlay } from "./renderers/AvcRefIdxRenderer";

/** Inputs some renderers need beyond `{ ctx, width, height, frame }`. */
export interface OverlayRenderExtras {
  /** AV1 advanced-feature data (CDEF, loop restoration, film grain, super-res). */
  av1Features?: Av1FeaturesData;
  /** Optional WebGL2 overlay canvas for high-density MV rendering. */
  webglCanvas?: HTMLCanvasElement;
}

export interface OverlayRendererSpec {
  render(props: OverlayRendererProps, extras: OverlayRenderExtras): void;
  /** The dispatcher wraps `render` in `ctx.save()` / `ctx.restore()`. */
  saveRestore: boolean;
  /** The mode needs AV1 feature data fetched from the backend (`useAv1Features`). */
  needsAv1Features?: boolean;
}

/** Run `fn` between `ctx.save()` and `ctx.restore()`. */
export function withSavedContext(
  ctx: CanvasRenderingContext2D,
  fn: () => void,
): void {
  ctx.save();
  fn();
  ctx.restore();
}

/** Called bare: whatever the renderer sets on `ctx` stays set. */
function bare(render: OverlayRendererSpec["render"]): OverlayRendererSpec {
  return { render, saveRestore: false };
}

/** Wrapped in `ctx.save()` / `ctx.restore()` by the dispatcher. */
function saved(
  render: OverlayRendererSpec["render"],
  needsAv1Features?: boolean,
): OverlayRendererSpec {
  return needsAv1Features
    ? { render, saveRestore: true, needsAv1Features }
    : { render, saveRestore: true };
}

/**
 * Main-mode pass (one F-key mode). Modes absent from this table draw nothing.
 * Row order only matters for `MAIN_MODES_NEEDING_AV1_FEATURES`, which is derived from it.
 */
export const MAIN_OVERLAY_RENDERERS: Partial<
  Record<VisualizationMode, OverlayRendererSpec>
> = {
  "coding-flow": bare((p) => CodingFlowOverlay(p)),
  prediction: bare((p) => PredictionOverlay(p)),
  // Codec branch: a JPEG XS frame carries `transform_map` (wrapped); everything else uses the
  // generic transform-grid renderer (bare).
  transform: bare((p) => {
    if (getFrameExtra(p.frame, "transform_map")) {
      withSavedContext(p.ctx, () => JpegXsTransformRenderer(p));
    } else {
      TransformOverlay(p);
    }
  }),
  // Legacy standalone modes -- kept so old code that sets these as the current mode still
  // renders something.
  "qp-map": bare((p) => QPMapOverlay(p)),
  "mv-field": bare((p, x) =>
    MVFieldOverlay({ ...p, webglCanvas: x.webglCanvas }),
  ),
  reference: bare((p) => ReferenceOverlay(p)),
  // AV1 advanced modes
  "cdef-filter": saved(
    (p, x) => Av1CdefRenderer({ ...p, cdef: x.av1Features?.cdef }),
    true,
  ),
  "loop-restoration": saved(
    (p, x) =>
      Av1LoopRestorationRenderer({
        ...p,
        loopRestoration: x.av1Features?.loopRestoration,
      }),
    true,
  ),
  "film-grain": saved(
    (p, x) =>
      Av1FilmGrainRenderer({ ...p, filmGrain: x.av1Features?.filmGrain }),
    true,
  ),
  "super-res": saved(
    (p, x) =>
      Av1SuperResRenderer({
        ...p,
        superResolution: x.av1Features?.superResolution,
      }),
    true,
  ),
  // VVC exclusive modes
  "dual-tree": saved((p) => VvcDualTreeRenderer(p)),
  "adaptive-filter": saved((p) => VvcAdaptiveFilterRenderer(p)),
  "inverse-map": saved((p) => VvcInverseMapRenderer(p)),
  // AVS3 exclusive modes
  esao: saved((p) => Avs3EsaoRenderer(p)),
  ccsao: saved((p) => Avs3CcsaoRenderer(p)),
  // JPEG XS exclusive modes
  precinct: saved((p) => JpegXsPrecinctRenderer(p)),
  dequant: saved((p) => JpegXsDequantRenderer(p)),
  mct: saved((p) => JpegXsMctRenderer(p)),
  nlt: saved((p) => JpegXsNltRenderer(p)),
  // VC-3 exclusive modes
  segment: saved((p) => Vc3SegmentRenderer(p)),
};

/**
 * Info-overlay pass (semi-transparent, additive, drawn over the main mode). The dispatcher
 * wraps EVERY info mode -- registered or not -- in one outer save + globalAlpha + restore, so
 * these specs are all `saveRestore: false`: the wrapper is not per-renderer.
 */
export const INFO_OVERLAY_RENDERERS: Partial<
  Record<VisualizationMode, OverlayRendererSpec>
> = {
  "qp-map": bare((p) => QPMapOverlay(p)),
  "mv-field": bare((p, x) =>
    MVFieldOverlay({ ...p, webglCanvas: x.webglCanvas }),
  ),
  // Measured bits per pixel where the sidecar provides them (AV1); other codecs have no bit
  // cost yet and keep showing the MV field as a magnitude heatmap.
  "heat-map": bare((p, x) => {
    if (p.frame.bit_grid) {
      Av1HeatMapOverlay(p);
    } else {
      MVFieldOverlay({ ...p, webglCanvas: x.webglCanvas });
    }
  }),
  "block-type": bare((p) => Av1BlockTypeOverlay(p)),
  "efficiency-map": bare((p) => Av1EfficiencyMapOverlay(p)),
  "mb-type": bare((p) => AvcMbTypeOverlay(p)),
  "reference-indices": bare((p) => AvcRefIdxOverlay(p)),
};

/** Main modes whose renderer needs AV1 feature data, in registry order. */
export const MAIN_MODES_NEEDING_AV1_FEATURES: readonly string[] = (
  Object.entries(MAIN_OVERLAY_RENDERERS) as [string, OverlayRendererSpec][]
)
  .filter(([, spec]) => spec.needsAv1Features)
  .map(([mode]) => mode);
