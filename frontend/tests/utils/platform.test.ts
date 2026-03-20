/**
 * Platform Detection Utility Tests
 * Tests platform detection and platform-specific UI behavior
 */

import { describe, it, expect, afterEach } from "vitest";
import {
  detectPlatform,
  isMacOS,
  isWindows,
  isLinux,
  shouldUseNativeMenu,
  shouldShowTitleBar,
  type Platform,
} from "@/utils/platform";

// Store original navigator values
const originalUserAgent = navigator.userAgent;

function setUserAgent(value: string) {
  Object.defineProperty(navigator, "userAgent", {
    value,
    writable: true,
    configurable: true,
  });
}

describe("detectPlatform", () => {
  afterEach(() => {
    // Restore original values
    Object.defineProperty(navigator, "userAgent", {
      value: originalUserAgent,
      writable: true,
      configurable: true,
    });
  });

  it("should detect macOS", () => {
    setUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(detectPlatform()).toBe("macos");
  });

  it("should detect macOS by user agent", () => {
    setUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(detectPlatform()).toBe("macos");
  });

  it("should detect Windows", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(detectPlatform()).toBe("windows");
  });

  it("should detect Windows by user agent", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(detectPlatform()).toBe("windows");
  });

  it("should detect Linux", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(detectPlatform()).toBe("linux");
  });

  it("should detect Linux by user agent", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(detectPlatform()).toBe("linux");
  });

  it("should return unknown for unrecognized platforms", () => {
    setUserAgent("Unknown");
    expect(detectPlatform()).toBe("unknown");
  });
});

describe("isMacOS", () => {
  afterEach(() => {
    setUserAgent(originalUserAgent);
  });

  it("should return true on macOS", () => {
    setUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(isMacOS()).toBe(true);
  });

  it("should return false on Windows", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(isMacOS()).toBe(false);
  });

  it("should return false on Linux", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(isMacOS()).toBe(false);
  });
});

describe("isWindows", () => {
  afterEach(() => {
    setUserAgent(originalUserAgent);
  });

  it("should return true on Windows", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(isWindows()).toBe(true);
  });

  it("should return false on macOS", () => {
    setUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(isWindows()).toBe(false);
  });

  it("should return false on Linux", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(isWindows()).toBe(false);
  });
});

describe("isLinux", () => {
  afterEach(() => {
    setUserAgent(originalUserAgent);
  });

  it("should return true on Linux", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(isLinux()).toBe(true);
  });

  it("should return false on macOS", () => {
    setUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(isLinux()).toBe(false);
  });

  it("should return false on Windows", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(isLinux()).toBe(false);
  });
});

describe("shouldUseNativeMenu", () => {
  afterEach(() => {
    setUserAgent(originalUserAgent);
  });

  it("should return true on macOS", () => {
    setUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(shouldUseNativeMenu()).toBe(true);
  });

  it("should return false on Windows", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(shouldUseNativeMenu()).toBe(false);
  });

  it("should return false on Linux", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(shouldUseNativeMenu()).toBe(false);
  });
});

describe("shouldShowTitleBar", () => {
  afterEach(() => {
    setUserAgent(originalUserAgent);
  });

  it("should return false on macOS", () => {
    setUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(shouldShowTitleBar()).toBe(false);
  });

  it("should return true on Windows", () => {
    setUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(shouldShowTitleBar()).toBe(true);
  });

  it("should return true on Linux", () => {
    setUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(shouldShowTitleBar()).toBe(true);
  });
});

describe("Platform type compatibility", () => {
  it("should accept all platform types", () => {
    const platforms: Platform[] = ["macos", "windows", "linux", "unknown"];

    platforms.forEach((platform) => {
      expect(platform).toBeTruthy();
    });
  });
});
