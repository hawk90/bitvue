/**
 * Overlay Renderer - Main Entry Point
 *
 * Delegates to mode-specific overlay renderers
 */

import type { OverlayRenderOptions } from "./types";
import { CodingFlowOverlay } from "./renderers/CodingFlowRenderer";
import { PredictionOverlay } from "./renderers/PredictionRenderer";
import { TransformOverlay } from "./renderers/TransformRenderer";
import { QPMapOverlay } from "./renderers/QPMapRenderer";
import { MVFieldOverlay } from "./renderers/MVFieldRenderer";
import { ReferenceOverlay } from "./renderers/ReferenceRenderer";

/**
 * Render mode-specific overlay on the canvas
 * Delegates to the appropriate mode-specific renderer
 */
export function renderModeOverlay({
  mode,
  frame,
  canvas,
  ctx,
}: OverlayRenderOptions): void {
  if (!frame) return;

  const width = canvas.width;
  const height = canvas.height;

  // Clear existing overlay (by redrawing base frame)
  // Note: The base frame should already be drawn by the parent component

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
    case "qp-map":
      QPMapOverlay({ ctx, width, height, frame });
      break;
    case "mv-field":
      MVFieldOverlay({ ctx, width, height, frame });
      break;
    case "reference":
      ReferenceOverlay({ ctx, width, height, frame });
      break;
    case "overview":
    default:
      // Overview mode - no overlay
      break;
  }
}

// Re-export types for convenience
export type {
  OverlayRenderOptions,
  OverlayRendererProps,
  LegendItem,
  ColorStop,
} from "./types";
