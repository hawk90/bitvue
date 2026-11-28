import "@testing-library/jest-dom";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";
import React from "react";
import { setLogLevel } from "../utils/logger";

// The app logger defaults to "info" under vitest, so routine info/debug lines (HexViewTab,
// FileStateContext, yuv renderer, ...) flood stdout. Warnings and errors still print; tests of
// the logger itself set their own level.
setLogLevel("warn");

// Clean up after each test
afterEach(() => {
  cleanup();
});

// Mock react-resizable-panels to avoid DOM issues in jsdom
vi.mock("react-resizable-panels", () => ({
  Group: ({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) =>
    React.createElement(
      "div",
      { "data-testid": "resizable-group", ...props },
      children,
    ),
  Panel: ({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) =>
    React.createElement(
      "div",
      { "data-testid": "resizable-panel", ...props },
      children,
    ),
  PanelResizeHandle: ({
    children,
    ...props
  }: React.HTMLAttributes<HTMLDivElement>) =>
    React.createElement(
      "div",
      { "data-testid": "resize-handle", ...props },
      children,
    ),
  Separator: ({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) =>
    React.createElement(
      "div",
      { "data-testid": "resizable-separator", ...props },
      children,
    ),
}));

// Mock scrollIntoView for jsdom (not implemented in jsdom)
HTMLElement.prototype.scrollIntoView = vi.fn();

// Mock ClipboardItem for jsdom (not implemented in jsdom)
if (typeof ClipboardItem === "undefined") {
  global.ClipboardItem = class MockClipboardItem {
    constructor(data: Record<string, Blob>) {
      Object.assign(this, data);
    }
  } as unknown as typeof ClipboardItem;
}

// Mock ImageData for jsdom (no real Canvas API implementation) -- needed by any test exercising
// frontend/utils/yuv/renderer.ts's yuvToImageData/YUVRenderer (real Electron/Chromium has a
// real ImageData, this is jsdom-only).
if (typeof ImageData === "undefined") {
  global.ImageData = class MockImageData {
    data: Uint8ClampedArray;
    width: number;
    height: number;
    constructor(width: number, height: number) {
      this.width = width;
      this.height = height;
      this.data = new Uint8ClampedArray(width * height * 4);
    }
  } as unknown as typeof ImageData;
}

// Mock IntersectionObserver for components that use it.
// vitest 4 only lets a mock be called with `new` when its implementation is
// constructible, so these observer mocks use `function`, not arrow functions.
const mockIntersectionObserver = vi.fn().mockImplementation(function () {
  return {
    observe: vi.fn(),
    unobserve: vi.fn(),
    disconnect: vi.fn(),
    takeRecords: vi.fn().mockReturnValue([]),
  };
});
Object.defineProperty(window, "IntersectionObserver", {
  writable: true,
  configurable: true,
  value: mockIntersectionObserver,
});
global.IntersectionObserver = mockIntersectionObserver;

// Mock ResizeObserver for components that use it
const mockResizeObserver = vi.fn().mockImplementation(function () {
  return {
    observe: vi.fn(),
    unobserve: vi.fn(),
    disconnect: vi.fn(),
  };
});
Object.defineProperty(window, "ResizeObserver", {
  writable: true,
  configurable: true,
  value: mockResizeObserver,
});
global.ResizeObserver = mockResizeObserver;

// Mock window.matchMedia for responsive tests
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// jsdom has no canvas backend: its getContext() prints "Not implemented" to stderr and
// returns undefined. Return null (what a real browser returns when a context is
// unavailable) without the noise. Tests that need a context stub it themselves.
HTMLCanvasElement.prototype.getContext = vi.fn(() => null) as never;
