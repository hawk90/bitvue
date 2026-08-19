import { BPyramidView } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  overflowX: "auto",
};

// Same AV1 hierarchical B-pyramid fixture as BPyramidTimeline.tsx (BPyramidView is a thin wrapper
// that calls `analyzeTemporalLevels` internally, so it only needs the raw `frames` prop).
interface PreviewFrame {
  frame_index: number;
  frame_type: string;
  size: number;
  pts: number;
  poc: number;
  display_order: number;
  coding_order: number;
  key_frame: boolean;
  temporal_id: number;
  ref_frames: number[];
}

function buildBPyramidFrames(gopCount = 4): PreviewFrame[] {
  const gopSize = 8;
  type Raw = { frame_index: number; tid: number; refs: number[] };
  const raw: Raw[] = [];

  for (let g = 0; g < gopCount; g++) {
    const base = g * gopSize;
    raw.push({
      frame_index: base,
      tid: 0,
      refs: g === 0 ? [] : [base - gopSize],
    });
    raw.push({ frame_index: base + 4, tid: 1, refs: [base, base + 8] });
    raw.push({ frame_index: base + 2, tid: 2, refs: [base, base + 4] });
    raw.push({ frame_index: base + 6, tid: 2, refs: [base + 4, base + 8] });
    raw.push({ frame_index: base + 1, tid: 3, refs: [base, base + 2] });
    raw.push({ frame_index: base + 3, tid: 3, refs: [base + 2, base + 4] });
    raw.push({ frame_index: base + 5, tid: 3, refs: [base + 4, base + 6] });
    raw.push({ frame_index: base + 7, tid: 3, refs: [base + 6, base + 8] });
  }
  raw.push({
    frame_index: gopCount * gopSize,
    tid: 0,
    refs: [(gopCount - 1) * gopSize],
  });

  return raw
    .sort((a, b) => a.frame_index - b.frame_index)
    .map(({ frame_index, tid, refs }) => {
      const isKey = frame_index === 0;
      const isAnchor = tid === 0;
      const baseSize = isKey
        ? 178000
        : isAnchor
          ? 61000
          : tid === 1
            ? 31000
            : tid === 2
              ? 18000
              : 10500;
      const jitter = 0.85 + 0.3 * Math.abs(Math.sin(frame_index * 12.9898));
      return {
        frame_index,
        frame_type: isKey ? "I" : isAnchor ? "P" : "B",
        size: Math.round(baseSize * jitter),
        pts: frame_index * 33,
        poc: frame_index,
        display_order: frame_index,
        coding_order: frame_index,
        key_frame: isKey,
        temporal_id: tid,
        ref_frames: refs,
      };
    });
}

function getFrameTypeColorClass(frameType: string): string {
  const type = frameType.toUpperCase();
  if (type === "I" || type === "KEY" || type === "INTRA") return "frame-i";
  if (type === "P" || type === "INTER") return "frame-p";
  if (type === "B") return "frame-b";
  return "frame-unknown";
}

const frames = buildBPyramidFrames(4);

export const Default = () => (
  <div style={previewBg}>
    <BPyramidView
      frames={frames}
      currentFrameIndex={5}
      onFrameClick={() => {}}
      getFrameTypeColorClass={getFrameTypeColorClass}
    />
  </div>
);

export const StructureView = () => (
  <div style={previewBg}>
    <BPyramidView
      frames={frames}
      currentFrameIndex={12}
      onFrameClick={() => {}}
      getFrameTypeColorClass={getFrameTypeColorClass}
      showAllArrows
    />
  </div>
);

export const Empty = () => (
  <div style={previewBg}>
    <BPyramidView
      frames={[]}
      currentFrameIndex={0}
      onFrameClick={() => {}}
      getFrameTypeColorClass={getFrameTypeColorClass}
    />
  </div>
);
