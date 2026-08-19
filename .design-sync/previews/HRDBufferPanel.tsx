import { HRDBufferPanel } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// HRDBufferPanel draws to a <canvas> sized off its container's getBoundingClientRect() via
// ResizeObserver -- needs a concrete height, not just padding, or the graph area collapses to 0.
const panelWrapper: CSSProperties = {
  ...previewBg,
  width: 640,
  height: 220,
};

// Real prop names/shapes confirmed via Filmstrip.tsx's usage: frames: FrameInfo[],
// currentFrameIndex: number, frameRate: number.
function makeFrames(count: number, spikeEvery = 30) {
  return Array.from({ length: count }, (_, i) => ({
    frame_index: i,
    frame_type: i % spikeEvery === 0 ? "I" : i % 3 === 0 ? "P" : "B",
    size: i % spikeEvery === 0 ? 180000 : i % 3 === 0 ? 24000 : 9000,
    pts: i,
    poc: i,
    display_order: i,
    coding_order: i,
  }));
}

export const Default = () => (
  <div style={panelWrapper}>
    <HRDBufferPanel
      frames={makeFrames(150)}
      currentFrameIndex={62}
      frameRate={30}
      targetBitrate={5_000_000}
      bufferSize={1_000_000}
    />
  </div>
);

// No target bitrate set -- HRDBufferPanel falls back to draining the buffer at each frame's own
// size per frame interval (see the component's `drainPerFrame` fallback), and skips the green
// target-bitrate line / Target stat entirely.
export const NoBitrateTarget = () => (
  <div style={panelWrapper}>
    <HRDBufferPanel frames={makeFrames(150, 45)} currentFrameIndex={20} frameRate={24} />
  </div>
);

// Small CPB (256 KB) against large keyframes -- pushes occupancy past the buffer limit line
// repeatedly, exercising the overflow/underflow markers and stat badges.
export const OverflowUnderflow = () => (
  <div style={panelWrapper}>
    <HRDBufferPanel
      frames={makeFrames(120, 12)}
      currentFrameIndex={45}
      frameRate={30}
      targetBitrate={1_500_000}
      bufferSize={256_000}
    />
  </div>
);
