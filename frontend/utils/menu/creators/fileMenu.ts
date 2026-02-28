/**
 * File Menu Creator
 *
 * Creates the File menu with all submenus:
 * - Open bitstream...
 * - Open bitstream as... (AV1, HEVC, AVC, VP9, VVC, MPEG-2)
 * - Open dependent bitstream...
 * - Close bitstream
 * - Extract... (YUV frames, Prediction frames, Reconstruction frames, Transform coefficients)
 * - Recent Files
 * - Quit
 *
 * Reference: File Menu
 */

import { PredefinedMenuItem } from "@tauri-apps/api/menu";
import { createMenuItem, createSubmenu } from "../types";

export async function createFileMenu(): Promise<
  import("@tauri-apps/api/menu").Submenu
> {
  return await createSubmenu({
    text: "File",
    items: [
      // Open bitstream...
      createMenuItem({
        id: "open",
        text: "Open bitstream...",
        accelerator: "cmd+o",
        event: "menu-open-bitstream",
      }),

      // Open bitstream as... (SUBMENU)
      createSubmenu({
        text: "Open bitstream as...",
        items: [
          createMenuItem({
            id: "open-av1",
            text: "AV1",
            event: "menu-open-as-av1",
          }),
          createMenuItem({
            id: "open-hevc",
            text: "HEVC",
            event: "menu-open-as-hevc",
          }),
          createMenuItem({
            id: "open-avc",
            text: "AVC/H.264",
            event: "menu-open-as-avc",
          }),
          createMenuItem({
            id: "open-vp9",
            text: "VP9",
            event: "menu-open-as-vp9",
          }),
          createMenuItem({
            id: "open-vvc",
            text: "VVC/H.266",
            event: "menu-open-as-vvc",
          }),
          createMenuItem({
            id: "open-mpeg2",
            text: "MPEG-2",
            event: "menu-open-as-mpeg2",
          }),
        ],
      }),

      // Open dependent bitstream...
      createMenuItem({
        id: "open-dependent",
        text: "Open dependent bitstream...",
        event: "menu-open-dependent",
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // Close bitstream
      createMenuItem({
        id: "close",
        text: "Close bitstream",
        accelerator: "cmd+w",
        event: "menu-close-bitstream",
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // Extract... (SUBMENU)
      createSubmenu({
        text: "Extract...",
        items: [
          createMenuItem({
            id: "extract-yuv",
            text: "YUV frames",
            event: "menu-extract-yuv",
          }),
          createMenuItem({
            id: "extract-prediction",
            text: "Prediction frames",
            event: "menu-extract-prediction",
          }),
          createMenuItem({
            id: "extract-reconstruction",
            text: "Reconstruction frames",
            event: "menu-extract-reconstruction",
          }),
          createMenuItem({
            id: "extract-transform",
            text: "Transform coefficients",
            event: "menu-extract-transform",
          }),
        ],
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // Recent Files
      createMenuItem({
        id: "recent-files",
        text: "Recent Files",
        event: "menu-recent-files",
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // Quit
      createMenuItem({
        id: "quit",
        text: "Quit",
        accelerator: "cmd+q",
        event: "menu-quit",
      }),
    ],
  });
}
