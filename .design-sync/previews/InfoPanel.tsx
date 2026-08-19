import { InfoPanel } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Loaded = () => (
  <div style={previewBg}>
    <InfoPanel
      filePath="/Users/hawk/clips/sample.ivf"
      frameCount={1482}
      currentFrameIndex={86}
      currentFrame={{
        frame_index: 86,
        frame_type: "P",
        size: 184220,
      }}
    />
  </div>
);

export const NoFrameSelected = () => (
  <div style={previewBg}>
    <InfoPanel
      filePath="/Users/hawk/clips/sample.ivf"
      frameCount={1482}
      currentFrameIndex={0}
      currentFrame={null}
    />
  </div>
);
