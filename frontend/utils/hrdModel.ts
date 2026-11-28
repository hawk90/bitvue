/**
 * Helpers for the HRD Buffer view over the sidecar's decoder model run (`get_hrd_info`'s `model`,
 * spec Annex E): where the smoothing buffer stood at a frame's removal and which conformance
 * requirements a frame breaks.
 */

import type {
  HrdModelFrameWire,
  HrdModelWire,
  HrdViolationKind,
  HrdViolationWire,
} from "../services/electronBridgeService";

/** What the user is told about each requirement the model checks. */
export const VIOLATION_LABELS: Record<HrdViolationKind, string> = {
  smoothing_buffer_underflow: "Underflow: frame not complete at its removal",
  smoothing_buffer_overflow: "Overflow: more than BufferSize in the buffer",
  decoder_buffer_delay_consistency:
    "decoder_buffer_delay longer than the gap before this key frame",
  decoder_buffer_delay_level:
    "decoder_buffer_delay is 0 or longer than BufferSize / BitRate",
  decode_buffer_available_late:
    "No free frame buffer before its presentation, at the timing the stream signals",
  decode_frame_buf_unavailable:
    "All frame buffers in use, at the timing the stream signals",
  decode_existing_frame_buf_empty: "Shown frame slot is empty",
  display_frame_late:
    "Decoded after its presentation time, at the timing the stream signals",
};

/**
 * The smoothing buffer's occupancy (bits) just before `time`: the curve is piecewise linear, and a
 * removal is two points at one time, so at a removal this is the level before the frame leaves.
 */
export function occupancyBefore(
  curve: readonly [number, number][],
  time: number,
): number {
  if (curve.length === 0) return 0;
  const next = curve.findIndex(([t]) => t >= time);
  if (next === -1) return curve[curve.length - 1][1];
  if (next === 0) return curve[0][1];
  const [t1, bits1] = curve[next];
  if (t1 === time) return bits1;
  const [t0, bits0] = curve[next - 1];
  return bits0 + ((bits1 - bits0) * (time - t0)) / (t1 - t0);
}

/** The decoded frame of a packet that the frame list shows: the last one, which is the shown one
 *  when a packet holds hidden frames before it. */
export function frameOfPacket(
  frames: readonly HrdModelFrameWire[],
  packet: number,
): HrdModelFrameWire | undefined {
  let found: HrdModelFrameWire | undefined;
  for (const frame of frames) {
    if (frame.packet === packet) found = frame;
  }
  return found;
}

/** The decoded frame removed nearest to `time`. */
export function nearestFrame(
  frames: readonly HrdModelFrameWire[],
  time: number,
): HrdModelFrameWire | undefined {
  let best: HrdModelFrameWire | undefined;
  for (const frame of frames) {
    if (
      !best ||
      Math.abs(frame.removal - time) < Math.abs(best.removal - time)
    ) {
      best = frame;
    }
  }
  return best;
}

export interface ViolationCounts {
  overflow: number;
  underflow: number;
  /** Every other requirement. */
  other: number;
}

export function countViolations(
  violations: readonly HrdViolationWire[],
): ViolationCounts {
  const counts = { overflow: 0, underflow: 0, other: 0 };
  for (const { kind } of violations) {
    if (kind === "smoothing_buffer_overflow") counts.overflow++;
    else if (kind === "smoothing_buffer_underflow") counts.underflow++;
    else counts.other++;
  }
  return counts;
}

/** The violations found at one decodable frame group. */
export function violationsAt(
  model: HrdModelWire,
  dfg: number,
): HrdViolationWire[] {
  return model.violations.filter((v) => v.dfg === dfg);
}

/** Seconds on the time axis: milliseconds under a second, else seconds. */
export function formatSeconds(seconds: number): string {
  return Math.abs(seconds) < 1
    ? `${(seconds * 1000).toFixed(1)} ms`
    : `${seconds.toFixed(2)} s`;
}
