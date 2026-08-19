import { EmptyState } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Minimal = () => (
  <div style={previewBg}>
    <EmptyState title="No file loaded" />
  </div>
);

export const FullWithAction = () => (
  <div style={previewBg}>
    <EmptyState
      icon="codicon-search"
      title="No results found"
      description="No matches for &quot;keyframe&quot; in sample.ivf"
      action={{ label: "Clear filters", onClick: () => {} }}
      size="lg"
    />
  </div>
);

export const SmallWithDescription = () => (
  <div style={previewBg}>
    <EmptyState
      icon="codicon-debug-stackframe"
      title="No frame selected"
      description="Select a frame to view detailed information"
      size="sm"
    />
  </div>
);
