/**
 * Export Menu Creator
 *
 * Creates the Export menu:
 * - General Export (opens export dialog)
 * - Data Export (Frame Sizes CSV, Unit Tree JSON, Syntax Tree JSON)
 * - Evidence Bundle
 *
 * Reference: VQAnalyzer Export Menu
 */

import { PredefinedMenuItem } from '@tauri-apps/api/menu';
import { createMenuItem, createSubmenu } from '../types';

export async function createExportMenu(): Promise<import('@tauri-apps/api/menu').Submenu> {
  return await createSubmenu({
    text: 'Export',
    items: [
      // Main Export Dialog (opens comprehensive export dialog)
      createMenuItem({
        id: 'export-dialog',
        text: 'Export...',
        event: 'menu-export',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Data Export (SUBMENU)
      createSubmenu({
        text: 'Data Export',
        items: [
          createMenuItem({
            id: 'export-frame-sizes',
            text: 'Frame Sizes (CSV)',
            event: 'menu-export-frame-sizes',
          }),
          createMenuItem({
            id: 'export-unit-tree',
            text: 'Unit Tree (JSON)',
            event: 'menu-export-unit-tree',
          }),
          createMenuItem({
            id: 'export-syntax-tree',
            text: 'Syntax Tree (JSON)',
            event: 'menu-export-syntax-tree',
          }),
        ],
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Evidence Bundle
      createMenuItem({
        id: 'export-evidence',
        text: 'Evidence Bundle...',
        event: 'menu-export-evidence',
      }),
    ],
  });
}
