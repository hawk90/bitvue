/**
 * Interactive Tooltips - v0.8.x UX Improvements
 *
 * Context-aware tooltips with interactive help and feature discovery
 *
 * @deprecated This file has been split into a module for better organization.
 * Import from './interactiveTooltips' instead.
 *
 * @example
 * // Old import (still works for backward compatibility):
 * import { globalTooltipManager, useTooltip } from './interactiveTooltips';
 *
 * // New imports (recommended):
 * import { globalTooltipManager, useTooltip } from './interactiveTooltips';
 * import { TooltipManager } from './interactiveTooltips/TooltipManager';
 * import type { TooltipConfig } from './interactiveTooltips/types';
 */

// Re-export everything from the new modular structure
export {
  TooltipManager,
  globalTooltipManager,
  registerFeatureTooltips,
  useTooltip,
  initTooltips,
} from "./interactiveTooltips";

export type {
  TooltipConfig,
  TooltipContext,
} from "./interactiveTooltips/types";
