import { ExportDialog } from "bitvue";
import type { CSSProperties } from "react";

// `.export-dialog-overlay` is `position: fixed` -- give the wrapper a containing block
// (transform) plus a concrete box so it renders inside the visible card instead of escaping it.
const dialogWrapper: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 560,
  height: 560,
};

const frames = [
  { frame_index: 0, frame_type: "I", size: 48213, poc: 0, pts: 0, temporal_id: 0 },
  { frame_index: 1, frame_type: "P", size: 21044, poc: 4, pts: 4, temporal_id: 1 },
  { frame_index: 2, frame_type: "B", size: 9887, poc: 2, pts: 2, temporal_id: 2 },
  { frame_index: 3, frame_type: "B", size: 8120, poc: 1, pts: 1, temporal_id: 2 },
  { frame_index: 4, frame_type: "B", size: 8654, poc: 3, pts: 3, temporal_id: 2 },
];

export const FrameData = () => (
  <div style={dialogWrapper}>
    <ExportDialog
      isOpen={true}
      onClose={() => {}}
      frames={frames}
      codec="AV1"
      width={1920}
      height={1080}
    />
  </div>
);

export const AnalysisReport = () => {
  const manyFrames = Array.from({ length: 250 }, (_, i) => ({
    frame_index: i,
    frame_type: i % 8 === 0 ? "I" : i % 3 === 0 ? "P" : "B",
    size: 8000 + ((i * 137) % 40000),
    poc: i,
    pts: i,
    temporal_id: i % 3,
  }));
  return (
    <div style={dialogWrapper}>
      <ExportDialog
        isOpen={true}
        onClose={() => {}}
        frames={manyFrames}
        codec="HEVC"
        width={3840}
        height={2160}
      />
    </div>
  );
};

export const NoFramesLoaded = () => (
  <div style={dialogWrapper}>
    <ExportDialog isOpen={true} onClose={() => {}} frames={[]} />
  </div>
);
