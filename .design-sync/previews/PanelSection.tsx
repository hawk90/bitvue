import { PanelSection } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };
const row: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  padding: "4px 0",
  fontSize: 12,
  color: "rgba(255,255,255,0.85)",
};

export const WithTitle = () => (
  <div style={previewBg}>
    <PanelSection title="Stream Statistics">
      <div style={row}>
        <span>Codec</span>
        <span>AV1</span>
      </div>
      <div style={row}>
        <span>Resolution</span>
        <span>1920x1080</span>
      </div>
      <div style={row}>
        <span>Frame Rate</span>
        <span>29.97 fps</span>
      </div>
    </PanelSection>
  </div>
);

export const WithoutTitle = () => (
  <div style={previewBg}>
    <PanelSection>
      <div style={row}>
        <span>Total Frames</span>
        <span>1,482</span>
      </div>
      <div style={row}>
        <span>Bitrate</span>
        <span>4.2 Mbps</span>
      </div>
    </PanelSection>
  </div>
);
