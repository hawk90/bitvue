import { KeyboardShortcutsDialog } from "bitvue";
import type { CSSProperties } from "react";

// `.shortcuts-overlay` is `position: fixed` -- give the wrapper a containing block (transform)
// plus a concrete box so it renders inside the visible card instead of escaping it. The dialog
// itself lists every shortcut category, so it needs real vertical room.
const dialogWrapper: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 640,
  height: 700,
};

// Props are just isOpen/onClose -- all shortcut content comes from the static
// KEYBOARD_SHORTCUTS/isMac() utils, so a single story covers the component fully.
export const Default = () => (
  <div style={dialogWrapper}>
    <KeyboardShortcutsDialog isOpen={true} onClose={() => {}} />
  </div>
);
