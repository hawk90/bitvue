import { VirtualizedFilmstrip } from "bitvue";
import { useState, type CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

interface PreviewFrame {
  frame_index: number;
  frame_type: string;
  size: number;
}

// Large frame count so the windowing/virtualization behavior actually has something to virtualize
// (only itemWidth/containerWidth-visible frames + overscan get rendered as DOM nodes).
function buildFrames(count: number): PreviewFrame[] {
  const frames: PreviewFrame[] = [];
  for (let i = 0; i < count; i++) {
    const posInGop = i % 8;
    const frame_type = posInGop === 0 ? "I" : posInGop === 4 ? "P" : "B";
    frames.push({
      frame_index: i,
      frame_type,
      size: frame_type === "I" ? 165000 : frame_type === "P" ? 58000 : 21000,
    });
  }
  return frames;
}

const frames = buildFrames(600);

export const Default = () => {
  const [currentFrameIndex, setCurrentFrameIndex] = useState(220);
  return (
    <div style={previewBg}>
      <VirtualizedFilmstrip
        frames={frames}
        currentFrameIndex={currentFrameIndex}
        onFrameChange={setCurrentFrameIndex}
        itemWidth={56}
        containerWidth={640}
      />
    </div>
  );
};

export const AtStart = () => {
  const [currentFrameIndex, setCurrentFrameIndex] = useState(0);
  return (
    <div style={previewBg}>
      <VirtualizedFilmstrip
        frames={frames}
        currentFrameIndex={currentFrameIndex}
        onFrameChange={setCurrentFrameIndex}
        itemWidth={56}
        containerWidth={640}
        overscan={2}
      />
    </div>
  );
};
