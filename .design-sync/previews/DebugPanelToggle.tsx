import { DebugPanelToggle } from "bitvue";
import type { CSSProperties } from "react";

// DebugPanelToggle's button is `position: fixed` (pinned to the app window's edge in the real
// app, top:60/right:0, 24x80px), which escapes a plain wrapper div. `transform` makes the wrapper
// the containing block for fixed-positioned descendants (standard CSS trick) so the button renders
// inside our box instead of floating over the page.
//
// The button is intentionally a small, subtle edge-docked tab in the real app (bg
// rgb(30 30 30 / 95%), border rgb(255 255 255 / 10%), icon color --text-secondary) -- that's by
// design, not a bug. The earlier attempt made the WRAPPER nearly as dark as the button itself
// (#050505 vs the button's ~#1e1e1e), which if anything reduced contrast further, and used a huge
// 200px-tall box that left the button looking lost in empty space. Fix: a wrapper close to the
// button's own footprint (so it isn't dwarfed by empty padding) with a lighter --bg-panel-ish tone
// so the button's dark chip and its faint white border both read clearly against it.
const closedBg: CSSProperties = {
  background: "#3a3a3a",
  padding: 16,
  width: 140,
  height: 120,
  position: "relative",
  transform: "translateZ(0)",
  overflow: "hidden",
  borderRadius: 8,
};

// `.debug-toggle.open` shifts to `right: 320px` (it tracks the real debug panel's width sliding
// into view) -- the closed story's 140px-wide box would clip it out of frame entirely, so the
// open story needs a wrapper wide enough to contain that offset.
const openBg: CSSProperties = {
  ...closedBg,
  width: 420,
};

export const Closed = () => (
  <div style={closedBg}>
    <DebugPanelToggle isOpen={false} onToggle={() => {}} />
  </div>
);

export const Open = () => (
  <div style={openBg}>
    <DebugPanelToggle isOpen={true} onToggle={() => {}} />
  </div>
);
