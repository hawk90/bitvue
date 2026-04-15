/**
 * Keyboard Navigation Hook
 *
 * Manages keyboard shortcuts for frame navigation in Bitvue.
 * Extracted from App.tsx for better separation of concerns.
 */

import { useEffect, useRef } from "react";
import {
  globalShortcutHandler,
  type ShortcutConfig,
} from "../utils/keyboardShortcuts";

export interface NavigationCallbacks {
  onPreviousFrame: () => void;
  onNextFrame: () => void;
  onFirstFrame: () => void;
  onLastFrame: () => void;
  /** Jump to previous I/key frame */
  onPreviousKeyFrame?: () => void;
  /** Jump to next I/key frame */
  onNextKeyFrame?: () => void;
}

export interface KeyboardNavigationOptions {
  /** Current frame index */
  currentIndex: number;
  /** Total number of frames */
  totalFrames: number;
  /** Navigation callbacks */
  callbacks: NavigationCallbacks;
  /** Optional: Show shortcuts dialog */
  onShowShortcuts?: () => void;
  /** Optional: Show go-to-frame dialog */
  onGoToFrame?: () => void;
  /** Optional: Open file */
  onOpenFile?: () => void;
  /** Optional: Close file */
  onCloseFile?: () => void;
  /** Optional: Show export dialog */
  onShowExport?: () => void;
  /** Optional: Save current frame as PNG */
  onSaveFrame?: () => void;
  /** Optional: Handle F-key mode switch (1-12). Returns true if handled. */
  onFKey?: (fKey: number) => boolean;
}

/**
 * Hook for managing keyboard navigation shortcuts.
 *
 * All callbacks are stored in refs so the single effect registers once
 * and always sees the latest callback values without re-registering.
 */
export function useKeyboardNavigation({
  currentIndex,
  totalFrames,
  callbacks,
  onShowShortcuts,
  onGoToFrame,
  onOpenFile,
  onCloseFile,
  onShowExport,
  onSaveFrame,
  onFKey,
}: KeyboardNavigationOptions) {
  // Keep numeric state in refs so the single effect closure always reads current values
  const currentIndexRef = useRef(currentIndex);
  const totalFramesRef = useRef(totalFrames);
  currentIndexRef.current = currentIndex;
  totalFramesRef.current = totalFrames;

  // Keep all callbacks in a ref so the effect never needs to re-register
  const cbRef = useRef({
    ...callbacks,
    onShowShortcuts,
    onGoToFrame,
    onOpenFile,
    onCloseFile,
    onShowExport,
    onSaveFrame,
    onFKey,
  });
  cbRef.current = {
    ...callbacks,
    onShowShortcuts,
    onGoToFrame,
    onOpenFile,
    onCloseFile,
    onShowExport,
    onSaveFrame,
    onFKey,
  };

  useEffect(() => {
    const unregs: Array<() => void> = [];
    const reg = (s: ShortcutConfig) =>
      unregs.push(globalShortcutHandler.register(s));

    // ── Frame navigation ──────────────────────────────────────────────────
    reg({
      key: "ArrowLeft",
      description: "Previous frame",
      action: () => {
        if (currentIndexRef.current > 0) cbRef.current.onPreviousFrame();
      },
    });
    reg({
      key: "ArrowRight",
      description: "Next frame",
      action: () => {
        if (currentIndexRef.current < totalFramesRef.current - 1)
          cbRef.current.onNextFrame();
      },
    });
    // Space = next frame
    reg({
      key: " ",
      description: "Next frame",
      action: () => {
        if (currentIndexRef.current < totalFramesRef.current - 1)
          cbRef.current.onNextFrame();
      },
    });
    reg({
      key: "Home",
      description: "First frame",
      action: () => cbRef.current.onFirstFrame(),
    });
    reg({
      key: "End",
      description: "Last frame",
      action: () => {
        if (totalFramesRef.current > 0) cbRef.current.onLastFrame();
      },
    });

    // Ctrl+← / Ctrl+→  — jump to previous/next I-frame
    reg({
      key: "ArrowLeft",
      ctrl: true,
      meta: true,
      description: "Previous I-frame",
      action: () => cbRef.current.onPreviousKeyFrame?.(),
    });
    reg({
      key: "ArrowRight",
      ctrl: true,
      meta: true,
      description: "Next I-frame",
      action: () => cbRef.current.onNextKeyFrame?.(),
    });

    // [ / ]  — also jump to previous/next I-frame (VQA parity)
    reg({
      key: "[",
      description: "Previous I-frame",
      action: () => cbRef.current.onPreviousKeyFrame?.(),
    });
    reg({
      key: "]",
      description: "Next I-frame",
      action: () => cbRef.current.onNextKeyFrame?.(),
    });

    // ── File operations ───────────────────────────────────────────────────
    reg({
      key: "o",
      ctrl: true,
      meta: true,
      description: "Open file",
      action: () => cbRef.current.onOpenFile?.(),
    });
    reg({
      key: "w",
      ctrl: true,
      meta: true,
      description: "Close file",
      action: () => cbRef.current.onCloseFile?.(),
    });
    reg({
      key: "e",
      ctrl: true,
      meta: true,
      description: "Export",
      action: () => cbRef.current.onShowExport?.(),
    });
    reg({
      key: "s",
      ctrl: true,
      meta: true,
      description: "Save frame PNG",
      action: () => cbRef.current.onSaveFrame?.(),
    });

    // ── Go to frame ───────────────────────────────────────────────────────
    reg({
      key: "g",
      ctrl: true,
      meta: true,
      description: "Go to frame",
      action: () => cbRef.current.onGoToFrame?.(),
    });
    reg({
      key: "f",
      ctrl: true,
      meta: true,
      description: "Go to frame",
      action: () => cbRef.current.onGoToFrame?.(),
    });

    // ── Help ──────────────────────────────────────────────────────────────
    reg({
      key: "?",
      description: "Show keyboard shortcuts",
      action: () => cbRef.current.onShowShortcuts?.(),
    });

    // ── F-key mode switching (F1–F12) ─────────────────────────────────────
    // Codec-specific: ModeContext maps each F-key to a visualization mode.
    // We register all 12 keys; getModeByFKey returns null if not mapped for
    // the current codec, so unhandled keys fall through silently.
    for (let n = 1; n <= 12; n++) {
      const fKey = n; // capture for closure
      reg({
        key: `F${fKey}`,
        description: `Mode F${fKey}`,
        action: () => cbRef.current.onFKey?.(fKey),
      });
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      globalShortcutHandler.handle(e);
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      unregs.forEach((fn) => fn());
    };
  }, []); // stable: registers once, reads current values via refs
}

export default useKeyboardNavigation;
