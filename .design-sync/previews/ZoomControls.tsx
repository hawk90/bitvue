import { ZoomControls } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Default100 = () => (
  <div style={previewBg}>
    <ZoomControls
      zoom={1}
      onZoomIn={() => {}}
      onZoomOut={() => {}}
      onResetZoom={() => {}}
    />
  </div>
);

export const ZoomedIn250 = () => (
  <div style={previewBg}>
    <ZoomControls
      zoom={2.5}
      onZoomIn={() => {}}
      onZoomOut={() => {}}
      onResetZoom={() => {}}
    />
  </div>
);
