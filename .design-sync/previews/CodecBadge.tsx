import { CodecBadge } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const AV1 = () => (
  <div style={previewBg}>
    <CodecBadge codec="AV1" />
  </div>
);

export const HEVC = () => (
  <div style={previewBg}>
    <CodecBadge codec="HEVC" />
  </div>
);

export const VP9 = () => (
  <div style={previewBg}>
    <CodecBadge codec="VP9" />
  </div>
);
