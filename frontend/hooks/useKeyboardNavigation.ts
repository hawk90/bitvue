/**
 * Keyboard Navigation Hook
 *
 * Manages keyboard shortcuts for frame navigation in Bitvue.
 * Extracted from App.tsx for better separation of concerns.
 */

import { useEffect, useRef, useCallback } from "react";
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

  // Use refs to avoid re-registering shortcuts on every render
  const currentIndexRef = useRef(currentIndex);
  const totalFramesRef = useRef(totalFrames);
  const shortcutsRef = useRef<(() => void)[]>([]);

  currentIndexRef.current = currentIndex;
  totalFramesRef.current = totalFrames;

  useEffect(() => {
    // Cleanup previous shortcuts
    shortcutsRef.current.forEach((fn) => fn());
    shortcutsRef.current = [];

    // Register shortcuts
    const shortcuts: ShortcutConfig[] = [
      {
        key: "?",
        ctrl: true,
        meta: true,
        description: "Show shortcuts",
        action: onShowShortcuts || (() => {}),
      },
      {
        key: "ArrowLeft",
        description: "Previous frame",
        action: () => {
          if (currentIndexRef.current > 0) {
            onPreviousFrame();
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
            onNextFrame();
          }
        },
      },
      {
        key: "Home",
        description: "First frame",
        action: onFirstFrame,
      },
      {
        key: "End",
        description: "Last frame",
        action: () => {
          if (totalFramesRef.current > 0) {
            onLastFrame();
          }
        },
      },
    ];

    shortcuts.forEach((shortcut) => {
      shortcutsRef.current.push(globalShortcutHandler.register(shortcut));
    });

    // Handle keyboard events
    const handleKeyDown = (e: KeyboardEvent) => {
      globalShortcutHandler.handle(e);
    };

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      shortcutsRef.current.forEach((fn) => fn());
    };
  }, [
    onPreviousFrame,
    onNextFrame,
    onFirstFrame,
    onLastFrame,
    onShowShortcuts,
  ]);
}

export default useKeyboardNavigation;
