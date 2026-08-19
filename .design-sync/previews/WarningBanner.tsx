import { WarningBanner } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  display: "flex",
  flexDirection: "column",
  gap: 12,
};

export const Warning = () => (
  <div style={previewBg}>
    <WarningBanner
      message="This file may be truncated — the last frame's data ends before the expected size."
      onDismiss={() => {}}
    />
  </div>
);

export const Info = () => (
  <div style={previewBg}>
    <WarningBanner
      severity="info"
      message="Unsupported profile detected — falling back to baseline parsing for this stream."
      onDismiss={() => {}}
    />
  </div>
);
