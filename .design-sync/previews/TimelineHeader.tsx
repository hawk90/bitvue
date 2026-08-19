import { TimelineHeader } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const MidStream = () => (
  <div style={previewBg}>
    <TimelineHeader currentFrame={142} totalFrames={300} />
  </div>
);

export const FirstFrame = () => (
  <div style={previewBg}>
    <TimelineHeader currentFrame={0} totalFrames={1482} />
  </div>
);
