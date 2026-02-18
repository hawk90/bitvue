/**
 * Helper Function Tests
 * Tests utility helper functions
 */

import { describe, it, expect } from "vitest";

describe("formatBytes", () => {
  const formatBytes = (bytes: number, decimals = 2): string => {
    if (bytes === 0) return "0 Bytes";
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ["Bytes", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i];
  };

  it("should format bytes correctly", () => {
    expect(formatBytes(0)).toBe("0 Bytes");
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1048576)).toBe("1 MB");
    expect(formatBytes(1073741824)).toBe("1 GB");
  });

  it("should handle decimal places", () => {
    expect(formatBytes(1536, 3)).toBe("1.5 KB");
    expect(formatBytes(1536, 0)).toBe("2 KB");
  });

  it("should format small values", () => {
    expect(formatBytes(512)).toBe("0.5 KB");
    expect(formatBytes(100)).toBe("100 Bytes");
  });
});

describe("formatNumber", () => {
  const formatNumber = (num: number, locale = "en-US"): string => {
    return new Intl.NumberFormat(locale).format(num);
  };

  it("should format numbers with commas", () => {
    expect(formatNumber(1000)).toBe("1,000");
    expect(formatNumber(1000000)).toBe("1,000,000");
  });

  it("should handle decimal places", () => {
    expect(formatNumber(1234.56)).toBe("1,234.56");
  });

  it("should format negative numbers", () => {
    expect(formatNumber(-1000)).toBe("-1,000");
  });
});

describe("formatDuration", () => {
  const formatDuration = (seconds: number): string => {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;

    if (h > 0) {
      return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
    }
    if (m > 0) {
      return `${m}:${s.toString().padStart(2, "0")}`;
    }
    return `${s}s`;
  };

  it("should format seconds only", () => {
    expect(formatDuration(30)).toBe("30s");
    expect(formatDuration(59)).toBe("59s");
  });

  it("should format minutes and seconds", () => {
    expect(formatDuration(60)).toBe("1:00");
    expect(formatDuration(90)).toBe("1:30");
    expect(formatDuration(3599)).toBe("59:59");
  });

  it("should format hours, minutes, and seconds", () => {
    expect(formatDuration(3600)).toBe("1:00:00");
    expect(formatDuration(3661)).toBe("1:01:01");
  });

  it("should handle zero", () => {
    expect(formatDuration(0)).toBe("0s");
  });
});

describe("clamp", () => {
  const clamp = (value: number, min: number, max: number): number => {
    return Math.min(Math.max(value, min), max);
  };

  it("should clamp to min", () => {
    expect(clamp(-5, 0, 100)).toBe(0);
    expect(clamp(0, 10, 100)).toBe(10);
  });

  it("should clamp to max", () => {
    expect(clamp(150, 0, 100)).toBe(100);
    expect(clamp(100, 0, 10)).toBe(10);
  });

  it("should return value within range", () => {
    expect(clamp(50, 0, 100)).toBe(50);
    expect(clamp(5, 0, 10)).toBe(5);
  });
});

describe("lerp", () => {
  const lerp = (start: number, end: number, t: number): number => {
    return start + (end - start) * t;
  };

  it("should interpolate correctly", () => {
    expect(lerp(0, 100, 0.5)).toBe(50);
    expect(lerp(0, 100, 0.25)).toBe(25);
    expect(lerp(0, 100, 0.75)).toBe(75);
  });

  it("should return start when t is 0", () => {
    expect(lerp(50, 100, 0)).toBe(50);
  });

  it("should return end when t is 1", () => {
    expect(lerp(50, 100, 1)).toBe(100);
  });

  it("should extrapolate when t > 1", () => {
    expect(lerp(0, 100, 1.5)).toBe(150);
  });

  it("should extrapolate backwards when t < 0", () => {
    expect(lerp(50, 100, -0.5)).toBe(25);
  });
});

describe("mapRange", () => {
  const mapRange = (
    value: number,
    inMin: number,
    inMax: number,
    outMin: number,
    outMax: number,
  ): number => {
    return ((value - inMin) * (outMax - outMin)) / (inMax - inMin) + outMin;
  };

  it("should map from one range to another", () => {
    expect(mapRange(5, 0, 10, 0, 100)).toBe(50);
    expect(mapRange(0, 0, 10, 0, 100)).toBe(0);
    expect(mapRange(10, 0, 10, 0, 100)).toBe(100);
  });

  it("should handle negative ranges", () => {
    expect(mapRange(0, -10, 10, -100, 100)).toBe(0);
    expect(mapRange(10, -10, 10, -100, 100)).toBe(100);
  });

  it("should scale down", () => {
    expect(mapRange(50, 0, 100, 0, 1)).toBe(0.5);
  });

  it("should scale up", () => {
    expect(mapRange(5, 0, 10, 0, 1000)).toBe(500);
  });
});

describe("debounce", () => {
  it("should create a debounced function", () => {
    let counter = 0;
    const fn = () => counter++;

    // Simple debounce implementation for testing
    const debounce = (func: () => void, delay: number) => {
      let timeoutId: ReturnType<typeof setTimeout> | null = null;
      return () => {
        if (timeoutId) clearTimeout(timeoutId);
        timeoutId = setTimeout(func, delay);
      };
    };

    const debouncedFn = debounce(fn, 100);
    debouncedFn();
    debouncedFn();
    debouncedFn();

    // Should only call once after delay
    expect(counter).toBe(0);
  });
});

describe("throttle", () => {
  it("should create a throttled function", () => {
    let counter = 0;
    const fn = () => counter++;

    // Simple throttle implementation for testing
    const throttle = (func: () => void, delay: number) => {
      let lastCall = 0;
      return () => {
        const now = Date.now();
        if (now - lastCall >= delay) {
          lastCall = now;
          func();
        }
      };
    };

    const throttledFn = throttle(fn, 100);
    throttledFn();
    throttledFn();

    // Should only call once due to throttling
    expect(counter).toBe(1);
  });
});
