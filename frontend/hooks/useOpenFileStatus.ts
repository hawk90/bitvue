/**
 * Open File Status Hook
 *
 * Reports "is there a real open bitstream right now" to the Electron main process, so the
 * native window-close / Quit paths (bitvue-desktop/electron/quitGuard.ts) can gate their
 * confirmation dialog on it. See electronBridgeService.ts's `setHasOpenFile` doc for why this
 * is the one *real* signal used, rather than a fabricated "unsaved work" flag.
 */

import { useEffect } from "react";
import { setHasOpenFile } from "../services/electronBridgeService";

export function useOpenFileStatus(isOpen: boolean): void {
  useEffect(() => {
    setHasOpenFile(isOpen);
    return () => setHasOpenFile(false);
  }, [isOpen]);
}
