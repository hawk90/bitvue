/**
 * Overlay Renderer — Main Entry Point
 *
 * Two-pass rendering:
 *   Pass 1 — main mode overlay (one of the F-key modes, e.g. Coding Flow)
 *   Pass 2 — info overlay(s) drawn on top (QP Map, Heat Map, MV Heat …)
 *
 * Info overlays are additive and semi-transparent so they can be combined with
 * any main mode without obscuring it completely.
 */

import type { VisualizationMode } from "../../../contexts/ModeContext";
import type { OverlayRenderOptions, Av1FeaturesData } from "./types";
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
import { Av1EfficiencyMapOverlay } from "./renderers/Av1EfficiencyMapRenderer";
import { Av1BlockTypeOverlay } from "./renderers/Av1BlockTypeRenderer";

// ─── Extended options ─────────────────────────────────────────────────────────

export interface OverlayRenderOptionsExtended extends OverlayRenderOptions {
  /** Additional info overlays to layer on top of the main mode overlay. */
  activeOverlays?: ReadonlySet<VisualizationMode>;
}

// ─── Main mode rendering ──────────────────────────────────────────────────────

function renderMainModeOverlay({
  mode,
  frame,
  canvas,
  ctx,
  av1Features,
}: OverlayRenderOptions): void {
  if (!frame) return;

  const width = canvas.width;
  const height = canvas.height;

  switch (mode) {
    case "coding-flow":
      CodingFlowOverlay({ ctx, width, height, frame });
      break;
    case "prediction":
      PredictionOverlay({ ctx, width, height, frame });
      break;
    case "transform":
      TransformOverlay({ ctx, width, height, frame });
      break;
    // Legacy standalone modes — kept so old code that sets these as the
    // current mode still renders something.
    case "qp-map":
      QPMapOverlay({ ctx, width, height, frame });
      break;
    case "mv-field":
      MVFieldOverlay({ ctx, width, height, frame });
      break;
    case "reference":
      ReferenceOverlay({ ctx, width, height, frame });
      break;
    // ── AV1 advanced modes ─────────────────────────────────────────────────
    case "cdef-filter":
      if (av1Features?.cdef) {
        ctx.save();
        Av1CdefRenderer({ ctx, width, height, frame, cdef: av1Features.cdef });
        ctx.restore();
      }
      break;
    case "loop-restoration":
      if (av1Features?.loopRestoration) {
        ctx.save();
        Av1LoopRestorationRenderer({
          ctx,
          width,
          height,
          frame,
          loopRestoration: av1Features.loopRestoration,
        });
        ctx.restore();
      }
      break;
    case "film-grain":
      ctx.save();
      Av1FilmGrainRenderer({
        ctx,
        width,
        height,
        frame,
        filmGrain: av1Features?.filmGrain,
      });
      ctx.restore();
      break;
    case "super-res":
      ctx.save();
      Av1SuperResRenderer({
        ctx,
        width,
        height,
        frame,
        superResolution: av1Features?.superResolution,
      });
      ctx.restore();
      break;
    // ── VVC exclusive modes ────────────────────────────────────────────────
    case "dual-tree":
      ctx.save();
      VvcDualTreeRenderer({ ctx, width, height, frame });
      ctx.restore();
      break;
    case "adaptive-filter":
      ctx.save();
      VvcAdaptiveFilterRenderer({ ctx, width, height, frame });
      ctx.restore();
      break;
    case "inverse-map":
      ctx.save();
      VvcInverseMapRenderer({ ctx, width, height, frame });
      ctx.restore();
      break;
    case "yuv":
    case "overview":
    default:
      // Pure YUV / no overlay — nothing to draw
      break;
  }
}

// ─── Info overlay rendering (semi-transparent, additive) ─────────────────────

/**
 * Opacity multiplier applied to info overlays so they don't fully hide the
 * underlying main-mode rendering.  Each renderer uses globalAlpha internally
 * but we also wrap in a save/restore block for safety.
 */
const INFO_OVERLAY_ALPHA = 0.72;

function renderInfoOverlay(
  mode: VisualizationMode,
  frame: NonNullable<OverlayRenderOptions["frame"]>,
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
): void {
  ctx.save();
  ctx.globalAlpha = INFO_OVERLAY_ALPHA;

  switch (mode) {
    case "qp-map":
      QPMapOverlay({ ctx, width, height, frame });
      break;
    case "mv-field":
    case "heat-map": // heat-map reuses MVField as a magnitude heatmap for now
      MVFieldOverlay({ ctx, width, height, frame });
      break;
    case "block-type":
      Av1BlockTypeOverlay({ ctx, width, height, frame });
      break;
    case "efficiency-map":
      Av1EfficiencyMapOverlay({ ctx, width, height, frame });
      break;
    // Future overlays (stubs — renderers will be added in later phases):
    // case "psnr-overlay": PSNROverlay({ ctx, width, height, frame }); break;
    // case "pu-type":      PuTypeOverlay({ ctx, width, height, frame }); break;
    default:
      break;
  }

  ctx.restore();
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Render the main-mode overlay followed by any active info overlays.
 *
 * @param options - Render options including optional `activeOverlays` set and
 *                  optional `av1Features` for AV1-specific modes.
 */
export function renderModeOverlay(options: OverlayRenderOptionsExtended): void {
  const { mode, frame, canvas, ctx, activeOverlays, av1Features } = options;
  if (!frame) return;

  const width = canvas.width;
  const height = canvas.height;

  // Pass 1 — main mode
  renderMainModeOverlay({ mode, frame, canvas, ctx, av1Features });

  // Pass 2 — info overlays (only when a set is provided and non-empty)
  if (activeOverlays && activeOverlays.size > 0) {
    for (const overlayMode of activeOverlays) {
      // Skip if this overlay is the same as the main mode (would double-render)
      if (overlayMode !== mode) {
        renderInfoOverlay(overlayMode, frame, ctx, width, height);
      }
    }
  }
}

// Re-export types
export type {
  OverlayRenderOptions,
  OverlayRendererProps,
  LegendItem,
  ColorStop,
  Av1FeaturesData,
} from "./types";
