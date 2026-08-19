import { MemoizedTimeline } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// `Timeline.tsx`'s default/`memo` export is unnamed at the module level -- the named export
// consumers actually use is `MemoizedTimeline` (`export const MemoizedTimeline = memo(Timeline,
// ...)`), per the task brief. It reads `SelectionContext` (auto-wrapped by the DS harness) and
// only calls `electronBridgeService.getContextMenuItems`/`useExportEvidenceBundle`'s bridge calls
// on right-click/export-click, never on mount, so no `window.bitvue` mocking is needed for a
// static render.
interface PreviewFrame {
  frame_index: number;
  frame_type: string;
  size: number;
  pts: number;
  poc: number;
  key_frame: boolean;
  ref_frames: number[];
}

function buildFrames(count = 60): PreviewFrame[] {
  const frames: PreviewFrame[] = [];
  for (let i = 0; i < count; i++) {
    const posInGop = i % 8;
    const isKey = posInGop === 0;
    let frame_type: string;
    let size: number;
    let ref_frames: number[];
    if (isKey) {
      frame_type = "I";
      size = 165000 + Math.round(15000 * Math.abs(Math.sin(i)));
      ref_frames = i === 0 ? [] : [i - 8];
    } else if (posInGop === 4) {
      frame_type = "P";
      size = 60000 + Math.round(12000 * Math.abs(Math.sin(i * 1.7)));
      ref_frames = [i - 4];
    } else {
      frame_type = "B";
      size = 22000 + Math.round(9000 * Math.abs(Math.sin(i * 2.9)));
      ref_frames = [i - 1, i + 1].filter((r) => r >= 0 && r < count);
    }
    frames.push({
      frame_index: i,
      frame_type,
      size,
      pts: i * 33,
      poc: i,
      key_frame: isKey,
      ref_frames,
    });
  }
  return frames;
}

const frames = buildFrames(60);

export const Default = () => (
  <div style={previewBg}>
    <MemoizedTimeline frames={frames} />
  </div>
);

export const Empty = () => (
  <div style={previewBg}>
    <MemoizedTimeline frames={[]} />
  </div>
);
