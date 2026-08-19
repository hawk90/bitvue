import { FrameViewTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const WithFrame = () => (
  <div style={previewBg}>
    <FrameViewTab
      frame={{
        frame_index: 86,
        frame_type: "P",
        size: 184220,
        pts: 86,
        temporal_id: 1,
        display_order: 86,
        coding_order: 83,
        ref_frames: [80, 84, 85],
      }}
    />
  </div>
);

export const NoFrame = () => (
  <div style={previewBg}>
    <FrameViewTab frame={null} />
  </div>
);
