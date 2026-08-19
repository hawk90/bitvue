import { CropDialog } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// CropDialog's `.yuv-diff-dialog-overlay` is `position: fixed` -- give the wrapper a containing
// block (transform) plus a concrete box so the overlay+dialog render inside the visible card
// instead of escaping it to the viewport.
const dialogWrapper: CSSProperties = {
  ...previewBg,
  position: "relative",
  transform: "translateZ(0)",
  width: 600,
  height: 460,
};

export const Default = () => (
  <div style={dialogWrapper}>
    <CropDialog
      current={{ left: 0, right: 0, top: 0, bottom: 0 }}
      onConfirm={() => {}}
      onCancel={() => {}}
    />
  </div>
);

export const WithExistingCrop = () => (
  <div style={dialogWrapper}>
    <CropDialog
      current={{ left: 8, right: 8, top: 16, bottom: 16 }}
      onConfirm={() => {}}
      onCancel={() => {}}
    />
  </div>
);
