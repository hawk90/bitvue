import { ReferencesTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

const frames = [
  { frame_index: 80, frame_type: "I", pts: 80 },
  { frame_index: 84, frame_type: "P", pts: 84 },
  { frame_index: 85, frame_type: "P", pts: 85 },
];

export const WithReferences = () => (
  <div style={previewBg}>
    <ReferencesTab
      currentFrame={{ frame_index: 86, frame_type: "P", ref_frames: [80, 84, 85] }}
      frames={frames}
    />
  </div>
);

export const NoReferences = () => (
  <div style={previewBg}>
    <ReferencesTab
      currentFrame={{ frame_index: 0, frame_type: "KEY", ref_frames: [] }}
      frames={frames}
    />
  </div>
);
