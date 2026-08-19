import { DetailsPanel, type FrameDetails } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

const withRefs: FrameDetails = {
  temporal_id: 1,
  display_order: 87,
  coding_order: 84,
  ref_frames: [83, 79, 76],
};

export const WithReferences = () => (
  <div style={previewBg}>
    <DetailsPanel frame={withRefs} />
  </div>
);

export const NoFrame = () => (
  <div style={previewBg}>
    <DetailsPanel frame={null} />
  </div>
);
