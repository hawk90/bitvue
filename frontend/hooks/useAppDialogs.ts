/**
 * App Dialogs Hook
 *
 * Manages all dialog states for the Bitvue application.
 * Combines keyboard shortcuts, export, and error dialogs.
 */

import { useState } from "react";
import { useErrorDialog } from "./useErrorDialog";

export interface UseAppDialogsReturn {
  /** Show shortcuts dialog state */
  showShortcuts: boolean;
  /** Set show shortcuts dialog state */
  setShowShortcuts: (show: boolean) => void;
  /** Show export dialog state */
  showExportDialog: boolean;
  /** Set show export dialog state */
  setShowExportDialog: (show: boolean) => void;
  /** Error dialog state and methods */
  errorDialog: ReturnType<typeof useErrorDialog>['errorDialog'];
  /** Show error dialog */
  showErrorDialog: ReturnType<typeof useErrorDialog>['showErrorDialog'];
  /** Close error dialog */
  closeErrorDialog: ReturnType<typeof useErrorDialog>['closeErrorDialog'];
}

/**
 * Hook for managing all application dialog states
 */
export function useAppDialogs(): UseAppDialogsReturn {
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const errorDialog = useErrorDialog();

  return {
    showShortcuts,
    setShowShortcuts,
    showExportDialog,
    setShowExportDialog,
    errorDialog: errorDialog.errorDialog,
    showErrorDialog: errorDialog.showErrorDialog,
    closeErrorDialog: errorDialog.closeErrorDialog,
  };
}

export default useAppDialogs;
