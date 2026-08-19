import { NoResultsFound } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const WithQuery = () => (
  <div style={previewBg}>
    <NoResultsFound query="keyframe" />
  </div>
);

export const WithoutQuery = () => (
  <div style={previewBg}>
    <NoResultsFound />
  </div>
);
