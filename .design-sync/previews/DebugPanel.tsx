import { DebugPanel } from "bitvue";
import type { CSSProperties } from "react";

// DebugPanel's `.debug-panel`/`.debug-toggle` are `position: fixed` (docked to the right edge of
// the app shell) -- give the wrapper a containing block (transform) plus a concrete box so it
// renders inside the visible card instead of escaping to the viewport.
const dialogWrapper: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 420,
  height: 640,
};

// DebugPanel reads `currentFrameIndex` from `useSelection()` (SelectionContext, part of the DS
// harness's auto-wrapped 7-provider chain) -- default selection is null, so `currentFrameIndex`
// falls back to 0. Frame data itself is a real prop (`frames`), not context-driven.
const smallStream = [
  { frame_index: 0, frame_type: "I", size: 48213, poc: 0, pts: 0, key_frame: true, display_order: 0, coding_order: 0, ref_frames: [] },
  { frame_index: 1, frame_type: "P", size: 21044, poc: 4, pts: 4, key_frame: false, display_order: 4, coding_order: 1, ref_frames: [0], ref_slots: [0] },
  { frame_index: 2, frame_type: "B", size: 9887, poc: 2, pts: 2, key_frame: false, display_order: 2, coding_order: 3, ref_frames: [0, 1], ref_slots: [0, 6] },
  { frame_index: 3, frame_type: "B", size: 8120, poc: 1, pts: 1, key_frame: false, display_order: 1, coding_order: 4, ref_frames: [0, 2], ref_slots: [0, 5] },
  { frame_index: 4, frame_type: "B", size: 8654, poc: 3, pts: 3, key_frame: false, display_order: 3, coding_order: 5, ref_frames: [2, 1], ref_slots: [5, 6] },
  { frame_index: 5, frame_type: "P", size: 19230, poc: 8, pts: 8, key_frame: false, display_order: 8, coding_order: 2, ref_frames: [1], ref_slots: [6] },
];

export const Default = () => (
  <div style={dialogWrapper}>
    <DebugPanel frames={smallStream} visible={true} onClose={() => {}} />
  </div>
);

// 60 frames -- exercises the "All Frames (First 50)" truncation note in the component.
export const LargeStream = () => {
  const frames = Array.from({ length: 60 }, (_, i) => {
    const frame_type = i % 8 === 0 ? "I" : i % 3 === 0 ? "P" : "B";
    return {
      frame_index: i,
      frame_type,
      size: frame_type === "I" ? 52000 - i * 20 : frame_type === "P" ? 22000 - i * 10 : 9000 + i * 5,
      poc: i,
      pts: i,
      key_frame: frame_type === "I",
      display_order: i,
      coding_order: i,
      ref_frames: i === 0 ? [] : [Math.max(0, i - 1)],
    };
  });
  return (
    <div style={dialogWrapper}>
      <DebugPanel frames={frames} visible={true} onClose={() => {}} />
    </div>
  );
};
