/**
 * View Menu Creator
 *
 * Creates the View menu:
 * - Reset Layout
 * - Stream Tree
 * - Player
 * - Diagnostics
 * - Info Overlays (submenu) — codec-aware toggles handled in ModeContext
 *
 * Reference: View Menu
 */

import { PredefinedMenuItem } from "@tauri-apps/api/menu";
import { createMenuItem, createSubmenu } from "../types";

/** Menu items for the Info Overlays submenu.
 *  All codecs are listed; ModeContext.toggleOverlay silently ignores
 *  overlays that are unavailable for the current codec. */
async function createInfoOverlaysSubmenu() {
  return createSubmenu({
    text: "Info Overlays",
    items: [
      createMenuItem({
        id: "overlay-qp-map",
        text: "QP Map",
        event: "menu-toggle-overlay",
        eventDetail: "qp-map",
      }),
      createMenuItem({
        id: "overlay-heat-map",
        text: "Heat Map",
        event: "menu-toggle-overlay",
        eventDetail: "heat-map",
      }),
      createMenuItem({
        id: "overlay-mv-field",
        text: "MV Heat",
        event: "menu-toggle-overlay",
        eventDetail: "mv-field",
      }),
      createMenuItem({
        id: "overlay-pu-type",
        text: "PU Type",
        event: "menu-toggle-overlay",
        eventDetail: "pu-type",
      }),
      createMenuItem({
        id: "overlay-mb-type",
        text: "MB Type",
        event: "menu-toggle-overlay",
        eventDetail: "mb-type",
      }),
      createMenuItem({
        id: "overlay-ref-idx",
        text: "Ref Indices",
        event: "menu-toggle-overlay",
        eventDetail: "reference-indices",
      }),
      createMenuItem({
        id: "overlay-block-type",
        text: "Block Type",
        event: "menu-toggle-overlay",
        eventDetail: "block-type",
      }),
      createMenuItem({
        id: "overlay-efficiency",
        text: "Efficiency Map",
        event: "menu-toggle-overlay",
        eventDetail: "efficiency-map",
      }),
      createMenuItem({
        id: "overlay-inter-mem",
        text: "Inter Memory",
        event: "menu-toggle-overlay",
        eventDetail: "inter-memory",
      }),
      createMenuItem({
        id: "overlay-simple-mv",
        text: "Simple Motion",
        event: "menu-toggle-overlay",
        eventDetail: "simple-motion",
      }),
      PredefinedMenuItem.new({ item: "Separator" }),
      createMenuItem({
        id: "overlay-psnr",
        text: "PSNR Overlay",
        event: "menu-toggle-overlay",
        eventDetail: "psnr-overlay",
      }),
      createMenuItem({
        id: "overlay-ssim",
        text: "SSIM Overlay",
        event: "menu-toggle-overlay",
        eventDetail: "ssim-overlay",
      }),
      PredefinedMenuItem.new({ item: "Separator" }),
      createMenuItem({
        id: "overlay-clear-all",
        text: "Clear All Overlays",
        event: "menu-clear-overlays",
      }),
    ],
  });
}

export async function createViewMenu(): Promise<
  import("@tauri-apps/api/menu").Submenu
> {
  return await createSubmenu({
    text: "View",
    items: [
      // Reset Layout
      createMenuItem({
        id: "reset-layout",
        text: "Reset Layout",
        event: "menu-reset-layout",
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // Stream Tree
      createMenuItem({
        id: "show-stream-tree",
        text: "Stream Tree",
        event: "menu-toggle-stream-tree",
      }),

      // Player
      createMenuItem({
        id: "show-player",
        text: "Player",
        event: "menu-toggle-player",
      }),

      // Diagnostics
      createMenuItem({
        id: "show-diagnostics",
        text: "Diagnostics",
        event: "menu-toggle-diagnostics",
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // Info Overlays submenu
      createInfoOverlaysSubmenu(),
    ],
  });
}
