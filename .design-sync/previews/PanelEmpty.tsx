import { PanelEmpty } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const NoReferenceFrames = () => (
  <div style={previewBg}>
    <PanelEmpty icon="codicon-references" message="No reference frames" />
  </div>
);

export const NoBookmarks = () => (
  <div style={previewBg}>
    <PanelEmpty icon="codicon-bookmark" message="No bookmarks yet" />
  </div>
);
