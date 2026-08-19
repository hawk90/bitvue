import { PanelLoading } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Default = () => (
  <div style={previewBg}>
    <PanelLoading />
  </div>
);

export const CustomMessage = () => (
  <div style={previewBg}>
    <PanelLoading message="Decoding frame 142..." />
  </div>
);
