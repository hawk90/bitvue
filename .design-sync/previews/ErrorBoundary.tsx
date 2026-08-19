import { ErrorBoundary } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

/** Throws unconditionally during render so ErrorBoundary's getDerivedStateFromError/
 * componentDidCatch lifecycle actually fires and the fallback UI renders -- there's no prop-only
 * way to force a class error boundary into its error state. */
function ThrowsDuringRender(): JSX.Element {
  throw new Error("Failed to parse OBU: unexpected tile_group_obu length");
}

export const NormalChildren = () => (
  <div style={previewBg}>
    <ErrorBoundary>
      <div
        style={{
          background: "#2d2d2d",
          border: "1px solid rgba(255,255,255,0.1)",
          borderRadius: 4,
          padding: 16,
          color: "rgba(255,255,255,0.85)",
          fontSize: 12,
        }}
      >
        <strong>sample.ivf</strong> — AV1, 1920x1080, 250 frames decoded
        successfully.
      </div>
    </ErrorBoundary>
  </div>
);

const fixedContainerBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 640,
  height: 400,
};

export const CaughtErrorFallback = () => (
  <div style={fixedContainerBg}>
    <ErrorBoundary>
      <ThrowsDuringRender />
    </ErrorBoundary>
  </div>
);
