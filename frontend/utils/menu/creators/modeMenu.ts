/**
 * Mode Menu Creator
 *
 * Creates the Mode menu (F1-F7 visualization modes):
 * - Overview (F1)
 * - Coding Flow (F2)
 * - Prediction (F3)
 * - Transform (F4)
 * - QP Map (F5)
 * - MV Field (F6)
 * - Reference Frames (F7)
 * - Extended Modes
 *
 * Reference: Mode Menu
 */

import { PredefinedMenuItem } from "@tauri-apps/api/menu";
import { createMenuItem, createSubmenu } from "../types";

export async function createModeMenu(): Promise<
  import("@tauri-apps/api/menu").Submenu
> {
  return await createSubmenu({
    text: "Mode",
    items: [
      // Overview (F1)
      createMenuItem({
        id: "overview",
        text: "Overview",
        accelerator: "f1",
        event: "menu-mode-change",
        eventDetail: "overview",
      }),

      // Coding Flow (F2)
      createMenuItem({
        id: "coding",
        text: "Coding Flow",
        accelerator: "f2",
        event: "menu-mode-change",
        eventDetail: "coding",
      }),

      // Prediction (F3)
      createMenuItem({
        id: "prediction",
        text: "Prediction",
        accelerator: "f3",
        event: "menu-mode-change",
        eventDetail: "prediction",
      }),

      // Transform (F4)
      createMenuItem({
        id: "transform",
        text: "Transform",
        accelerator: "f4",
        event: "menu-mode-change",
        eventDetail: "transform",
      }),

      // QP Map (F5)
      createMenuItem({
        id: "qp",
        text: "QP Map",
        accelerator: "f5",
        event: "menu-mode-change",
        eventDetail: "qp",
      }),

      // MV Field (F6)
      createMenuItem({
        id: "mv",
        text: "MV Field",
        accelerator: "f6",
        event: "menu-mode-change",
        eventDetail: "mv",
      }),

      // Reference Frames (F7)
      createMenuItem({
        id: "reference",
        text: "Reference Frames",
        accelerator: "f7",
        event: "menu-mode-change",
        eventDetail: "reference",
      }),

      // Separator
      PredefinedMenuItem.new({ item: "Separator" }),

      // Extended Modes
      createMenuItem({
        id: "ext-modes",
        text: "Extended Modes",
        event: "menu-mode-extended",
      }),
    ],
  });
}
