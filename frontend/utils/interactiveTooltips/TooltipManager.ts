/**
 * Interactive Tooltips - TooltipManager
 *
 * Core tooltip management class
 */

import { TIMING, DIMENSIONS, Z_INDEX } from "../../constants/ui";
import type { TooltipConfig, TooltipContext } from "./types";

/**
 * Tooltip manager for feature discovery
 */
export class TooltipManager {
  private tooltips: Map<string, TooltipConfig>;
  private dismissedTips: Set<string>;
  private shownTips: Map<string, number>;
  private currentTooltip: HTMLElement | null = null;
  private hideTimeout: number | null = null;
  private clickOutsideHandler: (() => void) | null = null;
  private saveTimeout: number | null = null;

  constructor() {
    this.tooltips = new Map();
    this.dismissedTips = new Set();
    this.shownTips = new Map();
    this.loadState();
  }

  /**
   * Register a tooltip
   */
  register(config: TooltipConfig): void {
    this.tooltips.set(config.id, config);
  }

  /**
   * Unregister a tooltip
   */
  unregister(id: string): void {
    this.tooltips.delete(id);
  }

  /**
   * Show a tooltip
   */
  show(
    id: string,
    target: HTMLElement,
    context?: Partial<TooltipContext>,
  ): boolean {
    const config = this.tooltips.get(id);
    if (!config) return false;

    // Check if should show
    if (!this.shouldShow(id, config, context)) {
      return false;
    }

    // Hide current tooltip
    this.hide();

    // Create tooltip element
    const tooltip = this.createTooltip(config, target);
    document.body.appendChild(tooltip);
    this.currentTooltip = tooltip;

    // Position tooltip
    this.positionTooltip(tooltip, target, config.position);

    // Increment shown count
    this.shownTips.set(id, (this.shownTips.get(id) || 0) + 1);

    // Auto-hide after delay
    if (config.delay !== false) {
      this.hideTimeout = window.setTimeout(() => {
        this.hide();
      }, config.delay ?? TIMING.TOOLTIP_AUTO_HIDE_DELAY);
    }

    // Save state
    this.saveState();

    return true;
  }

  /**
   * Hide current tooltip
   */
  hide(): void {
    if (this.hideTimeout) {
      clearTimeout(this.hideTimeout);
      this.hideTimeout = null;
    }

    // Clean up click-outside listener
    if (this.clickOutsideHandler) {
      document.removeEventListener("click", this.clickOutsideHandler);
      this.clickOutsideHandler = null;
    }

    if (this.currentTooltip) {
      this.currentTooltip.remove();
      this.currentTooltip = null;
    }
  }

  /**
   * Dismiss a tooltip (user explicitly closed it)
   */
  dismiss(id: string): void {
    this.dismissedTips.add(id);
    this.saveState();
  }

  /**
   * Check if tooltip should be shown
   */
  private shouldShow(
    id: string,
    config: TooltipConfig,
    context?: Partial<TooltipContext>,
  ): boolean {
    // Don't show if dismissed
    if (this.dismissedTips.has(id)) {
      return false;
    }

    // Check showOnce option
    if (config.showOnce && this.shownTips.has(id)) {
      return false;
    }

    // Check context
    if (context?.hasSeenBefore && config.showOnce) {
      return false;
    }

    // Don't show too many times
    const timesShown = this.shownTips.get(id) || 0;
    if (timesShown >= 3) {
      return false;
    }

    return true;
  }

  /**
   * Create tooltip element
   */
  private createTooltip(
    config: TooltipConfig,
    target: HTMLElement,
  ): HTMLElement {
    const tooltip = document.createElement("div");
    tooltip.className = "interactive-tooltip";
    tooltip.setAttribute("data-tooltip-id", config.id);

    // Create header with title (using textContent to prevent XSS)
    const header = document.createElement("div");
    header.className = "tooltip-header";

    const titleSpan = document.createElement("span");
    titleSpan.className = "tooltip-title";
    titleSpan.textContent = config.title;
    header.appendChild(titleSpan);

    if (config.dismissible !== false) {
      const closeBtn = document.createElement("button");
      closeBtn.className = "tooltip-close";
      closeBtn.setAttribute("aria-label", "Close");
      closeBtn.textContent = "×";
      closeBtn.addEventListener("click", () => {
        this.dismiss(config.id);
        this.hide();
      });
      header.appendChild(closeBtn);
    }

    tooltip.appendChild(header);

    // Create content with description (using textContent to prevent XSS)
    const content = document.createElement("div");
    content.className = "tooltip-content";

    const descP = document.createElement("p");
    descP.className = "tooltip-description";
    descP.textContent = config.description;
    content.appendChild(descP);

    // Add shortcut if present (using textContent to prevent XSS)
    if (config.shortcut) {
      const shortcutDiv = document.createElement("div");
      shortcutDiv.className = "tooltip-shortcut";
      const shortcutLabel = document.createTextNode("Shortcut: ");
      shortcutDiv.appendChild(shortcutLabel);

      const kbd = document.createElement("kbd");
      kbd.textContent = config.shortcut;
      shortcutDiv.appendChild(kbd);

      content.appendChild(shortcutDiv);
    }

    // Add action button if present (using textContent to prevent XSS)
    if (config.actionLabel && config.onAction) {
      const actionBtn = document.createElement("button");
      actionBtn.className = "tooltip-action";
      actionBtn.textContent = config.actionLabel;
      actionBtn.addEventListener("click", () => {
        config.onAction!();
        this.hide();
      });
      content.appendChild(actionBtn);
    }

    tooltip.appendChild(content);

    // Close on click outside - store handler reference for cleanup
    const handleOutsideClick = (e: Event) => {
      if (
        !tooltip.contains(e.target as Node) &&
        !target.contains(e.target as Node)
      ) {
        this.hide();
      }
    };
    this.clickOutsideHandler = handleOutsideClick;

    // Add listener with a small delay to avoid immediate triggering
    setTimeout(() => {
      document.addEventListener("click", handleOutsideClick);
    }, 0);

    return tooltip;
  }

