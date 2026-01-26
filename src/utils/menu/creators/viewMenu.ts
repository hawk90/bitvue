/**
 * View Menu Creator
 *
 * Creates the View menu:
 * - Reset Layout
 * - Stream Tree
 * - Player
 * - Diagnostics
 *
 * Reference: VQAnalyzer View Menu
 */

import { PredefinedMenuItem } from '@tauri-apps/api/menu';
import { createMenuItem, createSubmenu } from '../types';

export async function createViewMenu(): Promise<import('@tauri-apps/api/menu').Submenu> {
  return await createSubmenu({
    text: 'View',
    items: [
      // Reset Layout
      createMenuItem({
        id: 'reset-layout',
        text: 'Reset Layout',
        event: 'menu-reset-layout',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Stream Tree
      createMenuItem({
        id: 'show-stream-tree',
        text: 'Stream Tree',
        event: 'menu-toggle-stream-tree',
      }),

      // Player
      createMenuItem({
        id: 'show-player',
        text: 'Player',
        event: 'menu-toggle-player',
      }),

      // Diagnostics
      createMenuItem({
        id: 'show-diagnostics',
        text: 'Diagnostics',
        event: 'menu-toggle-diagnostics',
      }),
    ],
  });
}
