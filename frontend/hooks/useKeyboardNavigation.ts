/**
 * Keyboard Navigation Hook
 *
 * Manages keyboard shortcuts for frame navigation in Bitvue.
 * Extracted from App.tsx for better separation of concerns.
 */

import { useEffect, useRef } from "react";
import {
  globalShortcutHandler,
  isMac,
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
  /** Optional: Reload current file */
  onReloadFile?: () => void;
  /** Optional: Toggle fullscreen */
  onToggleFullscreen?: () => void;
  /** Optional: Escape action (exit fullscreen / clear selection) */
  onEscape?: () => void;
  /** Optional: Undo last selection */
  onUndoSelection?: () => void;
  /** Optional: Copy selected block info to clipboard */
  onCopyBlockInfo?: () => void;
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
  onReloadFile,
  onToggleFullscreen,
  onEscape,
  onUndoSelection,
  onCopyBlockInfo,
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
    onReloadFile,
    onToggleFullscreen,
    onEscape,
    onUndoSelection,
    onCopyBlockInfo,
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
    onReloadFile,
    onToggleFullscreen,
    onEscape,
    onUndoSelection,
    onCopyBlockInfo,
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
      // Cross-platform single modifier -- not both at once (`ctrl: true, meta: true` together
      // requires literally holding Ctrl AND Cmd/Meta simultaneously, an unreachable chord no user
      // would press; confirmed via a real screenshot that plain ctrl+g left "Go to Frame" dead
      // while ctrl+meta+g opened it). isMac() picks the one modifier each platform actually uses.
      ctrl: !isMac(),
      meta: isMac(),
      description: "Previous I-frame",
      action: () => cbRef.current.onPreviousKeyFrame?.(),
    });
    reg({
      key: "ArrowRight",
      ctrl: !isMac(),
      meta: isMac(),
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
      ctrl: !isMac(),
      meta: isMac(),
      description: "Open file",
      action: () => cbRef.current.onOpenFile?.(),
    });
    reg({
      key: "w",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Close file",
      action: () => cbRef.current.onCloseFile?.(),
    });
    reg({
      key: "e",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Export",
      action: () => cbRef.current.onShowExport?.(),
    });
    reg({
      key: "s",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Save frame PNG",
      action: () => cbRef.current.onSaveFrame?.(),
    });

    // ── Go to frame ───────────────────────────────────────────────────────
    reg({
      key: "g",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Go to frame",
      action: () => cbRef.current.onGoToFrame?.(),
    });
    reg({
      key: "f",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Go to frame",
      action: () => cbRef.current.onGoToFrame?.(),
    });

    // ── Help ──────────────────────────────────────────────────────────────
    reg({
      key: "?",
      description: "Show keyboard shortcuts",
      action: () => cbRef.current.onShowShortcuts?.(),
    });

    // ── File reload ───────────────────────────────────────────────────────
    reg({
      key: "r",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Reload file",
      action: () => cbRef.current.onReloadFile?.(),
    });

    // ── Fullscreen ────────────────────────────────────────────────────────
    reg({
      key: "f",
      description: "Toggle fullscreen",
      action: () => cbRef.current.onToggleFullscreen?.(),
    });
    reg({
      key: "F11",
      description: "Toggle OS fullscreen",
      action: () => cbRef.current.onToggleFullscreen?.(),
    });
    reg({
      key: "Escape",
      description: "Exit fullscreen / clear selection",
      action: () => cbRef.current.onEscape?.(),
    });

    // ── Selection ─────────────────────────────────────────────────────────
    reg({
      key: "z",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Undo selection",
      action: () => cbRef.current.onUndoSelection?.(),
    });
    reg({
      key: "c",
      ctrl: !isMac(),
      meta: isMac(),
      description: "Copy block info",
      action: () => cbRef.current.onCopyBlockInfo?.(),
    });

    // ── Channel display (Y / U / V) ───────────────────────────────────────
    reg({
      key: "y",
      description: "Toggle Y channel display",
      action: () => window.dispatchEvent(new CustomEvent("viewer-channel-y")),
    });
    reg({
      key: "u",
      description: "Toggle U channel display",
      action: () => window.dispatchEvent(new CustomEvent("viewer-channel-u")),
    });
    reg({
      key: "v",
      description: "Toggle V channel display",
      action: () => window.dispatchEvent(new CustomEvent("viewer-channel-v")),
    });

    // ── Ctrl+F1–F6: Info Overlay toggles ─────────────────────────────────
    for (let n = 1; n <= 6; n++) {
      const fKey = n;
      reg({
        key: `F${fKey}`,
        ctrl: true,
        description: `Toggle info overlay F${fKey}`,
        action: () => {
          window.dispatchEvent(
            new CustomEvent("viewer-toggle-overlay-fkey", { detail: fKey }),
          );
        },
      });
    }

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
