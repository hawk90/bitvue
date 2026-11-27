import { QmTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// QmTab takes its data as a `data` prop (the sidecar has no source
// for it yet), so the stories pass realistic data directly.
const qmData = {
  scaling_list_enabled: true,
  matrices: [
    {
      name: "Intra 4x4 Luma",
      size: 4,
      pred_type: "Intra",
      plane: "Y",
      values: [16, 16, 17, 21, 16, 17, 20, 25, 17, 20, 25, 30, 21, 25, 30, 36],
    },
    {
      name: "Inter 4x4 Luma",
      size: 4,
      pred_type: "Inter",
      plane: "Y",
      values: new Array(16).fill(16),
    },
    {
      name: "Intra 8x8 Chroma Cb",
      size: 8,
      pred_type: "Intra",
      plane: "Cb",
      values: new Array(64).fill(16),
    },
  ],
};

export const WithScalingLists = () => (
  <div style={previewBg}>
    <QmTab filePath="/Users/hawk/clips/clip_hevc.mp4" data={qmData} />
  </div>
);

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <QmTab />
  </div>
);
