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
}

export interface KeyboardNavigationOptions {
  /** Current frame index */
  currentIndex: number;
  /** Total number of frames */
  totalFrames: number;
  /** Navigation callbacks */
  callbacks: NavigationCallbacks;
  /** Optional: Show shortcuts callback */
  onShowShortcuts?: () => void;
}

/**
 * Hook for managing keyboard navigation shortcuts
 */
export function useKeyboardNavigation({
  currentIndex,
  totalFrames,
  callbacks,
  onShowShortcuts,
}: KeyboardNavigationOptions) {
  const { onPreviousFrame, onNextFrame, onFirstFrame, onLastFrame } = callbacks;

  // Keep numeric state in refs so the single effect closure always reads current values
  const currentIndexRef = useRef(currentIndex);
  const totalFramesRef = useRef(totalFrames);

  currentIndexRef.current = currentIndex;
  totalFramesRef.current = totalFrames;

  // Keep callbacks in a ref so the effect never needs to re-register on callback identity changes
  const callbacksRef = useRef({
    onPreviousFrame,
    onNextFrame,
    onFirstFrame,
    onLastFrame,
    onShowShortcuts,
  });
  callbacksRef.current = {
    onPreviousFrame,
    onNextFrame,
    onFirstFrame,
    onLastFrame,
    onShowShortcuts,
  };

  useEffect(() => {
    const shortcutUnregisters: Array<() => void> = [];

    // Register shortcuts — all callbacks are read via callbacksRef so this
    // effect only needs to run once (stable empty deps).
    const shortcuts: ShortcutConfig[] = [
      {
        key: "?",
        ctrl: true,
        meta: true,
        description: "Show shortcuts",
        action: () => {
          callbacksRef.current.onShowShortcuts?.();
        },
      },
      {
        key: "ArrowLeft",
        description: "Previous frame",
        action: () => {
          if (currentIndexRef.current > 0) {
            callbacksRef.current.onPreviousFrame();
          }
        },
      },
      {
        key: "ArrowRight",
        description: "Next frame",
        action: () => {
          if (
            totalFramesRef.current > 0 &&
            currentIndexRef.current < totalFramesRef.current - 1
          ) {
            callbacksRef.current.onNextFrame();
          }
        },
      },
      {
        key: "Home",
        description: "First frame",
        action: () => {
          callbacksRef.current.onFirstFrame();
        },
      },
      {
        key: "End",
        description: "Last frame",
        action: () => {
          if (totalFramesRef.current > 0) {
            callbacksRef.current.onLastFrame();
          }
        },
      },
    ];

    shortcuts.forEach((shortcut) => {
      shortcutUnregisters.push(globalShortcutHandler.register(shortcut));
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      globalShortcutHandler.handle(e);
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      shortcutUnregisters.forEach((fn) => fn());
    };
  }, []); // stable: never re-registers; callbacks read via refs
}

export default useKeyboardNavigation;
