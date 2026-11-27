import { ApsTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// ApsTab takes its data as a `data` prop (the sidecar has no source
// for it yet), so the stories pass realistic data directly.
const apsData = {
  aps_list: [
    { aps_id: 0, aps_type: "ALF", enabled: true, summary: "Luma+Chroma ALF, 4 filter sets" },
    { aps_id: 1, aps_type: "ALF", enabled: false, summary: "Cross-component ALF, unused this frame" },
    { aps_id: 0, aps_type: "LMCS", enabled: true, summary: "16-piece piecewise linear mapping" },
    { aps_id: 0, aps_type: "SCALING_LIST", enabled: false, summary: "Flat scaling (disabled)" },
  ],
};

export const WithApsEntries = () => (
  <div style={previewBg}>
    <ApsTab filePath="/Users/hawk/clips/clip_vvc.mp4" data={apsData} />
  </div>
);

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <ApsTab />
  </div>
);
