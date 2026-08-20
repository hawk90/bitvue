/**
 * Evidence/export domain — diagnostic evidence bundle export and the context-menu guard catalog
 * (grouped here rather than a separate file since both are small and both back the export/
 * diagnostics UI chrome). See `services/bridge/core.ts`'s module doc for how this file fits into
 * the overall bridge split.
 */

import { requireBridge } from "./core";

/** Matches `bitvue_engine::export::types::ContextMenuScope` (5 variants) -- serialized as the
 *  bare variant name (no `serde(rename_all)` on the Rust side). */
export type ContextMenuScopeWire =
  | "Player"
  | "HexView"
  | "StreamView"
  | "Timeline"
  | "DiagnosticsPanel";

export interface ContextMenuItemWire {
  id: string;
  label: string;
  command: string;
  guard: string;
  enabled: boolean;
  disabled_reason: string | null;
}

export interface EvidenceBundleExportResultWire {
  success: boolean;
  bundle_path: string | null;
  files_created: string[];
  total_bytes: number;
  error: string | null;
}

export interface ExportEvidenceBundleParams {
  outputDir: string;
  workspace?: string;
  mode?: string;
  orderType?: "display" | "decode";
  /** `data:image/png;base64,...` from `captureScreenshot()` -- written into the bundle's
   *  `screenshots/` directory if present. Omit to export without a screenshot. */
  screenshotDataUrl?: string;
}

/** Right-click menu items + guard-evaluated enabled/disabled state for one UI scope. Throws on
 *  failure (invalid scope). */
export async function getContextMenuItems(
  scope: ContextMenuScopeWire,
  hasSelection: boolean,
  hasByteRange: boolean,
): Promise<ContextMenuItemWire[]> {
  const result = await requireBridge().getContextMenuItems(
    scope,
    hasSelection,
    hasByteRange,
  );
  return result.items;
}

/** Writes a diagnostic evidence bundle (manifest, env/version info, selection state, order type,
 *  backend fingerprint, warnings) to `params.outputDir`. Throws on failure (write error). */
export async function exportEvidenceBundle(
  params: ExportEvidenceBundleParams,
): Promise<EvidenceBundleExportResultWire> {
  return requireBridge().exportEvidenceBundle(params);
}

/** Native "choose a folder" dialog, for evidence-bundle export. Returns the selected directory,
 *  or null if cancelled. */
export async function showDirectoryDialog(): Promise<string | null> {
  return requireBridge().showDirectoryDialog();
}

/** Captures the current window as a `data:image/png;base64,...` string, for embedding in an
 *  evidence bundle's `screenshots/` directory. Null if there's no window to capture from. */
export async function captureScreenshot(): Promise<string | null> {
  return requireBridge().captureScreenshot();
}
