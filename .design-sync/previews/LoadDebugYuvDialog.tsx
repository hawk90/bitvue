import { LoadDebugYuvDialog } from "bitvue";
import type { CSSProperties } from "react";

// `.yuv-diff-dialog-overlay` is `position: fixed` -- give the wrapper a containing block
// (transform) plus a concrete box so it renders inside the visible card instead of escaping it.
const dialogWrapper: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 560,
  height: 460,
};

// `filePath` drives the component's filename-token auto-detect heuristic (resolution/format/
// bitdepth) -- each story below exercises a different branch of `autoDetect()`.
export const Detected420_8bit = () => (
  <div style={dialogWrapper}>
    <LoadDebugYuvDialog
      filePath="/Users/hawk/clips/ref_1920x1080_420_8bit.yuv"
      onConfirm={() => {}}
      onCancel={() => {}}
    />
  </div>
);

export const Detected444_10bit = () => (
  <div style={dialogWrapper}>
    <LoadDebugYuvDialog
      filePath="/Users/hawk/clips/master_3840x2160_444_10bit.yuv"
      onConfirm={() => {}}
      onCancel={() => {}}
    />
  </div>
);

// No recognizable tokens in the filename -- falls back to the component's default
// (1920x1080 / I420 / 8-bit).
export const FallbackDefaults = () => (
  <div style={dialogWrapper}>
    <LoadDebugYuvDialog
      filePath="/Users/hawk/clips/reference.yuv"
      onConfirm={() => {}}
      onCancel={() => {}}
    />
  </div>
);
