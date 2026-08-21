import type { ReactNode } from "react";

interface WelcomeActionRowProps {
  icon: ReactNode;
  /** Spins the icon (CSS `.spinner` animation) -- used for "Opening..." states. */
  iconSpinning?: boolean;
  label: string;
  /** Keycap labels shown right-aligned, e.g. ["⌘", "O"] or ["?"]. Omit while the action is
   *  already in progress -- there's nothing meaningful to press. */
  shortcut?: string[];
  onClick: () => void;
  disabled?: boolean;
}

/**
 * A single "Start" row: icon, label, optional keyboard-shortcut badge. Plain row, not a filled
 * button -- matches VS Code's own Start page list style (see WelcomeScreen.css's module doc).
 */
export function WelcomeActionRow({
  icon,
  iconSpinning,
  label,
  shortcut,
  onClick,
  disabled,
}: WelcomeActionRowProps) {
  return (
    <button className="welcome-row" onClick={onClick} disabled={disabled}>
      <span className={`welcome-row-icon${iconSpinning ? " spinner" : ""}`}>
        {icon}
      </span>
      <span className="welcome-row-label">{label}</span>
      {shortcut && shortcut.length > 0 && (
        <span className="welcome-row-shortcut">
          {shortcut.map((key, i) => (
            <kbd key={i}>{key}</kbd>
          ))}
        </span>
      )}
    </button>
  );
}
