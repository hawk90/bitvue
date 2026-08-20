/**
 * Sidecar process lifecycle events — currently just the crash-and-respawn notification (see
 * `services/bridge/windowLifecycle.ts`'s `onSidecarRestarted` doc). Unlike the `menu-*` hooks
 * (`useFileMenuEvents` etc.), this isn't a `window` CustomEvent — it's a direct
 * `window.bitvue.onSidecarRestarted` subscription, since it's a main-process push rather than
 * something the native menu dispatches.
 */

import { useEffect } from "react";
import { onSidecarRestarted } from "../services/electronBridgeService";

export interface UseSidecarLifecycleEventsParams {
  showErrorDialog: (
    title: string,
    message: string,
    details?: string,
    errorCode?: string,
  ) => void;
}

export function useSidecarLifecycleEvents({
  showErrorDialog,
}: UseSidecarLifecycleEventsParams): void {
  useEffect(() => {
    const unsubscribe = onSidecarRestarted(() => {
      showErrorDialog(
        "Analyzer process restarted",
        "The background analyzer process crashed and has been restarted. Any open bitstream was closed — please re-open your file.",
        undefined,
        "SIDECAR_RESTARTED",
      );
    });
    return unsubscribe;
  }, [showErrorDialog]);
}
