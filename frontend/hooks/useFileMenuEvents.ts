/**
 * File menu events — open/open-as-codec/open-dependent/close/quit/export/export-evidence/
 * shortcuts/open-debug-yuv, plus the Recent Files submenu's "open this path" event. Extracted
 * from App.tsx's `AppContent` (2026-08-20, axis-4 cleanup) -- see `useOptionsMenuEvents`'s doc
 * for why this got split out from the rest of App.tsx's menu-event wiring. The two source
 * `useEffect`s (recent-files, file-menu) are combined here since both are squarely "File menu"
 * domain, same as `bitvue-desktop/electron/nativeMenu.ts`'s single "File" submenu covers both.
 */

import { closeWindow, showOpenDialog } from "../services/electronBridgeService";
import { MENU_EVENTS } from "../../bitvue-desktop/electron/menuEvents";
import { useWindowEventMap } from "./useWindowEvent";

export interface UseFileMenuEventsParams {
  openFileAtPath: (path: string) => void | Promise<void>;
  handleOpenFile: () => void | Promise<void>;
  handleCloseFile: () => void | Promise<void>;
  handleOpenDependentFile: () => void | Promise<void>;
  exportEvidenceBundle: () => void | Promise<void>;
  setShowExportDialog: (show: boolean) => void;
  setShowShortcuts: (show: boolean) => void;
  setPendingYuvPath: (path: string | null) => void;
}

export function useFileMenuEvents({
  openFileAtPath,
  handleOpenFile,
  handleCloseFile,
  handleOpenDependentFile,
  exportEvidenceBundle,
  setShowExportDialog,
  setShowShortcuts,
  setPendingYuvPath,
}: UseFileMenuEventsParams): void {
  const handleOpenBitstream = () => {
    void handleOpenFile();
  };
  const handleOpenDebugYuv = async () => {
    const selected = await showOpenDialog([
      { name: "Raw YUV", extensions: ["yuv", "raw", "y4m"] },
    ]);
    if (!selected) return;
    // Show the load dialog for the user to confirm resolution / format / bitdepth
    setPendingYuvPath(selected);
  };

  useWindowEventMap({
    // Recent files menu: open the selected path
    "menu-open-recent-file": (e) => {
      const path = (e as CustomEvent<string>).detail;
      if (path) void openFileAtPath(path);
    },
    "menu-open-bitstream": handleOpenBitstream,
    "menu-open-as-av1": handleOpenBitstream,
    "menu-open-as-hevc": handleOpenBitstream,
    "menu-open-as-avc": handleOpenBitstream,
    "menu-open-as-vp9": handleOpenBitstream,
    "menu-open-as-vvc": handleOpenBitstream,
    "menu-open-as-mpeg2": handleOpenBitstream,
    "menu-open-dependent": () => {
      void handleOpenDependentFile();
    },
    "menu-close-bitstream": () => {
      void handleCloseFile();
    },
    "menu-quit": () => {
      void closeWindow();
    },
    [MENU_EVENTS.keyboardShortcuts]: () => setShowShortcuts(true),
    "menu-export": () => setShowExportDialog(true),
    "menu-export-evidence": () => {
      void exportEvidenceBundle();
    },
    "menu-open-debug-yuv": () => void handleOpenDebugYuv(),
  });
}
