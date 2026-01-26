/**
 * Interactive Tooltips - v0.8.x UX Improvements
 *
 * Context-aware tooltips with interactive help and feature discovery
 */

export interface TooltipConfig {
  id: string;
  title: string;
  description: string;
  position: 'top' | 'bottom' | 'left' | 'right' | 'auto';
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

/**
 * Tooltip manager for feature discovery
 */
export class TooltipManager {
  private tooltips: Map<string, TooltipConfig>;
  private dismissedTips: Set<string>;
  private shownTips: Map<string, number>;
  private currentTooltip: HTMLElement | null = null;
  private hideTimeout: number | null = null;

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
    context?: Partial<TooltipContext>
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
      }, config.delay ?? 5000);
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
    context?: Partial<TooltipContext>
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
  private createTooltip(config: TooltipConfig, target: HTMLElement): HTMLElement {
    const tooltip = document.createElement('div');
    tooltip.className = 'interactive-tooltip';
    tooltip.setAttribute('data-tooltip-id', config.id);

    let html = `
      <div class="tooltip-header">
        <span class="tooltip-title">${config.title}</span>
        ${config.dismissible !== false ? '<button class="tooltip-close" aria-label="Close">×</button>' : ''}
      </div>
      <div class="tooltip-content">
        <p class="tooltip-description">${config.description}</p>
    `;

    if (config.shortcut) {
      html += `<div class="tooltip-shortcut">Shortcut: <kbd>${config.shortcut}</kbd></div>`;
    }

    if (config.actionLabel && config.onAction) {
      html += `<button class="tooltip-action">${config.actionLabel}</button>`;
    }

    html += `</div>`; // Close tooltip-content

    tooltip.innerHTML = html;

    // Add event listeners
    const closeBtn = tooltip.querySelector('.tooltip-close');
    if (closeBtn) {
      closeBtn.addEventListener('click', () => {
        this.dismiss(config.id);
        this.hide();
      });
    }

    const actionBtn = tooltip.querySelector('.tooltip-action');
    if (actionBtn && config.onAction) {
      actionBtn.addEventListener('click', () => {
        config.onAction!();
        this.hide();
      });
    }

    // Close on click outside
    setTimeout(() => {
      document.addEventListener('click', (e) => {
        if (!tooltip.contains(e.target as Node) && !target.contains(e.target as Node)) {
          this.hide();
        }
      }, { once: true });
    }, 0);

    return tooltip;
  }

  /**
   * Position tooltip
   */
  private positionTooltip(
    tooltip: HTMLElement,
    target: HTMLElement,
    position: 'top' | 'bottom' | 'left' | 'right' | 'auto'
  ): void {
    const targetRect = target.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    const viewport = {
      width: window.innerWidth,
      height: window.innerHeight,
    };

    let finalPosition = position;

    // Auto-detect best position
    if (position === 'auto') {
      if (targetRect.top < viewport.height * 0.3) {
        finalPosition = 'bottom';
      } else if (targetRect.bottom > viewport.height * 0.7) {
        finalPosition = 'top';
      } else if (targetRect.left < viewport.width * 0.3) {
        finalPosition = 'right';
      } else if (targetRect.right > viewport.width * 0.7) {
        finalPosition = 'left';
      } else {
        finalPosition = 'top';
      }
    }

    // Apply positioning
    tooltip.style.position = 'absolute';
    tooltip.style.zIndex = '10000';

    switch (finalPosition) {
      case 'top':
        tooltip.style.bottom = `${viewport.height - targetRect.top + 8}px`;
        tooltip.style.left = `${targetRect.left + (targetRect.width - tooltipRect.width) / 2}px`;
        break;
      case 'bottom':
        tooltip.style.top = `${targetRect.bottom + 8}px`;
        tooltip.style.left = `${targetRect.left + (targetRect.width - tooltipRect.width) / 2}px`;
        break;
      case 'left':
        tooltip.style.right = `${viewport.width - targetRect.left + 8}px`;
        tooltip.style.top = `${targetRect.top + (targetRect.height - tooltipRect.height) / 2}px`;
        break;
      case 'right':
        tooltip.style.left = `${targetRect.right + 8}px`;
        tooltip.style.top = `${targetRect.top + (targetRect.height - tooltipRect.height) / 2}px`;
        break;
    }

    // Ensure tooltip stays in viewport
    requestAnimationFrame(() => {
      const rect = tooltip.getBoundingClientRect();

      if (rect.left < 8) {
        tooltip.style.left = '8px';
      }
      if (rect.right > viewport.width - 8) {
        tooltip.style.left = `${viewport.width - rect.width - 8}px`;
      }
      if (rect.top < 8) {
        tooltip.style.top = '8px';
      }
      if (rect.bottom > viewport.height - 8) {
        tooltip.style.top = `${viewport.height - rect.height - 8}px`;
      }
    });
  }

  /**
   * Load state from localStorage
   */
  private loadState(): void {
    try {
      const data = localStorage.getItem('bitvue-tooltips');
      if (data) {
        const parsed = JSON.parse(data);
        this.dismissedTips = new Set(parsed.dismissed || []);
        this.shownTips = new Map(Object.entries(parsed.shown || {}).map(([k, v]) => [k, v as number]));
      }
    } catch {
      // Ignore errors
    }
  }

  /**
   * Save state to localStorage
   */
  private saveState(): void {
    try {
      const data = {
        dismissed: Array.from(this.dismissedTips),
        shown: Object.fromEntries(this.shownTips),
      };
      localStorage.setItem('bitvue-tooltips', JSON.stringify(data));
    } catch {
      // Ignore errors
    }
  }

  /**
   * Reset all tooltips (for testing/debug)
   */
  reset(): void {
    this.dismissedTips.clear();
    this.shownTips.clear();
    localStorage.removeItem('bitvue-tooltips');
  }

  /**
   * Get statistics
   */
  getStats(): { registered: number; dismissed: number; shownTotal: number } {
    let shownTotal = 0;
    this.shownTips.forEach(count => {
      shownTotal += count;
    });

    return {
      registered: this.tooltips.size,
      dismissed: this.dismissedTips.size,
      shownTotal,
    };
  }
}

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
    id: 'filmstrip-intro',
    title: 'Filmstrip Navigation',
    description: 'Click on any frame thumbnail to jump to that frame. Use the view selector to see frame sizes, GOP structure, and more.',
    position: 'bottom',
    showOnce: true,
  });

  // Visualization modes tooltip
  manager.register({
    id: 'viz-modes-intro',
    title: 'Visualization Modes',
    description: 'Press F1-F10 to switch between different visualization modes. Each mode shows different aspects of the video encoding.',
    position: 'right',
    shortcut: 'F1-F10',
    showOnce: true,
  });

  // Keyboard shortcuts tooltip
  manager.register({
    id: 'keyboard-shortcuts-intro',
    title: 'Keyboard Shortcuts',
    description: 'Use arrow keys to navigate frames. Press ? to see all available keyboard shortcuts.',
    position: 'bottom',
    shortcut: 'Ctrl/Cmd + ?',
    showOnce: true,
  });

  // Panels tooltip
  manager.register({
    id: 'panels-intro',
    title: 'Analysis Panels',
    description: 'Toggle different panels to see stream info, syntax details, frame data, and more. Use Alt+1-6 to toggle specific panels.',
    position: 'left',
    showOnce: true,
  });

  // YUV Viewer tooltip
  manager.register({
    id: 'yuv-viewer-intro',
    title: 'YUV Viewer',
    description: 'View the raw YUV pixel data. Toggle channels to see Y, U, V components separately.',
    position: 'top',
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
  if (typeof document === 'undefined') return;

  // Register all feature tooltips
  registerFeatureTooltips();

  // Add tooltip CSS
  if (!document.getElementById('tooltip-styles')) {
    const style = document.createElement('style');
    style.id = 'tooltip-styles';
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
