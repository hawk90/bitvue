/**
 * System Menu Setup
 *
 * Main entry point for initializing the native system menu on macOS.
 * Uses the modular menu creators to build the complete menu structure.
 *
 * On macOS, we use Tauri's native menu API which displays in the system menu bar.
 * On Windows/Linux, we use the custom TitleBar component instead.
 */

import { Menu, Submenu } from '@tauri-apps/api/menu';
import { shouldUseNativeMenu } from '../platform';
import { createLogger } from '../logger';
import { createMenuItem } from './types';
import { createFileMenu } from './creators/fileMenu';
import { createModeMenu } from './creators/modeMenu';
import { createYUVDiffMenu } from './creators/yuvDiffMenu';
import { createOptionsMenu } from './creators/optionsMenu';
import { createExportMenu } from './creators/exportMenu';
import { createViewMenu } from './creators/viewMenu';
import { createHelpMenu } from './creators/helpMenu';

const logger = createLogger('menuSetup');

/**
 * Initialize the native system menu (macOS only)
 */
export async function initializeSystemMenu(): Promise<void> {
  logger.debug('initializeSystemMenu called');

  // Only set up native menu on macOS
  if (!shouldUseNativeMenu()) {
    logger.debug('Skipping menu - not macOS');
    return;
  }

  logger.info('Setting up native menu for macOS');

  try {
    // Create About submenu (required as first submenu on macOS)
    const aboutSubmenu = await Submenu.new({
      text: 'About',
      items: [
        await createMenuItem({
          id: 'about',
          text: 'About Bitvue',
          event: 'menu-about',
        }),
      ],
    });

    // Create all menus using the modular creators
    const [
      fileSubmenu,
      modeSubmenu,
      yuvdiffSubmenu,
      optionsSubmenu,
      exportSubmenu,
      viewSubmenu,
      helpSubmenu,
    ] = await Promise.all([
      createFileMenu(),
      createModeMenu(),
      createYUVDiffMenu(),
      createOptionsMenu(),
      createExportMenu(),
      createViewMenu(),
      createHelpMenu(),
    ]);

    // Create the main menu
    const menu = await Menu.new({
      items: [
        aboutSubmenu,
        fileSubmenu,
        modeSubmenu,
        yuvdiffSubmenu,
        optionsSubmenu,
        exportSubmenu,
        viewSubmenu,
        helpSubmenu,
      ],
    });

    // Set as app menu for macOS system menu bar
    await menu.setAsAppMenu();
    logger.info('System menu initialized successfully');
  } catch (error) {
    logger.error('Failed to initialize system menu:', error);
  }
}
