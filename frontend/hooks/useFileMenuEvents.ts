/**
 * File menu events — open/open-as-codec/open-dependent/close/quit/export/export-evidence/
 * shortcuts/open-debug-yuv, plus the Recent Files submenu's "open this path" event. Extracted
 * from App.tsx's `AppContent` (2026-08-20, axis-4 cleanup) -- see `useOptionsMenuEvents`'s doc
 * for why this got split out from the rest of App.tsx's menu-event wiring. The two source
 * `useEffect`s (recent-files, file-menu) are combined here since both are squarely "File menu"
 * domain, same as `bitvue-desktop/electron/nativeMenu.ts`'s single "File" submenu covers both.
 */

import { useEffect } from "react";
import { closeWindow, showOpenDialog } from "../services/electronBridgeService";

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
  // Recent files menu: open the selected path
  useEffect(() => {
    const handleOpenRecent = (e: Event) => {
      const path = (e as CustomEvent<string>).detail;
      if (path) void openFileAtPath(path);
    };
    window.addEventListener("menu-open-recent-file", handleOpenRecent);
    return () => {
      window.removeEventListener("menu-open-recent-file", handleOpenRecent);
    };
  }, [openFileAtPath]);

  useEffect(() => {
    const handleExportListener = () => setShowExportDialog(true);
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
    const handleCloseBitstream = () => {
      void handleCloseFile();
    };
    const handleOpenDependentBitstream = () => {
      void handleOpenDependentFile();
    };
    const handleExportEvidence = () => {
      void exportEvidenceBundle();
    };
    const handleQuit = () => {
      void closeWindow();
    };
    const handleShowShortcuts = () => setShowShortcuts(true);

    window.addEventListener("menu-open-bitstream", handleOpenBitstream);
    window.addEventListener("menu-open-as-av1", handleOpenBitstream);
    window.addEventListener("menu-open-as-hevc", handleOpenBitstream);
    window.addEventListener("menu-open-as-avc", handleOpenBitstream);
    window.addEventListener("menu-open-as-vp9", handleOpenBitstream);
    window.addEventListener("menu-open-as-vvc", handleOpenBitstream);
    window.addEventListener("menu-open-as-mpeg2", handleOpenBitstream);
    window.addEventListener(
      "menu-open-dependent",
      handleOpenDependentBitstream,
    );
    window.addEventListener("menu-close-bitstream", handleCloseBitstream);
    window.addEventListener("menu-quit", handleQuit);
    window.addEventListener("menu-shortcuts", handleShowShortcuts);
    window.addEventListener("menu-export", handleExportListener);
    window.addEventListener("menu-export-evidence", handleExportEvidence);
    const handleOpenDebugYuvEvent = () => void handleOpenDebugYuv();
    window.addEventListener("menu-open-debug-yuv", handleOpenDebugYuvEvent);
    return () => {
      window.removeEventListener("menu-open-bitstream", handleOpenBitstream);
      window.removeEventListener("menu-open-as-av1", handleOpenBitstream);
      window.removeEventListener("menu-open-as-hevc", handleOpenBitstream);
      window.removeEventListener("menu-open-as-avc", handleOpenBitstream);
      window.removeEventListener("menu-open-as-vp9", handleOpenBitstream);
      window.removeEventListener("menu-open-as-vvc", handleOpenBitstream);
      window.removeEventListener("menu-open-as-mpeg2", handleOpenBitstream);
      window.removeEventListener(
        "menu-open-dependent",
        handleOpenDependentBitstream,
      );
      window.removeEventListener("menu-close-bitstream", handleCloseBitstream);
      window.removeEventListener("menu-quit", handleQuit);
      window.removeEventListener("menu-shortcuts", handleShowShortcuts);
      window.removeEventListener("menu-export", handleExportListener);
      window.removeEventListener("menu-export-evidence", handleExportEvidence);
      window.removeEventListener(
        "menu-open-debug-yuv",
        handleOpenDebugYuvEvent,
      );
    };
  }, [
    exportEvidenceBundle,
    handleCloseFile,
    handleOpenDependentFile,
    handleOpenFile,
    setShowExportDialog,
    setShowShortcuts,
    setPendingYuvPath,
  ]);
}
