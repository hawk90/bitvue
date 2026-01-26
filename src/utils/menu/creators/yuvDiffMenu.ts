/**
 * YUVDiff Menu Creator
 *
 * Creates the YUVDiff menu with comparison tools:
 * - Open debug YUV... (Cmd+Y)
 * - Recent YUV files
 * - Close debug YUV
 * - Subsampling (Planar, Interleaved)
 * - Display/Decode order
 * - Stream crop values
 * - Picture offset
 * - Bitdepth options
 * - Check for file changes
 * - PSNR/SSIM/Delta maps
 * - Export All Frames
 * - Export Metrics CSV
 *
 * Reference: VQAnalyzer YUVDiff Menu
 */

import { PredefinedMenuItem } from '@tauri-apps/api/menu';
import { createMenuItem, createSubmenu } from '../types';

export async function createYUVDiffMenu(): Promise<import('@tauri-apps/api/menu').Submenu> {
  return await createSubmenu({
    text: 'YUVDiff',
    items: [
      // Open debug YUV...
      createMenuItem({
        id: 'open-debug-yuv',
        text: 'Open debug YUV...',
        accelerator: 'cmd+y',
        event: 'menu-open-debug-yuv',
      }),

      // Recent YUV files
      createMenuItem({
        id: 'recent-yuv',
        text: 'Recent YUV files',
        event: 'menu-recent-yuv',
      }),

      // Close debug YUV
      createMenuItem({
        id: 'close-debug-yuv',
        text: 'Close debug YUV',
        event: 'menu-close-debug-yuv',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Subsampling (SUBMENU)
      createSubmenu({
        text: 'Subsampling',
        items: [
          createMenuItem({
            id: 'subsampling-planar',
            text: 'Planar (YUV420)',
            event: 'menu-subsampling-planar',
          }),
          createMenuItem({
            id: 'subsampling-interleaved',
            text: 'Interleaved (YUV422)',
            event: 'menu-subsampling-interleaved',
          }),
        ],
      }),

      // Display order
      createMenuItem({
        id: 'display-order',
        text: 'Display order',
        event: 'menu-display-order',
      }),

      // Decode order
      createMenuItem({
        id: 'decode-order',
        text: 'Decode order',
        event: 'menu-decode-order',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Stream crop values
      createMenuItem({
        id: 'stream-crop',
        text: 'Use stream crop values',
        event: 'menu-stream-crop',
      }),

      // Picture offset
      createMenuItem({
        id: 'picture-offset',
        text: 'Set picture offset here',
        event: 'menu-picture-offset',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Bitdepth options
      createMenuItem({
        id: 'bitdepth-stream',
        text: 'Use stream bitdepth',
        event: 'menu-bitdepth-stream',
      }),

      createMenuItem({
        id: 'bitdepth-max',
        text: 'Use max stream bitdepth',
        event: 'menu-bitdepth-max',
      }),

      createMenuItem({
        id: 'bitdepth-16',
        text: 'Use 16 bit bitdepth',
        event: 'menu-bitdepth-16',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Check for file changes
      createMenuItem({
        id: 'check-file-changes',
        text: 'Check for file changes',
        event: 'menu-check-file-changes',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // PSNR Map
      createMenuItem({
        id: 'show-psnr',
        text: 'Show PSNR Map',
        event: 'menu-show-psnr',
      }),

      // SSIM Map
      createMenuItem({
        id: 'show-ssim',
        text: 'Show SSIM Map',
        event: 'menu-show-ssim',
      }),

      // Delta Image
      createMenuItem({
        id: 'show-delta',
        text: 'Show Delta Image',
        event: 'menu-show-delta',
      }),

      // Separator
      PredefinedMenuItem.new({ item: 'Separator' }),

      // Export All Frames
      createMenuItem({
        id: 'export-all-frames',
        text: 'Export All Frames',
        event: 'menu-export-all-frames',
      }),

      // Export Metrics CSV
      createMenuItem({
        id: 'export-metrics',
        text: 'Export Metrics CSV...',
        event: 'menu-export-metrics',
      }),
    ],
  });
}
