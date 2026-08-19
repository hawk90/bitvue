import { FilmstripTooltip } from "bitvue";
import type { CSSProperties } from "react";

// FilmstripTooltip is `position: fixed` (viewport-relative x/y, see its own CSS doc) -- a
// `transform` on this wrapper makes it the containing block for fixed descendants, so the
// tooltip renders inside the visible card instead of floating off to the real viewport corner.
const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 480,
  height: 260,
};

export const RightPlacement = () => (
  <div style={previewBg}>
    <FilmstripTooltip
      frame={{
        frame_index: 142,
        frame_type: "B",
        size: 24680,
        pts: 4686,
        poc: 142,
        temporal_id: 2,
        ref_frames: [140, 144],
      }}
      x={120}
      y={140}
      placement="right"
    />
  </div>
);

export const LeftPlacement = () => (
  <div style={previewBg}>
    <FilmstripTooltip
      frame={{
        frame_index: 8,
        frame_type: "I",
        size: 178432,
        pts: 264,
        poc: 8,
        temporal_id: 0,
        ref_frames: [],
      }}
      x={360}
      y={140}
      placement="left"
    />
  </div>
);

export const MinimalFields = () => (
  <div style={previewBg}>
    <FilmstripTooltip
      frame={{
        frame_index: 57,
        frame_type: "P",
        size: 61204,
      }}
      x={200}
      y={140}
      placement="right"
    />
  </div>
);
