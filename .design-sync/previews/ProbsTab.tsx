import { ProbsTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// ProbsTab takes its data as a `data` prop (the sidecar has no source
// for it yet), so the stories pass realistic data directly.
const probsData = {
  frame_index: 42,
  is_key_frame: false,
  entries: [
    { group: "Coefficient", label: "coef_probs[TX_4X4][Y][0]", probs: [214, 158, 92] },
    { group: "Coefficient", label: "coef_probs[TX_8X8][UV][1]", probs: [201, 133, 77] },
    { group: "Mode", label: "y_mode_probs", probs: [128, 96, 172, 64] },
    { group: "Mode", label: "partition_probs", probs: [199, 122, 41] },
    { group: "MV", label: "mv_joint_probs", probs: [32, 64, 96] },
  ],
};

export const WithProbabilityData = () => (
  <div style={previewBg}>
    <ProbsTab filePath="/Users/hawk/clips/clip_vp9.webm" data={probsData} />
  </div>
);

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <ProbsTab />
  </div>
);
