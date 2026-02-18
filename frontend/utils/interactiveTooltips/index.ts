/**
 * Interactive Tooltips - Main Module
 *
 * Context-aware tooltips with interactive help and feature discovery
 * Re-exports all types and functionality
 */

export { TooltipManager } from "./TooltipManager";
export type { TooltipConfig, TooltipContext } from "./types";

import { TooltipManager } from "./TooltipManager";
import type { TooltipConfig } from "./types";

/**
 * Global tooltip manager instance
 */
export const globalTooltipManager = new TooltipManager();

/**
 * Register feature discovery tooltips
 */
export function registerFeatureTooltips(): void {
  const manager = globalTooltipManager;

  // Filmstrip tooltip
  manager.register({
    id: "filmstrip-intro",
    title: "Filmstrip Navigation",
    description:
      "Click on any frame thumbnail to jump to that frame. Use the view selector to see frame sizes, GOP structure, and more.",
    position: "bottom",
    showOnce: true,
  });

  // Visualization modes tooltip
  manager.register({
    id: "viz-modes-intro",
    title: "Visualization Modes",
    description:
      "Press F1-F10 to switch between different visualization modes. Each mode shows different aspects of the video encoding.",
    position: "right",
    shortcut: "F1-F10",
    showOnce: true,
  });

  // Keyboard shortcuts tooltip
  manager.register({
    id: "keyboard-shortcuts-intro",
    title: "Keyboard Shortcuts",
    description:
      "Use arrow keys to navigate frames. Press ? to see all available keyboard shortcuts.",
    position: "bottom",
    shortcut: "Ctrl/Cmd + ?",
    showOnce: true,
  });

  // Panels tooltip
  manager.register({
    id: "panels-intro",
    title: "Analysis Panels",
    description:
      "Toggle different panels to see stream info, syntax details, frame data, and more. Use Alt+1-6 to toggle specific panels.",
    position: "left",
    showOnce: true,
  });

  // YUV Viewer tooltip
  manager.register({
    id: "yuv-viewer-intro",
    title: "YUV Viewer",
    description:
      "View the raw YUV pixel data. Toggle channels to see Y, U, V components separately.",
    position: "top",
    showOnce: true,
  });
}

/**
 * Hook for showing tooltips on mount
 */
export function useTooltip(id: string, config?: Partial<TooltipConfig>) {
  const show = (target: HTMLElement) => {
    return globalTooltipManager.show(id, target);
  };

  const dismiss = () => {
    globalTooltipManager.dismiss(id);
  };

  return { show, dismiss };
}

/**
 * Initialize tooltips when DOM is ready
 */
export function initTooltips(): void {
  if (typeof document === "undefined") return;

  // Register all feature tooltips
  registerFeatureTooltips();

  // Add tooltip CSS
  if (!document.getElementById("tooltip-styles")) {
    const style = document.createElement("style");
    style.id = "tooltip-styles";
    style.textContent = `
      .interactive-tooltip {
        background: var(--bg-secondary, #1e1e1e);
        border: 1px solid var(--border-color, #444);
        border-radius: 8px;
        padding: 12px 16px;
        max-width: 320px;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
        animation: tooltip-fade-in 0.2s ease-out;
      }

      @keyframes tooltip-fade-in {
        from {
          opacity: 0;
          transform: translateY(-4px);
        }
        to {
          opacity: 1;
          transform: translateY(0);
        }
      }

      .tooltip-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 8px;
      }

      .tooltip-title {
        font-weight: 600;
        font-size: 14px;
        color: var(--text-primary, #fff);
      }

      .tooltip-close {
        background: none;
        border: none;
        color: var(--text-secondary, #888);
        font-size: 20px;
        line-height: 1;
        padding: 0;
        width: 24px;
        height: 24px;
        cursor: pointer;
        border-radius: 4px;
      }

      .tooltip-close:hover {
        background: var(--bg-hover, #333);
        color: var(--text-primary, #fff);
      }

      .tooltip-content {
        font-size: 13px;
        color: var(--text-secondary, #aaa);
      }

      .tooltip-description {
        margin: 0 0 8px 0;
        line-height: 1.4;
      }

      .tooltip-shortcut {
        display: flex;
        align-items: center;
        gap: 6px;
        margin-top: 8px;
        font-size: 12px;
      }

      .tooltip-shortcut kbd {
        background: var(--bg-tertiary, #2a2a2a);
        border: 1px solid var(--border-color, #444);
        border-radius: 4px;
        padding: 2px 6px;
        font-family: monospace;
        font-size: 11px;
      }

      .tooltip-action {
        margin-top: 12px;
        padding: 6px 12px;
        background: var(--accent-color, #007acc);
        border: none;
        border-radius: 4px;
        color: white;
        font-size: 12px;
        cursor: pointer;
        width: 100%;
      }

      .tooltip-action:hover {
        opacity: 0.9;
      }
    `;
    document.head.appendChild(style);
  }
}
