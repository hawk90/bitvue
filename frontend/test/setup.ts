import "@testing-library/jest-dom";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";
import React from "react";

// Clean up after each test
afterEach(() => {
  cleanup();
});

/**
 * Generate mock hex data for testing
 * Creates realistic-looking hex dump with start codes and OBU headers
 */
function generateMockHexData(
  frameIndex: number,
  maxBytes: number,
  frameSize: number,
) {
  const data: number[] = [];
  const size = Math.min(maxBytes, frameSize);

  // Add AV1 OBU start code pattern (00 00 01) at the beginning
  data.push(0x00, 0x00, 0x01);

  // Generate mock data with variation based on frame index
  for (let i = 3; i < size; i++) {
    // Create patterns that vary by frame index
    if (i === 3) {
      // OBU header byte (varies by frame)
      data.push(0x10 + (frameIndex % 8));
    } else if (i < 20) {
      // OBU header extension
      data.push(0x20 + ((i + frameIndex) % 64));
    } else {
      // Mock frame data (pseudo-random but deterministic)
      data.push((i * 7 + frameIndex * 13) % 256);
    }
  }

  return data;
}

// Mock Tauri APIs - must be hoisted before module imports
const { mockInvoke } = vi.hoisted(() => {
  return {
    mockInvoke: vi
      .fn()
      .mockImplementation((cmd: string, args: Record<string, unknown> = {}) => {
        if (cmd === "get_frame_hex_data") {
          const frameIndex = args?.frameIndex ?? 0;
          const maxBytes = args?.maxBytes ?? 512;
          // Use frameSize from args or default to 100
          // Tests pass frame size in the frames array, not as an argument
          const frameSize = args?.frameSize ?? 100;

          // Generate mock hex data based on frame index
          const dataLength = Math.min(maxBytes, frameSize);
          const mockData = generateMockHexData(frameIndex, maxBytes, frameSize);
          const isTruncated = maxBytes < frameSize;

          return Promise.resolve({
            frame_index: frameIndex,
            data: mockData,
            size: frameSize,
            truncated: isTruncated,
            success: true,
          });
        }

        if (cmd === "open_file") {
          return Promise.resolve({
            success: true,
            path: "/test/path/video.ivf",
            codec: "av1",
            frame_count: 100,
          });
        }

        // Default return for other commands
        return Promise.resolve({});
      }),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  readTextFile: vi.fn(),
  exists: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

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

// Mock IntersectionObserver for components that use it
const mockIntersectionObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
  takeRecords: vi.fn().mockReturnValue([]),
}));
Object.defineProperty(window, "IntersectionObserver", {
  writable: true,
  configurable: true,
  value: mockIntersectionObserver,
});
global.IntersectionObserver = mockIntersectionObserver;

// Mock ResizeObserver for components that use it
const mockResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}));
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
