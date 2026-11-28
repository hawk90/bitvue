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
import type { OverlayRenderOptions } from "./types";
import {
  INFO_OVERLAY_RENDERERS,
  MAIN_OVERLAY_RENDERERS,
  withSavedContext,
  type OverlayRenderExtras,
} from "./registry";

// ─── Extended options ─────────────────────────────────────────────────────────

export interface OverlayRenderOptionsExtended extends OverlayRenderOptions {
  /** Additional info overlays to layer on top of the main mode overlay. */
  activeOverlays?: ReadonlySet<VisualizationMode>;
  /** Optional WebGL2 overlay canvas for high-density MV rendering. */
  webglCanvas?: HTMLCanvasElement;
}

// ─── Main mode rendering ──────────────────────────────────────────────────────

function renderMainModeOverlay(
  { mode, frame, canvas, ctx, av1Features, dpr = 1 }: OverlayRenderOptions,
  webglCanvas?: HTMLCanvasElement,
): void {
  if (!frame) return;

  // Logical (frame-native) size -- canvas.width/height are the *physical* buffer, scaled by
  // `dpr` (see VideoCanvas.tsx's render effect); ctx's transform is already dpr-scaled too, so
  // every renderer below keeps using plain frame-native coordinates unchanged.
  const width = canvas.width / dpr;
  const height = canvas.height / dpr;

  const spec = MAIN_OVERLAY_RENDERERS[mode];
  if (!spec) return; // yuv / overview / modes with no overlay -- nothing to draw
  const props = { ctx, width, height, frame };
  const extras: OverlayRenderExtras = { av1Features, webglCanvas };
  if (spec.saveRestore) {
    withSavedContext(ctx, () => spec.render(props, extras));
  } else {
    spec.render(props, extras);
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
  webglCanvas?: HTMLCanvasElement,
): void {
  withSavedContext(ctx, () => {
    ctx.globalAlpha = INFO_OVERLAY_ALPHA;
    // Modes without a renderer still get the save/alpha/restore wrapper (no drawing).
    INFO_OVERLAY_RENDERERS[mode]?.render(
      { ctx, width, height, frame },
      { webglCanvas },
    );
  });
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Render the main-mode overlay followed by any active info overlays.
 *
 * @param options - Render options including optional `activeOverlays` set and
 *                  optional `av1Features` for AV1-specific modes.
 */
export function renderModeOverlay(options: OverlayRenderOptionsExtended): void {
  const {
    mode,
    frame,
    canvas,
    ctx,
    activeOverlays,
    av1Features,
    webglCanvas,
    dpr = 1,
  } = options;
  if (!frame) return;

  // Logical (frame-native) size -- see renderMainModeOverlay's matching comment.
  const width = canvas.width / dpr;
  const height = canvas.height / dpr;

  // Pass 1 — main mode
  renderMainModeOverlay(
    { mode, frame, canvas, ctx, av1Features, dpr },
    webglCanvas,
  );

  // Pass 2 — info overlays (only when a set is provided and non-empty)
  if (activeOverlays && activeOverlays.size > 0) {
    for (const overlayMode of activeOverlays) {
      // Skip if this overlay is the same as the main mode (would double-render)
      if (overlayMode !== mode) {
        renderInfoOverlay(overlayMode, frame, ctx, width, height, webglCanvas);
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
