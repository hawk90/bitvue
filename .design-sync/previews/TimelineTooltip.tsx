import { TimelineTooltip } from "bitvue";
import type { CSSProperties } from "react";

// TimelineTooltip is `position: absolute; bottom: calc(100% + 8px)` -- it renders *above* its
// positioned ancestor's own box, so the wrapper needs `position: relative` plus extra top padding
// (room for the tooltip to appear in) rather than just background/padding.
const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: "80px 24px 24px",
  position: "relative",
  width: 420,
  height: 40,
  boxSizing: "content-box",
};

const barStyle: CSSProperties = {
  position: "relative",
  height: 40,
  background: "#151515",
  borderRadius: 4,
  border: "1px solid rgba(255,255,255,0.08)",
};

export const Default = () => (
  <div style={previewBg}>
    <div style={barStyle}>
      <TimelineTooltip
        frame={{
          frame_index: 214,
          frame_type: "B",
          size: 26840,
        }}
        positionPercent={45}
      />
    </div>
  </div>
);

export const NearRightEdge = () => (
  <div style={previewBg}>
    <div style={barStyle}>
      <TimelineTooltip
        frame={{
          frame_index: 998,
          frame_type: "I",
          size: 181920,
        }}
        positionPercent={88}
      />
    </div>
  </div>
);
