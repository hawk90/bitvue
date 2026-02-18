/**
 * Interactive Tooltips - Type Definitions
 *
 * Types for the tooltip system
 */

export interface TooltipConfig {
  id: string;
  title: string;
  description: string;
  position: "top" | "bottom" | "left" | "right" | "auto";
  delay?: number;
  shortcut?: string;
  actionLabel?: string;
  onAction?: () => void;
  dismissible?: boolean;
  showOnce?: boolean;
}

export interface TooltipContext {
  component: string;
  feature: string;
  hasSeenBefore: boolean;
  timesShown: number;
  lastShown: number | null;
}
