import { PanelEmptyState } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Statistics = () => (
  <div style={previewBg}>
    <PanelEmptyState panelName="Statistics" />
  </div>
);

export const Bookmarks = () => (
  <div style={previewBg}>
    <PanelEmptyState panelName="Bookmarks" />
  </div>
);
