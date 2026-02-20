/**
 * Helper Utilities for Overlay Rendering
 *
 * General utility functions
 */

import type { ColorStop } from "../types";
import { QP_COLOR_STOPS } from "./colors";

/**
 * Linear interpolation between two values
 */
export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

/**
 * Map QP value to color using 4-stop RGB ramp
 *
 * @param qp - QP value to map
 * @param minQP - Minimum QP for normalization
 * @param maxQP - Maximum QP for normalization
 * @returns CSS color string with alpha
 */
export function qpToColor(qp: number, minQP: number, maxQP: number): string {
  // Normalize QP to 0-1 range
  const t = Math.max(0, Math.min(1, (qp - minQP) / (maxQP - minQP || 1)));

  // Find color stops to interpolate between
  for (let i = 0; i < QP_COLOR_STOPS.length - 1; i++) {
    const stop1 = QP_COLOR_STOPS[i];
    const stop2 = QP_COLOR_STOPS[i + 1];

    if (t >= stop1.t && t <= stop2.t) {
      // Interpolate between stops
      const localT = (t - stop1.t) / (stop2.t - stop1.t);
      const r = Math.round(lerp(stop1.r, stop2.r, localT));
      const g = Math.round(lerp(stop1.g, stop2.g, localT));
      const b = Math.round(lerp(stop1.b, stop2.b, localT));
      const alpha = 0.63; // Base alpha (160/255)

      return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }
  }

  // Fallback (shouldn't reach here)
  return "rgba(0, 70, 255, 0.63)";
}
