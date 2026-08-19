import { TimelineThumbnails } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

interface PreviewFrame {
  frame_index: number;
  frame_type: string;
  size: number;
  pts: number;
  poc: number;
  key_frame: boolean;
  temporal_id: number;
}

function buildFrames(count = 60): PreviewFrame[] {
  const frames: PreviewFrame[] = [];
  for (let i = 0; i < count; i++) {
    const posInGop = i % 8;
    const isKey = posInGop === 0;
    const frame_type = isKey ? "I" : posInGop === 4 ? "P" : "B";
    frames.push({
      frame_index: i,
      frame_type,
      size: isKey ? 165000 : frame_type === "P" ? 58000 : 21000,
      pts: i * 33,
      poc: i,
      key_frame: isKey,
      temporal_id: isKey ? 0 : frame_type === "P" ? 0 : 2,
    });
  }
  return frames;
}

const frames = buildFrames(60);

export const Default = () => (
  <div style={previewBg}>
    <TimelineThumbnails
      frames={frames}
      highlightedFrameIndex={22}
      cursorPositionPx={198}
      onMouseDown={() => {}}
      onMouseMove={() => {}}
      onMouseLeave={() => {}}
      onKeyDown={() => {}}
    />
  </div>
);

export const AtStart = () => (
  <div style={previewBg}>
    <TimelineThumbnails
      frames={frames}
      highlightedFrameIndex={0}
      cursorPositionPx={6}
      onMouseDown={() => {}}
      onMouseMove={() => {}}
      onMouseLeave={() => {}}
      onKeyDown={() => {}}
    />
  </div>
);
