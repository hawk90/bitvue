import { NoFileLoaded } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const WithOpenAction = () => (
  <div style={previewBg}>
    <NoFileLoaded onOpenFile={() => {}} />
  </div>
);

export const WithoutAction = () => (
  <div style={previewBg}>
    <NoFileLoaded />
  </div>
);
