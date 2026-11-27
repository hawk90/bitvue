/**
 * X axis of the Bitrate Graph: which order the bars are drawn in and what the ticks say.
 *
 * Bars used to be drawn in decode order with no axis at all, so a stream with B-frames drew its
 * frames out of time order. Here frames are laid out by PTS (display order) when every frame has a
 * distinct PTS, and stay in decode order otherwise (a missing or repeated PTS has no honest sort).
 * Tick labels are frame numbers; seconds are added only when a frame rate is actually known --
 * PTS units are the container's timebase, which the renderer is not told, so seconds are never
 * guessed from PTS.
 */

import type { GraphFrame } from "../services/electronBridgeService";

export type AxisOrder = "display" | "decode";

export interface BitrateAxis {
  /** `frames` in the order the bars are drawn. */
  frames: GraphFrame[];
  order: AxisOrder;
}

export function orderFramesForAxis(frames: GraphFrame[]): BitrateAxis {
  const seen = new Set<number>();
  for (const f of frames) {
    if (f.pts === null || seen.has(f.pts)) {
      return { frames, order: "decode" };
    }
    seen.add(f.pts);
  }
  const sorted = [...frames].sort(
    (a, b) => (a.pts as number) - (b.pts as number),
  );
  return { frames: sorted, order: "display" };
}

/** Bar positions (0-based, ascending, unique) that get a tick label: first, last and evenly
 *  spaced in between. At most `maxTicks`; fewer for short streams. */
export function axisTickPositions(count: number, maxTicks = 5): number[] {
  if (count <= 0) return [];
  if (count === 1) return [0];
  const n = Math.min(maxTicks, count);
  const out = new Set<number>();
  for (let k = 0; k < n; k++) {
    out.add(Math.round((k * (count - 1)) / (n - 1)));
  }
  return [...out];
}

/** Label under a tick: the bar's position in the drawn order, or the time when `frameRate`
 *  (frames per second) is known and positive. */
export function formatAxisTick(
  position: number,
  frameRate: number | null | undefined,
): string {
  if (frameRate === null || frameRate === undefined || !(frameRate > 0)) {
    return `#${position}`;
  }
  return `${(position / frameRate).toFixed(2)}s`;
}
