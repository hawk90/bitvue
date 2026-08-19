import { MinimapView } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

interface PreviewFrame {
  frame_index: number;
  frame_type: string;
  size: number;
}

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

// Local copies of `frontend/types/video.ts`'s helpers -- that module isn't under
// `frontend/components/` so it's outside the synth-entry "bitvue" package (see
// `.design-sync/NOTES.md`); MinimapView's props expect plain functions, so local equivalents
// (matching the real switch logic) are the correct stand-in.
function getFrameTypeColorClass(frameType: string): string {
  const type = frameType.toUpperCase();
  if (type === "I" || type === "KEY" || type === "INTRA") return "frame-i";
  if (type === "P" || type === "INTER") return "frame-p";
  if (type === "B") return "frame-b";
  return "frame-unknown";
}

function getFrameTypeColor(frameType: string): string {
  const type = frameType.toLowerCase();
  if (type === "i" || type === "key") return "var(--frame-i)";
  if (type === "p" || type === "inter") return "var(--frame-p)";
  if (type.startsWith("b")) return "var(--frame-b)";
  return "var(--text-secondary)";
}

const manyFrames = buildFrames(240);
const fewFrames = buildFrames(24);

export const Default = () => (
  <div style={previewBg}>
    <MinimapView
      frames={manyFrames}
      currentFrameIndex={112}
      onFrameClick={() => {}}
      getFrameTypeColorClass={getFrameTypeColorClass}
      getFrameTypeColor={getFrameTypeColor}
    />
  </div>
);

export const ShortClip = () => (
  <div style={previewBg}>
    <MinimapView
      frames={fewFrames}
      currentFrameIndex={4}
      onFrameClick={() => {}}
      getFrameTypeColorClass={getFrameTypeColorClass}
      getFrameTypeColor={getFrameTypeColor}
    />
  </div>
);
