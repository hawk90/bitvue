import { Skeleton } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  display: "flex",
  flexDirection: "column",
  gap: 16,
  alignItems: "flex-start",
};

export const TextVariant = () => (
  <div style={previewBg}>
    <Skeleton variant="text" width={200} height={16} />
  </div>
);

export const CircularVariant = () => (
  <div style={previewBg}>
    <Skeleton variant="circular" width={40} height={40} />
  </div>
);

export const RectangularVariant = () => (
  <div style={previewBg}>
    <Skeleton variant="rectangular" width={300} height={120} />
  </div>
);

export const DefaultVariant = () => (
  <div style={previewBg}>
    <Skeleton variant="default" width={100} height={20} />
  </div>
);
