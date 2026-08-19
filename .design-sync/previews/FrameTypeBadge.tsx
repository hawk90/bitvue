import { FrameTypeBadge } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  display: "flex",
  gap: 8,
};

export const IFrame = () => (
  <div style={previewBg}>
    <FrameTypeBadge frameType="I" />
  </div>
);

export const PFrame = () => (
  <div style={previewBg}>
    <FrameTypeBadge frameType="P" />
  </div>
);

export const BFrame = () => (
  <div style={previewBg}>
    <FrameTypeBadge frameType="B" />
  </div>
);

export const AllTypes = () => (
  <div style={previewBg}>
    <FrameTypeBadge frameType="I" />
    <FrameTypeBadge frameType="P" />
    <FrameTypeBadge frameType="B" />
  </div>
);
