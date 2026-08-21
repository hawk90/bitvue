/**
 * Welcome Actions Hook
 *
 * Wraps the raw "open a file" operations (dialog-driven or a known recent path) with an
 * `isOpening` guard spanning the *entire* operation -- dialog time, `openStream`, `selectFrame`,
 * `refreshFrames`, all of it. `useFileState().loading` (FileStateContext) only covers the
 * frame-loading sub-step of that pipeline, not the whole thing, so relying on it alone would
 * leave an early window (dialog showing, `openStream` in flight) where a double-click on "Open
 * Bitstream File" or a Recent item wasn't actually guarded against.
 *
 * Failure policy: a recent file that fails to *open* (wrong codec, corrupted, etc.) is NOT
 * removed from the list here -- `openFileAtPath` already surfaces that failure via the global
 * error dialog (see `useAppFileOperations`'s `onError`), and a failed-to-open file may still be a
 * perfectly valid path worth retrying (unlike `useValidatedRecentFiles`'s pruning, which is about
 * the path not existing on disk at all).
 */

import { useCallback, useState } from "react";

export interface UseWelcomeActionsReturn {
  /** True for the whole duration of any open attempt (dialog-driven or a recent path). */
  isOpening: boolean;
  handleOpenFile: () => Promise<void>;
  handleOpenRecent: (path: string) => Promise<void>;
}

export function useWelcomeActions(
  onOpenFile: () => Promise<void>,
  openFileAtPath: (path: string) => Promise<void>,
): UseWelcomeActionsReturn {
  const [isOpening, setIsOpening] = useState(false);

  const handleOpenFile = useCallback(async () => {
    if (isOpening) return;
    setIsOpening(true);
    try {
      await onOpenFile();
    } finally {
      setIsOpening(false);
    }
  }, [isOpening, onOpenFile]);

  const handleOpenRecent = useCallback(
    async (path: string) => {
      if (isOpening) return;
      setIsOpening(true);
      try {
        await openFileAtPath(path);
      } finally {
        setIsOpening(false);
      }
    },
    [isOpening, openFileAtPath],
  );

  return { isOpening, handleOpenFile, handleOpenRecent };
}
