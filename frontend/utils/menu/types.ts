/**
 * Menu Module Type Definitions
 *
 * Shared types for menu creation functions
 */

import {
  MenuItem,
  Submenu,
  PredefinedMenuItem,
  type MenuItemOptions,
} from "@tauri-apps/api/menu";

/**
 * Extended menu item options with action
 */
interface ExtendedMenuItemOptions extends Omit<MenuItemOptions, "action"> {
  action?: () => void;
}

/**
 * Result type for menu item creators
 * Can be a MenuItem, Submenu, PredefinedMenuItem (separator), or their promises
 */
export type MenuItemResult =
  | MenuItem
  | Submenu
  | PredefinedMenuItem
  | Promise<MenuItem>
  | Promise<Submenu>
  | Promise<PredefinedMenuItem>;

/**
 * Base interface for menu item configuration
 */
export interface MenuItemConfig {
  /** Unique identifier for the menu item */
  id: string;
  /** Display text for the menu item */
  text: string;
  /** Keyboard shortcut (e.g., 'cmd+o', 'f1') */
  accelerator?: string;
  /** Event name to dispatch when clicked */
  event?: string;
  /** Optional event detail payload */
  eventDetail?: unknown;
}

/**
 * Configuration for creating menu items
 */
export interface MenuItemCreator {
  (config: MenuItemConfig): Promise<MenuItem>;
}

/**
 * Configuration for creating submenu items
 */
export interface SubmenuItemConfig {
  /** Display text for the submenu */
  text: string;
  /** Array of submenu items */
  items: MenuItemResult[];
}

/**
 * Configuration for creating submenus
 */
export interface SubmenuCreator {
  (config: SubmenuItemConfig): Promise<Submenu>;
}

/**
 * Helper to dispatch menu events
 */
export function dispatchMenuEvent(event: string, detail?: unknown): void {
  if (detail !== undefined) {
    window.dispatchEvent(new CustomEvent(event, { detail }));
  } else {
    window.dispatchEvent(new CustomEvent(event));
  }
}

/**
 * Create a standard menu item with event dispatch
 */
export async function createMenuItem(
  config: MenuItemConfig,
): Promise<MenuItem> {
  const itemConfig: ExtendedMenuItemOptions = {
    id: config.id,
    text: config.text,
  };

  if (config.accelerator) {
    itemConfig.accelerator = config.accelerator;
  }

  if (config.event || config.eventDetail !== undefined) {
    itemConfig.action = () => {
      if (config.event) {
        dispatchMenuEvent(config.event!, config.eventDetail);
      }
    };
  }

  return await MenuItem.new(itemConfig);
}

/**
 * Create a submenu with nested items
 */
export async function createSubmenu(
  config: SubmenuItemConfig,
): Promise<Submenu> {
  // Wait for all items to resolve
  const resolvedItems = await Promise.all(config.items);
  return await Submenu.new({
    text: config.text,
    items: resolvedItems,
  });
}
