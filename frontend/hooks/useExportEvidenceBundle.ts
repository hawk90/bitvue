/**
 * useExportEvidenceBundle
 *
 * Shared "write a diagnostic evidence bundle into a folder the user picks" flow -- the folder
 * dialog is shown by the main process inside `exportEvidenceBundle` (the renderer never supplies a
 * path, #640). Used by the MainMenu
 * entry point (App.tsx's "menu-export-evidence" listener), the Player/HexView/Timeline/
 * DiagnosticsPanel context menus' "Export Evidence Bundle" item, and CompareWorkspace's toolbar
 * "Export Diff Bundle" button (`docs/UX_PARITY_MATRIX.md` §7 lists all 4 as the same
 * `Export.EvidenceBundle` command). BottomBar still has no export affordance today (that gap is
 * unrelated to this hook).
 *
 * `workspace`/`mode` are free-form manifest metadata strings (see
 * `bitvue_engine::export::evidence::EvidenceBundleManifest`), not a guarded enum -- every caller
 * so far has used the default single-stream "player"/"normal", but CompareWorkspace passes
 * "compare"/"diff" (or "compare"/"normal" when the diff overlay is off) so the exported
 * `bundle_manifest.json` records which workspace and mode the bundle was captured from.
 */

import { useCallback } from "react";
import {
  exportEvidenceBundle,
  captureScreenshot,
} from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("useExportEvidenceBundle");

export interface UseExportEvidenceBundleOptions {
  /** Manifest `workspace` field. Defaults to `"player"`. */
  workspace?: string;
  /** Manifest `mode` field. Defaults to `"normal"`. */
  mode?: string;
}

export function useExportEvidenceBundle(
  options?: UseExportEvidenceBundleOptions,
): () => Promise<void> {
  const workspace = options?.workspace ?? "player";
  const mode = options?.mode ?? "normal";

  return useCallback(async () => {
    try {
      // Captured before the folder dialog opens, so it shows the app as it was when the user
      // asked for the export. Best-effort: a bundle without a screenshot is still useful, so a
      // capture failure shouldn't block the export itself.
      const screenshotDataUrl = await captureScreenshot().catch((err) => {
        logger.warn("Screenshot capture failed, exporting without one", err);
        return null;
      });

      const result = await exportEvidenceBundle({
        workspace,
        mode,
        orderType: "display",
        screenshotDataUrl: screenshotDataUrl ?? undefined,
      });
      // The user cancelled the folder dialog: nothing was written, nothing to report.
      if (result === null) return;
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
  }, [workspace, mode]);
}
