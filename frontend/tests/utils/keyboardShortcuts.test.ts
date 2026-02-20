/**
 * Keyboard Shortcuts Utility Tests
 * Tests keyboard shortcut matching, display, and handler
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  KEYBOARD_SHORTCUTS,
  matchesShortcut,
  getShortcutDisplay,
  isMac,
  KeyboardShortcutHandler,
  globalShortcutHandler,
} from "@/utils/keyboardShortcuts";

// Mock navigator.platform for platform detection
const originalPlatform = Object.getOwnPropertyDescriptor(navigator, "platform");

describe("KEYBOARD_SHORTCUTS", () => {
  it("should have all categories", () => {
    const categories = KEYBOARD_SHORTCUTS.map((s) => s.name);
    expect(categories).toContain("Navigation");
    expect(categories).toContain("Playback");
    expect(categories).toContain("View");
    expect(categories).toContain("Visualization Modes");
    expect(categories).toContain("Application");
  });

  it("should have navigation shortcuts", () => {
    const nav = KEYBOARD_SHORTCUTS.find((c) => c.name === "Navigation");
    expect(nav).toBeDefined();
    expect(nav?.shortcuts).toHaveLength(12);
  });

  it("should have playback shortcuts", () => {
    const playback = KEYBOARD_SHORTCUTS.find((c) => c.name === "Playback");
    expect(playback).toBeDefined();
    expect(playback?.shortcuts).toHaveLength(8);
  });

  it("should have F1-F10 mode shortcuts", () => {
    const modes = KEYBOARD_SHORTCUTS.find(
      (c) => c.name === "Visualization Modes",
    );
    expect(modes?.shortcuts).toHaveLength(10);
  });
});

describe("matchesShortcut", () => {
  it("should match simple key shortcut", () => {
    const event = new KeyboardEvent("keydown", { key: "a" });
    const shortcut = { key: "a", description: "Test", action: () => {} };

    expect(matchesShortcut(event, shortcut)).toBe(true);
  });

  it("should match Ctrl key shortcut", () => {
    const event = new KeyboardEvent("keydown", { key: "o", ctrlKey: true });
    const shortcut = {
      key: "o",
      ctrl: true,
      description: "Test",
      action: () => {},
    };

    expect(matchesShortcut(event, shortcut)).toBe(true);
  });

  it("should match Meta key shortcut", () => {
    const event = new KeyboardEvent("keydown", { key: "o", metaKey: true });
    const shortcut = {
      key: "o",
      meta: true,
      description: "Test",
      action: () => {},
    };

    expect(matchesShortcut(event, shortcut)).toBe(true);
  });

  it("should match Shift key shortcut", () => {
    const event = new KeyboardEvent("keydown", { key: "A", shiftKey: true });
    const shortcut = {
      key: "A",
      shift: true,
      description: "Test",
      action: () => {},
    };

    expect(matchesShortcut(event, shortcut)).toBe(true);
  });

  it("should match Alt key shortcut", () => {
    const event = new KeyboardEvent("keydown", { key: "Tab", altKey: true });
    const shortcut = {
      key: "Tab",
      alt: true,
      description: "Test",
      action: () => {},
    };

    expect(matchesShortcut(event, shortcut)).toBe(true);
  });

  it("should not match when modifiers differ", () => {
    const event = new KeyboardEvent("keydown", { key: "o", ctrlKey: false });
    const shortcut = {
      key: "o",
      ctrl: true,
      description: "Test",
      action: () => {},
    };

    expect(matchesShortcut(event, shortcut)).toBe(false);
  });

  it("should match complex modifier combinations", () => {
    const event = new KeyboardEvent("keydown", {
      key: "0",
      ctrlKey: true,
      metaKey: true,
    });
    const shortcut = {
      key: "0",
      ctrl: true,
      meta: true,
      description: "Test",
      action: () => {},
    };

    expect(matchesShortcut(event, shortcut)).toBe(true);
  });

  it("should match Space key", () => {
    const event = new KeyboardEvent("keydown", { key: " " });
    const shortcut = { key: " ", description: "Test", action: () => {} };

    expect(matchesShortcut(event, shortcut)).toBe(true);
  });
});

describe("getShortcutDisplay", () => {
  beforeEach(() => {
    // Mock Windows platform
    Object.defineProperty(navigator, "platform", {
      value: "Win32",
      writable: true,
      configurable: true,
    });
  });

  afterEach(() => {
    // Restore original platform
    if (originalPlatform) {
      Object.defineProperty(navigator, "platform", originalPlatform);
    }
  });

  it("should display simple key", () => {
    const shortcut = { key: "a", description: "Test", action: () => {} };
    expect(getShortcutDisplay(shortcut)).toBe("a");
  });

  it("should display Ctrl+key on Windows", () => {
    const shortcut = {
      key: "o",
      ctrl: true,
      description: "Test",
      action: () => {},
    };
    expect(getShortcutDisplay(shortcut)).toBe("Ctrl+o");
  });

  it("should display Space key", () => {
    const shortcut = { key: " ", description: "Test", action: () => {} };
    expect(getShortcutDisplay(shortcut)).toBe("Space");
  });

  it("should display arrow keys", () => {
    const leftShortcut = {
      key: "ArrowLeft",
      description: "Test",
      action: () => {},
    };
    const rightShortcut = {
      key: "ArrowRight",
      description: "Test",
      action: () => {},
    };

    expect(getShortcutDisplay(leftShortcut)).toBe("←");
    expect(getShortcutDisplay(rightShortcut)).toBe("→");
  });

  it("should display Page Up/Down", () => {
    const pageUpShortcut = {
      key: "PageUp",
      description: "Test",
      action: () => {},
    };
    const pageDownShortcut = {
      key: "PageDown",
      description: "Test",
      action: () => {},
    };

    expect(getShortcutDisplay(pageUpShortcut)).toBe("Page Up");
    expect(getShortcutDisplay(pageDownShortcut)).toBe("Page Down");
  });

  it("should display multiple modifiers", () => {
    const shortcut = {
      key: "0",
      ctrl: true,
      meta: true,
      shift: true,
      description: "Test",
      action: () => {},
    };
    expect(getShortcutDisplay(shortcut)).toBe("Ctrl+Win+Shift+0");
  });

  it("should display Mac-style shortcuts on macOS", () => {
    Object.defineProperty(navigator, "platform", {
      value: "MacIntel",
      writable: true,
      configurable: true,
    });

    const shortcut = {
      key: "o",
      ctrl: true,
      description: "Test",
      action: () => {},
    };
    expect(getShortcutDisplay(shortcut)).toBe("⌘o");
  });
});

describe("isMac", () => {
  afterEach(() => {
    if (originalPlatform) {
      Object.defineProperty(navigator, "platform", originalPlatform);
    }
  });

  it("should return true on macOS", () => {
    Object.defineProperty(navigator, "platform", {
      value: "MacIntel",
      writable: true,
      configurable: true,
    });

    expect(isMac()).toBe(true);
  });

  it("should return true on iPhone", () => {
    Object.defineProperty(navigator, "platform", {
      value: "iPhone",
      writable: true,
      configurable: true,
    });

    expect(isMac()).toBe(true);
  });

  it("should return false on Windows", () => {
    Object.defineProperty(navigator, "platform", {
      value: "Win32",
      writable: true,
      configurable: true,
    });

    expect(isMac()).toBe(false);
  });

  it("should return false on Linux", () => {
    Object.defineProperty(navigator, "platform", {
      value: "Linux x86_64",
      writable: true,
      configurable: true,
    });

    expect(isMac()).toBe(false);
  });
});

describe("KeyboardShortcutHandler", () => {
  let handler: KeyboardShortcutHandler;

  beforeEach(() => {
    handler = new KeyboardShortcutHandler();
  });

  it("should register a shortcut", () => {
    const action = vi.fn();
    const unregister = handler.register({
      key: "a",
      description: "Test",
      action,
    });

    expect(typeof unregister).toBe("function");
  });

  it("should handle registered shortcut", () => {
    const action = vi.fn();
    handler.register({
      key: "a",
      description: "Test",
      action,
    });

    const event = new KeyboardEvent("keydown", { key: "a" });
    const handled = handler.handle(event);

    expect(handled).toBe(true);
    expect(action).toHaveBeenCalled();
  });

  it("should not handle unregistered shortcut", () => {
    const event = new KeyboardEvent("keydown", { key: "x" });
    const handled = handler.handle(event);

    expect(handled).toBe(false);
  });

  it("should not handle when disabled", () => {
    const action = vi.fn();
    handler.register({
      key: "a",
      description: "Test",
      action,
    });

    handler.setEnabled(false);

    const event = new KeyboardEvent("keydown", { key: "a" });
    const handled = handler.handle(event);

    expect(handled).toBe(false);
    expect(action).not.toHaveBeenCalled();
  });

  it("should unregister shortcut", () => {
    const action = vi.fn();
    const unregister = handler.register({
      key: "a",
      description: "Test",
      action,
    });

    unregister();

    const event = new KeyboardEvent("keydown", { key: "a" });
    const handled = handler.handle(event);

    expect(handled).toBe(false);
    expect(action).not.toHaveBeenCalled();
  });

  it("should get all registered shortcuts", () => {
    handler.register({ key: "a", description: "A", action: () => {} });
    handler.register({ key: "b", description: "B", action: () => {} });

    const all = handler.getAllShortcuts();
    expect(all).toHaveLength(2);
  });

  it("should clear all shortcuts", () => {
    handler.register({ key: "a", description: "A", action: () => {} });
    handler.register({ key: "b", description: "B", action: () => {} });

    handler.clear();

    const all = handler.getAllShortcuts();
    expect(all).toHaveLength(0);
  });

  it("should prevent default when handling", () => {
    const action = vi.fn();
    handler.register({
      key: "a",
      description: "Test",
      action,
    });

    const event = new KeyboardEvent("keydown", { key: "a" });
    const preventDefault = vi.spyOn(event, "preventDefault");

    handler.handle(event);

    expect(preventDefault).toHaveBeenCalled();
  });
});

describe("KeyboardShortcutHandler edge cases", () => {
  it("should not handle in input fields", () => {
    const handler = new KeyboardShortcutHandler();
    const action = vi.fn();
    handler.register({
      key: "a",
      description: "Test",
      action,
    });

    const input = document.createElement("input");
    document.body.appendChild(input);

    const event = new KeyboardEvent("keydown", { key: "a" });
    Object.defineProperty(event, "target", { value: input, writable: true });

    const handled = handler.handle(event);

    expect(handled).toBe(false);
    expect(action).not.toHaveBeenCalled();

    document.body.removeChild(input);
  });

  it("should not handle in textarea", () => {
    const handler = new KeyboardShortcutHandler();
    const action = vi.fn();
    handler.register({
      key: "a",
      description: "Test",
      action,
    });

    const textarea = document.createElement("textarea");
    document.body.appendChild(textarea);

    const event = new KeyboardEvent("keydown", { key: "a" });
    Object.defineProperty(event, "target", { value: textarea, writable: true });

    const handled = handler.handle(event);

    expect(handled).toBe(false);

    document.body.removeChild(textarea);
  });

  it("should not handle in contentEditable elements", () => {
    const handler = new KeyboardShortcutHandler();
    const action = vi.fn();
    handler.register({
      key: "a",
      description: "Test",
      action,
    });

    const div = document.createElement("div");
    div.setAttribute("contenteditable", "true");
    document.body.appendChild(div);

    let handled = false;
    const eventCallback = (e: Event) => {
      handled = handler.handle(e as KeyboardEvent);
      e.stopImmediatePropagation();
    };

    div.addEventListener("keydown", eventCallback);
    div.dispatchEvent(
      new KeyboardEvent("keydown", { key: "a", bubbles: true }),
    );
    div.removeEventListener("keydown", eventCallback);

    expect(handled).toBe(false);

    document.body.removeChild(div);
  });
});

describe("globalShortcutHandler", () => {
  it("should be a singleton instance", () => {
    expect(globalShortcutHandler).toBeInstanceOf(KeyboardShortcutHandler);
  });
});
