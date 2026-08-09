/**
 * useExportEvidenceBundle
 *
 * Shared "choose a folder, write a diagnostic evidence bundle there" flow -- used by both the
 * MainMenu entry point (App.tsx's "menu-export-evidence" listener) and the Player context menu's
 * "Export Evidence Bundle" item. Two of the 4 documented entry points (BottomBar toolbar,
 * CompareWorkspace toolbar) aren't wired yet -- BottomBar has no export affordance today, and
 * CompareWorkspace is unmounted dead UI (see docs/DEVELOPMENT_PHASES.md's Phase 7.6 section).
 */

import { useCallback } from "react";
import {
  showDirectoryDialog,
  exportEvidenceBundle,
} from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("useExportEvidenceBundle");

export function useExportEvidenceBundle(): () => Promise<void> {
  return useCallback(async () => {
    const dir = await showDirectoryDialog();
    if (!dir) return;

    try {
      const result = await exportEvidenceBundle({
        outputDir: dir,
        workspace: "player",
        mode: "normal",
        orderType: "display",
      });
      if (result.success) {
        window.alert(`Evidence bundle exported to:\n${result.bundle_path}`);
      } else {
        window.alert(
          `Evidence bundle export failed: ${result.error ?? "unknown error"}`,
        );
      }
    } catch (err) {
      logger.warn("Evidence bundle export threw", err);
      window.alert(`Evidence bundle export failed: ${String(err)}`);
    }
  }, []);
}
