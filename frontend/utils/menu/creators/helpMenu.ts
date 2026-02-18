/**
 * Help Menu Creator
 *
 * Creates the Help menu:
 * - Documentation
 * - Keyboard Shortcuts
 * - About Bitvue
 *
 * Reference: VQAnalyzer Help Menu
 */

import { PredefinedMenuItem } from "@tauri-apps/api/menu";
import { createMenuItem, createSubmenu } from "../types";

export async function createHelpMenu(): Promise<
  import("@tauri-apps/api/menu").Submenu
> {
  return await createSubmenu({
    text: "Help",
    items: [
      // Documentation
      createMenuItem({
        id: "docs",
        text: "Documentation",
        event: "menu-documentation",
      }),

      // Keyboard Shortcuts
      createMenuItem({
        id: "shortcuts",
        text: "Keyboard Shortcuts",
        event: "menu-keyboard-shortcuts",
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // About Bitvue
      createMenuItem({
        id: "about",
        text: "About Bitvue",
        event: "menu-about",
      }),
    ],
  });
}
