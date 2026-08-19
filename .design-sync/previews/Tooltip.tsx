import { Tooltip } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 48,
  position: "relative",
  transform: "translateZ(0)",
  width: 360,
  height: 140,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
};

const triggerStyle: CSSProperties = {
  background: "#2d2d2d",
  color: "rgba(255,255,255,0.9)",
  border: "1px solid rgba(255,255,255,0.15)",
  borderRadius: 4,
  padding: "6px 14px",
  fontSize: 12,
};

// Tooltip only becomes visible on hover/focus (internal `isVisible` state), so a static story
// needs the trigger to already be focused when it mounts -- `autoFocus` fires the `onFocus`
// handler Tooltip injects via `React.cloneElement`, and `delay={0}` skips the hover debounce so
// the popup is showing by the time a screenshot is taken.
export const TopPlacement = () => (
  <div style={previewBg}>
    <Tooltip content="Frame 86 — INTER_FRAME, QP 28" placement="top" delay={0}>
      <button type="button" style={triggerStyle} autoFocus>
        Frame 86
      </button>
    </Tooltip>
  </div>
);

export const BottomPlacement = () => (
  <div style={previewBg}>
    <Tooltip
      content="base_q_idx: 96 (spec 5.9.12)"
      placement="bottom"
      delay={0}
    >
      <button type="button" style={triggerStyle} autoFocus>
        base_q_idx
      </button>
    </Tooltip>
  </div>
);

export const Disabled = () => (
  <div style={previewBg}>
    <Tooltip content="This will never show" placement="top" disabled>
      <button type="button" style={triggerStyle}>
        Hover disabled
      </button>
    </Tooltip>
  </div>
);
