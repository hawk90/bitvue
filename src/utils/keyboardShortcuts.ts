/**
 * Keyboard Shortcuts Utility
 *
 * Global keyboard shortcuts for navigation and actions
 */

export interface ShortcutConfig {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  shift?: boolean;
  alt?: boolean;
  description: string;
  action: () => void;
}

export interface ShortcutCategory {
  name: string;
  shortcuts: ShortcutConfig[];
}

/**
 * All keyboard shortcuts organized by category - v0.8.x
 */
export const KEYBOARD_SHORTCUTS: ShortcutCategory[] = [
  {
    name: 'Navigation',
    shortcuts: [
      { key: 'ArrowLeft', description: 'Previous frame', action: () => {} },
      { key: 'ArrowRight', description: 'Next frame', action: () => {} },
      { key: 'ArrowUp', description: 'Jump to previous keyframe', action: () => {} },
      { key: 'ArrowDown', description: 'Jump to next keyframe', action: () => {} },
      { key: 'Home', description: 'First frame', action: () => {} },
      { key: 'End', description: 'Last frame', action: () => {} },
      { key: 'PageUp', description: 'Previous 10 frames', action: () => {} },
      { key: 'PageDown', description: 'Next 10 frames', action: () => {} },
      { key: '[', description: 'Previous I-frame', action: () => {} },
      { key: ']', description: 'Next I-frame', action: () => {} },
      { key: '<', description: 'Go back 10%', action: () => {} },
      { key: '>', description: 'Go forward 10%', action: () => {} },
    ],
  },
  {
    name: 'Playback',
    shortcuts: [
      { key: ' ', description: 'Play/Pause', action: () => {} },
      { key: 'j', description: 'Reverse playback', action: () => {} },
      { key: 'k', description: 'Pause', action: () => {} },
      { key: 'l', description: 'Forward playback', action: () => {} },
      { key: 'J', shift: true, description: 'Slower reverse', action: () => {} },
      { key: 'L', shift: true, description: 'Faster forward', action: () => {} },
      { key: '/', description: 'Step backward', action: () => {} },
      { key: '.', description: 'Step forward', action: () => {} },
    ],
  },
  {
    name: 'View',
    shortcuts: [
      { key: '+', description: 'Zoom in', action: () => {} },
      { key: '-', description: 'Zoom out', action: () => {} },
      { key: '0', ctrl: true, meta: true, description: 'Reset zoom', action: () => {} },
      { key: 'f', description: 'Toggle fullscreen', action: () => {} },
      { key: '1', description: 'Fit to window', action: () => {} },
      { key: '2', description: 'Actual size (100%)', action: () => {} },
      { key: '3', description: 'Zoom 200%', action: () => {} },
    ],
  },
  {
    name: 'Visualization Modes',
    shortcuts: [
      { key: 'F1', description: 'Overview mode', action: () => {} },
      { key: 'F2', description: 'Coding Flow mode', action: () => {} },
      { key: 'F3', description: 'Prediction mode', action: () => {} },
      { key: 'F4', description: 'Transform mode', action: () => {} },
      { key: 'F5', description: 'QP Map mode', action: () => {} },
      { key: 'F6', description: 'MV Field mode', action: () => {} },
      { key: 'F7', description: 'Reference mode', action: () => {} },
      { key: 'F8', description: 'B-Pyramid mode', action: () => {} },
      { key: 'F9', description: 'HRD Buffer mode', action: () => {} },
      { key: 'F10', description: 'Tile/Partition mode', action: () => {} },
    ],
  },
  {
    name: 'Filmstrip',
    shortcuts: [
      { key: 't', description: 'Toggle filmstrip', action: () => {} },
      { key: 'T', shift: true, description: 'Cycle filmstrip view', action: () => {} },
      { key: 's', ctrl: true, meta: true, description: 'Toggle frame sizes', action: () => {} },
    ],
  },
  {
    name: 'Panels',
    shortcuts: [
      { key: 'p', description: 'Toggle panels', action: () => {} },
      { key: '1', alt: true, description: 'Toggle Stream panel', action: () => {} },
      { key: '2', alt: true, description: 'Toggle Syntax panel', action: () => {} },
      { key: '3', alt: true, description: 'Toggle Selection panel', action: () => {} },
      { key: '4', alt: true, description: 'Toggle HEX panel', action: () => {} },
      { key: '5', alt: true, description: 'Toggle Info panel', action: () => {} },
      { key: '6', alt: true, description: 'Toggle Stats panel', action: () => {} },
    ],
  },
  {
    name: 'Analysis',
    shortcuts: [
      { key: 'q', description: 'Quick QP view', action: () => {} },
      { key: 'm', description: 'Quick MV view', action: () => {} },
      { key: 'b', description: 'Quick partition view', action: () => {} },
      { key: 'r', description: 'Toggle reference graph', action: () => {} },
    ],
  },
  {
    name: 'Application',
    shortcuts: [
      { key: 'o', ctrl: true, meta: true, description: 'Open file', action: () => {} },
      { key: 'w', ctrl: true, meta: true, description: 'Close file', action: () => {} },
      { key: '?', ctrl: true, meta: true, description: 'Show shortcuts', action: () => {} },
      { key: ',', ctrl: true, meta: true, description: 'Settings', action: () => {} },
      { key: 'Escape', description: 'Clear selection / Close modal', action: () => {} },
      { key: 'i', ctrl: true, meta: true, description: 'Show file info', action: () => {} },
    ],
  },
];

