import { GoToFrameDialog } from "bitvue";
import type { CSSProperties } from "react";

// `.goto-overlay` is `position: fixed` -- give the wrapper a containing block (transform) plus a
// concrete box so it renders inside the visible card instead of escaping it.
const dialogWrapper: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 480,
  height: 320,
};

export const Default = () => (
  <div style={dialogWrapper}>
    <GoToFrameDialog
      isOpen={true}
      onClose={() => {}}
      currentIndex={85}
      totalFrames={250}
      onGoTo={() => {}}
    />
  </div>
);

export const NearLastFrame = () => (
  <div style={dialogWrapper}>
    <GoToFrameDialog
      isOpen={true}
      onClose={() => {}}
      currentIndex={247}
      totalFrames={250}
      onGoTo={() => {}}
    />
  </div>
);
