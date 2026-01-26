/**
 * Platform Detection Utility Tests
 * Tests platform detection and platform-specific UI behavior
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  detectPlatform,
  isMacOS,
  isWindows,
  isLinux,
  shouldUseNativeMenu,
  shouldShowTitleBar,
  type Platform,
} from '../platform';

// Store original navigator values
const originalUserAgent = navigator.userAgent;
const originalPlatform = navigator.platform;

describe('detectPlatform', () => {
  afterEach(() => {
    // Restore original values
    Object.defineProperty(navigator, 'userAgent', {
      value: originalUserAgent,
      writable: true,
      configurable: true,
    });
    Object.defineProperty(navigator, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    });
  });

  it('should detect macOS', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'MacIntel',
      writable: true,
      configurable: true,
    });

    expect(detectPlatform()).toBe('macos');
  });

  it('should detect macOS by user agent', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Unknown',
      writable: true,
      configurable: true,
    });
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)',
      writable: true,
      configurable: true,
    });

    expect(detectPlatform()).toBe('macos');
  });

  it('should detect Windows', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Win32',
      writable: true,
      configurable: true,
    });

    expect(detectPlatform()).toBe('windows');
  });

  it('should detect Windows by user agent', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Unknown',
      writable: true,
      configurable: true,
    });
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
      writable: true,
      configurable: true,
    });

    expect(detectPlatform()).toBe('windows');
  });

  it('should detect Linux', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Linux x86_64',
      writable: true,
      configurable: true,
    });

    expect(detectPlatform()).toBe('linux');
  });

  it('should detect Linux by user agent', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Unknown',
      writable: true,
      configurable: true,
    });
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Mozilla/5.0 (X11; Linux x86_64)',
      writable: true,
      configurable: true,
    });

    expect(detectPlatform()).toBe('linux');
  });

  it('should return unknown for unrecognized platforms', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Unknown',
      writable: true,
      configurable: true,
    });
    Object.defineProperty(navigator, 'userAgent', {
      value: 'Unknown',
      writable: true,
      configurable: true,
    });

    expect(detectPlatform()).toBe('unknown');
  });
});

describe('isMacOS', () => {
  afterEach(() => {
    Object.defineProperty(navigator, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    });
  });

  it('should return true on macOS', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'MacIntel',
      writable: true,
      configurable: true,
    });

    expect(isMacOS()).toBe(true);
  });

  it('should return false on Windows', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Win32',
      writable: true,
      configurable: true,
    });

    expect(isMacOS()).toBe(false);
  });

  it('should return false on Linux', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Linux x86_64',
      writable: true,
      configurable: true,
    });

    expect(isMacOS()).toBe(false);
  });
});

describe('isWindows', () => {
  afterEach(() => {
    Object.defineProperty(navigator, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    });
  });

  it('should return true on Windows', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Win32',
      writable: true,
      configurable: true,
    });

    expect(isWindows()).toBe(true);
  });

  it('should return false on macOS', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'MacIntel',
      writable: true,
      configurable: true,
    });

    expect(isWindows()).toBe(false);
  });

  it('should return false on Linux', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Linux x86_64',
      writable: true,
      configurable: true,
    });

    expect(isWindows()).toBe(false);
  });
});

describe('isLinux', () => {
  afterEach(() => {
    Object.defineProperty(navigator, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    });
  });

  it('should return true on Linux', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Linux x86_64',
      writable: true,
      configurable: true,
    });

    expect(isLinux()).toBe(true);
  });

  it('should return false on macOS', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'MacIntel',
      writable: true,
      configurable: true,
    });

    expect(isLinux()).toBe(false);
  });

  it('should return false on Windows', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Win32',
      writable: true,
      configurable: true,
    });

    expect(isLinux()).toBe(false);
  });
});

describe('shouldUseNativeMenu', () => {
  afterEach(() => {
    Object.defineProperty(navigator, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    });
  });

  it('should return true on macOS', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'MacIntel',
      writable: true,
      configurable: true,
    });

    expect(shouldUseNativeMenu()).toBe(true);
  });

  it('should return false on Windows', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Win32',
      writable: true,
      configurable: true,
    });

    expect(shouldUseNativeMenu()).toBe(false);
  });

  it('should return false on Linux', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Linux x86_64',
      writable: true,
      configurable: true,
    });

    expect(shouldUseNativeMenu()).toBe(false);
  });
});

describe('shouldShowTitleBar', () => {
  afterEach(() => {
    Object.defineProperty(navigator, 'platform', {
      value: originalPlatform,
      writable: true,
      configurable: true,
    });
  });

  it('should return false on macOS', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'MacIntel',
      writable: true,
      configurable: true,
    });

    expect(shouldShowTitleBar()).toBe(false);
  });

  it('should return true on Windows', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Win32',
      writable: true,
      configurable: true,
    });

    expect(shouldShowTitleBar()).toBe(true);
  });

  it('should return true on Linux', () => {
    Object.defineProperty(navigator, 'platform', {
      value: 'Linux x86_64',
      writable: true,
      configurable: true,
    });

    expect(shouldShowTitleBar()).toBe(true);
  });
});

describe('Platform type compatibility', () => {
  it('should accept all platform types', () => {
    const platforms: Platform[] = ['macos', 'windows', 'linux', 'unknown'];

    platforms.forEach(platform => {
      expect(platform).toBeTruthy();
    });
  });
});
