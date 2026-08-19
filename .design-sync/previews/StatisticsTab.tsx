import { StatisticsTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24, width: 320 };

// The "QP Distribution" section fetches its histogram on-demand (Load button) via
// `getCodecExtendedInfo()` (electronBridgeService -> `window.bitvue.getCodecExtendedInfo`), so the
// bridge must be mocked for that section to produce real content when clicked -- same pattern as
// .design-sync/previews/FrameSyntaxTab.tsx.
function buildQpHistogram(centerQp: number): Array<{ qp: number; count: number }> {
  const hist: Array<{ qp: number; count: number }> = [];
  for (let qp = Math.max(0, centerQp - 12); qp <= Math.min(63, centerQp + 12); qp++) {
    const dist = Math.abs(qp - centerQp);
    const count = Math.round(40 * Math.exp(-(dist * dist) / 30));
    if (count > 0) hist.push({ qp, count });
  }
  return hist;
}

if (typeof window !== "undefined") {
  (window as unknown as { bitvue: Record<string, unknown> }).bitvue = {
    getCodecExtendedInfo: async (frameIndex: number) => ({
      frame_index: frameIndex,
      l0_refs: [
        { list_idx: 0, slot: 0, poc: frameIndex - 1, frame_index: frameIndex - 1, frame_type: "P", long_term: false, weight: null, offset: null },
      ],
      l1_refs: [],
      qp_histogram: buildQpHistogram(28),
    }),
  };
}

// ─── Frame data helpers ────────────────────────────────────────────────────────

type FrameRow = {
  frame_index: number;
  frame_type: string;
  size: number;
  temporal_id?: number;
  display_order?: number;
  coding_order?: number;
  ref_frames?: number[];
};

/** AV1-style GOP: KEY every 30 frames, alternating P/B in between, sizes scaled by type. */
function buildAv1Gop(count: number): FrameRow[] {
  const frames: FrameRow[] = [];
  for (let i = 0; i < count; i++) {
    const isKey = i % 30 === 0;
    const isB = !isKey && i % 3 !== 0;
    const frame_type = isKey ? "KEY" : isB ? "B" : "P";
    const baseSize = isKey ? 42000 : isB ? 6000 : 14000;
    const jitter = ((i * 2654435761) >>> 0) % 4000;
    frames.push({
      frame_index: i,
      frame_type,
      size: baseSize + jitter,
      temporal_id: isB ? 1 : 0,
      display_order: i,
      coding_order: isKey ? i : i - (isB ? 1 : 0),
      ref_frames: isKey ? [] : isB ? [i - 1, i + 1] : [i - 1],
    });
  }
  return frames;
}

/** HEVC-style stream: heavier IDR frames, denser B-frame hierarchy. */
function buildHevcStream(count: number): FrameRow[] {
  const frames: FrameRow[] = [];
  for (let i = 0; i < count; i++) {
    const isIdr = i % 48 === 0;
    const cycle = i % 8;
    const frame_type = isIdr ? "I" : cycle === 4 ? "P" : "B";
    const baseSize = isIdr ? 88000 : frame_type === "P" ? 24000 : 9000;
    const jitter = ((i * 40503) >>> 0) % 5000;
    frames.push({
      frame_index: i,
      frame_type,
      size: baseSize + jitter,
      temporal_id: frame_type === "B" ? (cycle % 4) + 1 : 0,
      display_order: i,
      coding_order: i,
      ref_frames: isIdr ? [] : [Math.max(0, i - 1)],
    });
  }
  return frames;
}

const av1Frames = buildAv1Gop(90);
const hevcFrames = buildHevcStream(64);

export const Av1Stream = () => (
  <div style={previewBg}>
    <StatisticsTab
      currentFrame={av1Frames[31]}
      frames={av1Frames}
      filePath="/Users/hawk/clips/sample.ivf"
      frameIndex={31}
    />
  </div>
);

export const HevcStream = () => (
  <div style={previewBg}>
    <StatisticsTab
      currentFrame={hevcFrames[hevcFrames.length - 5]}
      frames={hevcFrames}
      filePath="/Users/hawk/clips/tears_of_steel_hevc.mp4"
      frameIndex={hevcFrames.length - 5}
    />
  </div>
);

export const NoFilePathLoaded = () => {
  const frames = buildAv1Gop(12);
  return (
    <div style={previewBg}>
      <StatisticsTab currentFrame={frames[0]} frames={frames} />
    </div>
  );
};

export const EmptyStream = () => (
  <div style={previewBg}>
    <StatisticsTab currentFrame={null} frames={[]} />
  </div>
);
