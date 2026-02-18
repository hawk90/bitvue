/**
 * Error Dialog Hook
 *
 * Manages error dialog state for displaying user-friendly error messages.
 * Extracted from App.tsx for better separation of concerns.
 */

import { useState, useCallback } from "react";

export interface ErrorDialogState {
  isOpen: boolean;
  title: string;
  message: string;
  details?: string;
  errorCode?: string;
}

export interface UseErrorDialogReturn {
  /** Current error dialog state */
  errorDialog: ErrorDialogState;
  /** Show an error dialog with the given details */
  showErrorDialog: (
    title: string,
    message: string,
    details?: string,
    errorCode?: string,
  ) => void;
  /** Close the error dialog */
  closeErrorDialog: () => void;
}

/**
 * Hook for managing error dialog state
 */
export function useErrorDialog(): UseErrorDialogReturn {
  const [errorDialog, setErrorDialog] = useState<ErrorDialogState>({
    isOpen: false,
    title: "",
    message: "",
  });

  /**
   * Show error dialog with the given details
   */
  const showErrorDialog = useCallback(
    (title: string, message: string, details?: string, errorCode?: string) => {
      setErrorDialog({
        isOpen: true,
        title,
        message,
        details,
        errorCode,
      });
    },
    [],
  );

  /**
   * Close the error dialog
   */
  const closeErrorDialog = useCallback(() => {
    setErrorDialog((prev) => ({ ...prev, isOpen: false }));
  }, []);

  return {
    errorDialog,
    showErrorDialog,
    closeErrorDialog,
  };
}

export default useErrorDialog;
