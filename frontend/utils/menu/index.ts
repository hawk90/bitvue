/**
 * Menu Module Index
 *
 * Main export point for the menu system.
 * Re-exports all menu creators and utilities.
 */

// Main setup function
export { initializeSystemMenu } from "./setup";

// Type definitions
export {
  type MenuItemResult,
  type MenuItemConfig,
  type MenuItemCreator,
  type SubmenuItemConfig,
  type SubmenuCreator,
  dispatchMenuEvent,
  createMenuItem,
  createSubmenu,
} from "./types";

// Menu creators
export { createFileMenu } from "./creators/fileMenu";
export { createModeMenu } from "./creators/modeMenu";
export { createYUVDiffMenu } from "./creators/yuvDiffMenu";
export { createOptionsMenu } from "./creators/optionsMenu";
export { createExportMenu } from "./creators/exportMenu";
export { createViewMenu } from "./creators/viewMenu";
export { createHelpMenu } from "./creators/helpMenu";