/**
 * Check if event matches shortcut configuration
 */
export function matchesShortcut(event: KeyboardEvent, shortcut: ShortcutConfig): boolean {
  return (
    event.key === shortcut.key &&
    !!event.ctrlKey === !!shortcut.ctrl &&
    !!event.metaKey === !!shortcut.meta &&
    !!event.shiftKey === !!shortcut.shift &&
    !!event.altKey === !!shortcut.alt
  );
}

/**
 * Get display string for shortcut
 */
export function getShortcutDisplay(shortcut: ShortcutConfig): string {
  const parts: string[] = [];

  if (shortcut.ctrl) parts.push(isMac() ? '⌘' : 'Ctrl');
  if (shortcut.meta) parts.push(isMac() ? '⌘' : 'Win');
  if (shortcut.alt) parts.push(isMac() ? '⌥' : 'Alt');
  if (shortcut.shift) parts.push(isMac() ? '⇧' : 'Shift');

  let key = shortcut.key;
  // Map special keys
  const keyMap: Record<string, string> = {
    ' ': 'Space',
    'ArrowLeft': '←',
    'ArrowRight': '→',
    'ArrowUp': '↑',
    'ArrowDown': '↓',
    'PageUp': 'Page Up',
    'PageDown': 'Page Down',
  };
  key = keyMap[key] || key;

  parts.push(key);
  return parts.join(isMac() ? '' : '+');
}

/**
 * Check if running on macOS
 */
export function isMac(): boolean {
  return typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform);
}

/**
 * Global keyboard shortcut handler class
 */
export class KeyboardShortcutHandler {
  private shortcuts: Map<string, ShortcutConfig[]> = new Map();
  private isEnabled = true;

  /**
   * Register a shortcut
   */
  register(shortcut: ShortcutConfig): () => void {
    const key = this.getShortcutKey(shortcut);

    if (!this.shortcuts.has(key)) {
      this.shortcuts.set(key, []);
    }

    this.shortcuts.get(key)!.push(shortcut);

    // Return unregister function
    return () => {
      const shortcuts = this.shortcuts.get(key);
      if (shortcuts) {
        const index = shortcuts.indexOf(shortcut);
        if (index > -1) {
          shortcuts.splice(index, 1);
        }
      }
    };
  }

  /**
   * Handle keyboard event
   */
  handle(event: KeyboardEvent): boolean {
    if (!this.isEnabled) return false;

    // Don't handle if in input field
    const target = event.target as HTMLElement;
    if (
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable
    ) {
      return false;
    }

    const key = this.getEventKey(event);
    const shortcuts = this.shortcuts.get(key);

    if (shortcuts && shortcuts.length > 0) {
      event.preventDefault();
      shortcuts.forEach(shortcut => shortcut.action());
      return true;
    }

    return false;
  }

  /**
   * Enable/disable handler
   */
  setEnabled(enabled: boolean): void {
    this.isEnabled = enabled;
  }

  /**
   * Get all registered shortcuts
   */
  getAllShortcuts(): ShortcutConfig[] {
    const all: ShortcutConfig[] = [];
    this.shortcuts.forEach(shortcuts => all.push(...shortcuts));
    return all;
  }

  /**
   * Clear all shortcuts
   */
  clear(): void {
    this.shortcuts.clear();
  }

  private getShortcutKey(shortcut: ShortcutConfig): string {
    return [
      shortcut.ctrl ? 'c' : '',
      shortcut.meta ? 'm' : '',
      shortcut.shift ? 's' : '',
      shortcut.alt ? 'a' : '',
      shortcut.key.toLowerCase(),
    ].join('');
  }

  private getEventKey(event: KeyboardEvent): string {
    return [
      event.ctrlKey ? 'c' : '',
      event.metaKey ? 'm' : '',
      event.shiftKey ? 's' : '',
      event.altKey ? 'a' : '',
      event.key.toLowerCase(),
    ].join('');
  }
}

/**
 * Global shortcut handler instance
 */
export const globalShortcutHandler = new KeyboardShortcutHandler();
