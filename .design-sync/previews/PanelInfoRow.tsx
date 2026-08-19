import { PanelInfoRow } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Codec = () => (
  <div style={previewBg}>
    <PanelInfoRow label="Codec" value="AV1" />
  </div>
);

export const Resolution = () => (
  <div style={previewBg}>
    <PanelInfoRow label="Resolution" value="1920x1080" />
  </div>
);

export const FrameRate = () => (
  <div style={previewBg}>
    <PanelInfoRow label="Frame Rate" value="29.97 fps" />
  </div>
);
