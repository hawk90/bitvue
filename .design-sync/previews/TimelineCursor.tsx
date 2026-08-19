import { TimelineCursor } from "bitvue";
import type { CSSProperties } from "react";

// TimelineCursor is `position: absolute; top:0; bottom:0` -- it needs a sized, positioned
// ancestor to be visible against, so the wrapper mocks up a strip of timeline bars behind it
// (same idea as the real `.timeline-thumbnails` track it's normally placed inside).
const track: CSSProperties = {
  position: "relative",
  height: 96,
  width: 480,
  background:
    "repeating-linear-gradient(90deg, #3a3a3a 0px, #3a3a3a 3px, #2a2a2a 3px, #2a2a2a 6px)",
  borderRadius: 4,
  overflow: "hidden",
};

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const EarlyFrame = () => (
  <div style={previewBg}>
    <div style={track}>
      <TimelineCursor positionPx={64} frameIndex={12} />
    </div>
  </div>
);

export const LateFrame = () => (
  <div style={previewBg}>
    <div style={track}>
      <TimelineCursor positionPx={360} frameIndex={204} />
    </div>
  </div>
);
