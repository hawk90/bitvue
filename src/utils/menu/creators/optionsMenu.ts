/**
 * Options Menu Creator
 *
 * Creates the Options menu with settings:
 * - Color Space (BT.601, BT.709, BT.2020, YUV as RGB, YUV as GBR)
 * - CPU & Performance (AVX2, Loop playback)
 * - Codec Settings (HEVC extensions, VVC options, Digest options)
 * - Dark/Light Theme
 * - Save/Load/Reset/Auto-save Layout
 *
 * Reference: VQAnalyzer Options Menu
 */

import { PredefinedMenuItem } from '@tauri-apps/api/menu';
import { createMenuItem, createSubmenu } from '../types';

export async function createOptionsMenu(): Promise<import('@tauri-apps/api/menu').Submenu> {
  return await createSubmenu({
    text: 'Options',
    items: [
      // Color Space (SUBMENU)
      createSubmenu({
        text: 'Color Space',
        items: [
          createMenuItem({
            id: 'color-bt601',
            text: 'ITU Rec. 601',
            event: 'menu-color-bt601',
          }),
          createMenuItem({
            id: 'color-bt709',
            text: 'ITU Rec. 709',
            event: 'menu-color-bt709',
          }),
          createMenuItem({
            id: 'color-bt2020',
            text: 'ITU Rec. 2020',
            event: 'menu-color-bt2020',
          }),
          PredefinedMenuItem.new({ item: 'Separator' }),
          createMenuItem({
            id: 'color-yuv-rgb',
            text: 'YUV as RGB',
            event: 'menu-color-yuv-rgb',
          }),
          createMenuItem({
            id: 'color-yuv-gbr',
            text: 'YUV as GBR',
            event: 'menu-color-yuv-gbr',
          }),
        ],
      }),

      // CPU & Performance (SUBMENU)
      createSubmenu({
        text: 'CPU & Performance',
        items: [
          createMenuItem({
            id: 'cpu-avx2',
            text: 'Enable CPU optimizations [avx2]',
            event: 'menu-cpu-avx2',
          }),
          PredefinedMenuItem.new({ item: 'Separator' }),
          createMenuItem({
            id: 'loop-playback',
            text: 'Loop playback',
            event: 'menu-loop-playback',
          }),
        ],
      }),

      // Codec Settings (SUBMENU)
      createSubmenu({
        text: 'Codec Settings',
        items: [
          createMenuItem({
            id: 'codec-hevc-extensions',
            text: 'HEVC: Enable extensions',
            event: 'menu-codec-hevc-ext',
          }),
          createMenuItem({
            id: 'codec-hevc-index',
            text: 'HEVC: Enable stream index',
            event: 'menu-codec-hevc-index',
          }),
          createMenuItem({
            id: 'codec-hevc-mv',
            text: 'HEVC: Show only visible CTB MV',
            event: 'menu-codec-hevc-mv',
          }),
          PredefinedMenuItem.new({ item: 'Separator' }),
          createMenuItem({
            id: 'codec-vvc-dynamic',
            text: 'VVC: Dynamic selection info',
            event: 'menu-codec-vvc-dynamic',
          }),
          createMenuItem({
            id: 'codec-vvc-details',
            text: 'VVC: Details popup window',
            event: 'menu-codec-vvc-details',
          }),
          PredefinedMenuItem.new({ item: 'Separator' }),
          createMenuItem({
            id: 'codec-digest-force',
            text: 'Digest: Force digest',
            event: 'menu-digest-force',
          }),
          createMenuItem({
            id: 'codec-digest-none',
            text: 'Digest: No digest',
            event: 'menu-digest-none',
          }),
          createMenuItem({
            id: 'codec-digest-stream',
            text: 'Digest: As in bitstream',
            event: 'menu-digest-stream',
          }),
        ],
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Theme options
      createMenuItem({
        id: 'theme-dark',
        text: 'Dark Theme',
        event: 'menu-theme-change',
        eventDetail: 'dark',
      }),

      createMenuItem({
        id: 'theme-light',
        text: 'Light Theme',
        event: 'menu-theme-change',
        eventDetail: 'light',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Layout options
      createMenuItem({
        id: 'save-layout',
        text: 'Save Layout...',
        event: 'menu-save-layout',
      }),

      createMenuItem({
        id: 'load-layout',
        text: 'Load Layout...',
        event: 'menu-load-layout',
      }),

      createMenuItem({
        id: 'reset-layout',
        text: 'Reset Layout',
        event: 'menu-reset-layout',
      }),

      createMenuItem({
        id: 'auto-save-layout',
        text: 'Auto-save on exit',
        event: 'menu-auto-save-layout',
      }),
    ],
  });
}