  /**
   * Position tooltip
   */
  private positionTooltip(
    tooltip: HTMLElement,
    target: HTMLElement,
    position: "top" | "bottom" | "left" | "right" | "auto",
  ): void {
    const targetRect = target.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    const viewport = {
      width: window.innerWidth,
      height: window.innerHeight,
    };

    let finalPosition = position;

    // Auto-detect best position
    if (position === "auto") {
      if (targetRect.top < viewport.height * 0.3) {
        finalPosition = "bottom";
      } else if (targetRect.bottom > viewport.height * 0.7) {
        finalPosition = "top";
      } else if (targetRect.left < viewport.width * 0.3) {
        finalPosition = "right";
      } else if (targetRect.right > viewport.width * 0.7) {
        finalPosition = "left";
      } else {
        finalPosition = "top";
      }
    }

    // Apply positioning
    tooltip.style.position = "absolute";
    tooltip.style.zIndex = String(Z_INDEX.TOOLTIP);

    switch (finalPosition) {
      case "top":
        tooltip.style.bottom = `${viewport.height - targetRect.top + DIMENSIONS.TOOLTIP_OFFSET}px`;
        tooltip.style.left = `${targetRect.left + (targetRect.width - tooltipRect.width) / 2}px`;
        break;
      case "bottom":
        tooltip.style.top = `${targetRect.bottom + DIMENSIONS.TOOLTIP_OFFSET}px`;
        tooltip.style.left = `${targetRect.left + (targetRect.width - tooltipRect.width) / 2}px`;
        break;
      case "left":
        tooltip.style.right = `${viewport.width - targetRect.left + DIMENSIONS.TOOLTIP_OFFSET}px`;
        tooltip.style.top = `${targetRect.top + (targetRect.height - tooltipRect.height) / 2}px`;
        break;
      case "right":
        tooltip.style.left = `${targetRect.right + DIMENSIONS.TOOLTIP_OFFSET}px`;
        tooltip.style.top = `${targetRect.top + (targetRect.height - tooltipRect.height) / 2}px`;
        break;
    }

    // Ensure tooltip stays in viewport
    requestAnimationFrame(() => {
      const rect = tooltip.getBoundingClientRect();

      if (rect.left < DIMENSIONS.TOOLTIP_OFFSET) {
        tooltip.style.left = `${DIMENSIONS.TOOLTIP_OFFSET}px`;
      }
      if (rect.right > viewport.width - DIMENSIONS.TOOLTIP_OFFSET) {
        tooltip.style.left = `${viewport.width - rect.width - DIMENSIONS.TOOLTIP_OFFSET}px`;
      }
      if (rect.top < DIMENSIONS.TOOLTIP_OFFSET) {
        tooltip.style.top = `${DIMENSIONS.TOOLTIP_OFFSET}px`;
      }
      if (rect.bottom > viewport.height - DIMENSIONS.TOOLTIP_OFFSET) {
        tooltip.style.top = `${viewport.height - rect.height - DIMENSIONS.TOOLTIP_OFFSET}px`;
      }
    });
  }

  /**
   * Load state from localStorage
   */
  private loadState(): void {
    try {
      const data = localStorage.getItem("bitvue-tooltips");
      if (data) {
        const parsed = JSON.parse(data);
        this.dismissedTips = new Set(parsed.dismissed || []);
        this.shownTips = new Map(
          Object.entries(parsed.shown || {}).map(([k, v]) => [k, v as number]),
        );
      }
    } catch {
      // Ignore errors
    }
  }

  /**
   * Save state to localStorage (debounced)
   * Saves after a delay to reduce write frequency
   */
  private saveState(): void {
    // Clear any pending save
    if (this.saveTimeout !== null) {
      clearTimeout(this.saveTimeout);
    }

    // Debounce the save operation
    this.saveTimeout = window.setTimeout(() => {
      try {
        const data = {
          dismissed: Array.from(this.dismissedTips),
          shown: Object.fromEntries(this.shownTips),
        };
        localStorage.setItem("bitvue-tooltips", JSON.stringify(data));
      } catch {
        // Ignore errors
      }
      this.saveTimeout = null;
    }, TIMING.STORAGE_DEBOUNCE_DELAY);
  }

  /**
   * Reset all tooltips (for testing/debug)
   */
  reset(): void {
    // Clear any pending save
    if (this.saveTimeout !== null) {
      clearTimeout(this.saveTimeout);
      this.saveTimeout = null;
    }
    this.dismissedTips.clear();
    this.shownTips.clear();
    localStorage.removeItem("bitvue-tooltips");
  }

  /**
   * Get statistics
   */
  getStats(): { registered: number; dismissed: number; shownTotal: number } {
    let shownTotal = 0;
    this.shownTips.forEach((count) => {
      shownTotal += count;
    });

    return {
      registered: this.tooltips.size,
      dismissed: this.dismissedTips.size,
      shownTotal,
    };
  }
}
