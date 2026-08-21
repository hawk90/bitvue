/**
 * useKeyboardNavigation Hook Tests
 *
 * Regression test for a real bug found via screenshot verification (2026-08-20): every
 * modifier-key shortcut in the "File operations"/"Go to frame"/"Selection" groups (Open,
 * Close, Export, Save, Reload, Go to Frame, Undo, Copy, Ctrl+Arrow I-frame jump) registered
 * with `ctrl: true, meta: true` together -- requiring Ctrl AND Cmd/Meta held simultaneously,
 * a chord no user would ever press. A real Electron screenshot confirmed plain ctrl+g left
 * "Go to Frame" completely unreachable (it has no button, only this shortcut) while
 * ctrl+meta+g opened it. Fixed to a single platform-conditional modifier via `isMac()`.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { useKeyboardNavigation } from "@/hooks/useKeyboardNavigation";
import * as keyboardShortcuts from "@/utils/keyboardShortcuts";

function baseCallbacks() {
  return {
    onPreviousFrame: vi.fn(),
    onNextFrame: vi.fn(),
    onFirstFrame: vi.fn(),
    onLastFrame: vi.fn(),
  };
}

function dispatchKeydown(key: string, opts: Partial<KeyboardEventInit> = {}) {
  // Dispatch on document.body (a real Element, has getAttribute) rather than window directly --
  // matches how a real keydown's target is always an in-page element, and how the Electron
  // screenshot harness itself dispatches these (document.activeElement || document.body).
  document.body.dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, ...opts }),
  );
}

describe("useKeyboardNavigation -- cross-platform modifier for Go to Frame etc.", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("on non-Mac, fires on plain Ctrl+G (not Ctrl+Meta+G)", () => {
    vi.spyOn(keyboardShortcuts, "isMac").mockReturnValue(false);
    const onGoToFrame = vi.fn();
    renderHook(() =>
      useKeyboardNavigation({
        currentIndex: 0,
        totalFrames: 10,
        callbacks: baseCallbacks(),
        onGoToFrame,
      }),
    );

    dispatchKeydown("g", { ctrlKey: true, metaKey: false });
    expect(onGoToFrame).toHaveBeenCalledTimes(1);
  });

  it("on non-Mac, does NOT require Meta held alongside Ctrl", () => {
    vi.spyOn(keyboardShortcuts, "isMac").mockReturnValue(false);
    const onGoToFrame = vi.fn();
    renderHook(() =>
      useKeyboardNavigation({
        currentIndex: 0,
        totalFrames: 10,
        callbacks: baseCallbacks(),
        onGoToFrame,
      }),
    );

    // The bug this regresses: requiring ctrl+meta together means plain ctrl+g (the real,
    // expected shortcut) did nothing.
    dispatchKeydown("g", { ctrlKey: true, metaKey: true });
    expect(onGoToFrame).not.toHaveBeenCalled();
  });

  it("on Mac, fires on plain Cmd+G (not Ctrl+Cmd+G)", () => {
    vi.spyOn(keyboardShortcuts, "isMac").mockReturnValue(true);
    const onGoToFrame = vi.fn();
    renderHook(() =>
      useKeyboardNavigation({
        currentIndex: 0,
        totalFrames: 10,
        callbacks: baseCallbacks(),
        onGoToFrame,
      }),
    );

    dispatchKeydown("g", { ctrlKey: false, metaKey: true });
    expect(onGoToFrame).toHaveBeenCalledTimes(1);
  });

  it("on Mac, does NOT require Ctrl held alongside Cmd", () => {
    vi.spyOn(keyboardShortcuts, "isMac").mockReturnValue(true);
    const onOpenFile = vi.fn();
    renderHook(() =>
      useKeyboardNavigation({
        currentIndex: 0,
        totalFrames: 10,
        callbacks: baseCallbacks(),
        onOpenFile,
      }),
    );

    dispatchKeydown("o", { ctrlKey: true, metaKey: true });
    expect(onOpenFile).not.toHaveBeenCalled();
  });
});
