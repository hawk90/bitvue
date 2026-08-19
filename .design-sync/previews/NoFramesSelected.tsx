import { NoFramesSelected } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Default = () => (
  <div style={previewBg}>
    <NoFramesSelected />
  </div>
);
