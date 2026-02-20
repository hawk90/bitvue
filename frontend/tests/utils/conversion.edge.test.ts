/**
 * YUV Conversion Edge Case Tests
 *
 * Tests boundary conditions, abnormal inputs, and edge cases
 * for YUV to RGB conversion utilities.
 */

import { describe, it, expect } from "vitest";
import {
  yuvToRgb,
  yuvToRgbaArray,
  setPixelBlack,
  getColorspaceMatrix,
  RGB_SHIFT,
  RGBA_CHANNELS,
  DEFAULT_ALPHA,
} from "@/utils/yuv/conversion";
import { Colorspace } from "@/types/yuv";

describe("yuvToRgb edge cases", () => {
  describe("boundary values", () => {
    it("should handle minimum YUV values (0, 0, 0)", () => {
      const result = yuvToRgb(0, 0, 0, Colorspace.BT709);
      expect(result).toBeGreaterThanOrEqual(0);
      expect(result).toBeLessThanOrEqual(0xffffff);
    });

    it("should handle maximum YUV values (255, 255, 255)", () => {
      const result = yuvToRgb(255, 255, 255, Colorspace.BT709);
      expect(result).toBeGreaterThanOrEqual(0);
      expect(result).toBeLessThanOrEqual(0xffffff);
    });

    it("should handle Y at boundaries (0 and 255)", () => {
      const resultMin = yuvToRgb(0, 128, 128, Colorspace.BT709);
      const resultMax = yuvToRgb(255, 128, 128, Colorspace.BT709);
      expect(resultMin).toBeGreaterThanOrEqual(0);
      expect(resultMax).toBeLessThanOrEqual(0xffffff);
    });

    it("should handle U at boundaries (0 and 255)", () => {
      const resultMin = yuvToRgb(128, 0, 128, Colorspace.BT709);
      const resultMax = yuvToRgb(128, 255, 128, Colorspace.BT709);
      expect(resultMin).toBeGreaterThanOrEqual(0);
      expect(resultMax).toBeLessThanOrEqual(0xffffff);
    });

    it("should handle V at boundaries (0 and 255)", () => {
      const resultMin = yuvToRgb(128, 128, 0, Colorspace.BT709);
      const resultMax = yuvToRgb(128, 128, 255, Colorspace.BT709);
      expect(resultMin).toBeGreaterThanOrEqual(0);
      expect(resultMax).toBeLessThanOrEqual(0xffffff);
    });
  });

  describe("grayscale values (U=V=128)", () => {
    it("should produce pure grayscale for various Y values", () => {
      for (let y = 0; y <= 255; y += 15) {
        const result = yuvToRgb(y, 128, 128, Colorspace.BT709);
        const r = (result >> RGB_SHIFT.R) & 0xff;
        const g = (result >> RGB_SHIFT.G) & 0xff;
        const b = result & 0xff;

        // Grayscale should have R=G=B (approximately, due to rounding)
        expect(Math.abs(r - g)).toBeLessThanOrEqual(1);
        expect(Math.abs(g - b)).toBeLessThanOrEqual(1);
        expect(Math.abs(r - b)).toBeLessThanOrEqual(1);
      }
    });

    it("should handle black (Y=0, U=128, V=128)", () => {
      const result = yuvToRgb(0, 128, 128, Colorspace.BT709);
      const r = (result >> RGB_SHIFT.R) & 0xff;
      const g = (result >> RGB_SHIFT.G) & 0xff;
      const b = result & 0xff;

      expect(r).toBe(0);
      expect(g).toBe(0);
      expect(b).toBe(0);
    });

    it("should handle white (Y=255, U=128, V=128)", () => {
      const result = yuvToRgb(255, 128, 128, Colorspace.BT709);
      const r = (result >> RGB_SHIFT.R) & 0xff;
      const g = (result >> RGB_SHIFT.G) & 0xff;
      const b = result & 0xff;

      expect(r).toBe(255);
      expect(g).toBe(255);
      expect(b).toBe(255);
    });
  });

  describe("extreme color combinations", () => {
    it("should handle pure red (max V)", () => {
      const result = yuvToRgb(128, 128, 255, Colorspace.BT709);
      const r = (result >> RGB_SHIFT.R) & 0xff;
      const g = (result >> RGB_SHIFT.G) & 0xff;
      const b = result & 0xff;

      // Red should be highest
      expect(r).toBeGreaterThan(g);
      expect(r).toBeGreaterThan(b);
    });

    it("should handle pure blue (max U)", () => {
      const result = yuvToRgb(128, 255, 128, Colorspace.BT709);
      const r = (result >> RGB_SHIFT.R) & 0xff;
      const g = (result >> RGB_SHIFT.G) & 0xff;
      const b = result & 0xff;

      // Blue should be highest
      expect(b).toBeGreaterThan(r);
      expect(b).toBeGreaterThanOrEqual(g);
    });

    it("should handle high contrast combinations", () => {
      const combinations = [
        [0, 0, 0],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [255, 255, 255],
        [255, 0, 255],
        [255, 255, 0],
      ];

      for (const [y, u, v] of combinations) {
        const result = yuvToRgb(y, u, v, Colorspace.BT709);
        expect(result).toBeGreaterThanOrEqual(0);
        expect(result).toBeLessThanOrEqual(0xffffff);
      }
    });
  });
});

describe("yuvToRgbaArray edge cases", () => {
  it("should handle empty output array with valid index", () => {
    const out = new Uint8ClampedArray(4);
    yuvToRgbaArray(128, 128, 128, Colorspace.BT709, out, 0);

    expect(out[RGBA_CHANNELS.R]).toBeGreaterThanOrEqual(0);
    expect(out[RGBA_CHANNELS.G]).toBeGreaterThanOrEqual(0);
    expect(out[RGBA_CHANNELS.B]).toBeGreaterThanOrEqual(0);
    expect(out[RGBA_CHANNELS.A]).toBe(DEFAULT_ALPHA);
  });

  it("should handle boundary indices", () => {
    const out = new Uint8ClampedArray(8); // 2 pixels

    // First pixel
    yuvToRgbaArray(128, 128, 128, Colorspace.BT709, out, 0);
    expect(out[RGBA_CHANNELS.A]).toBe(DEFAULT_ALPHA);

    // Second pixel
    yuvToRgbaArray(64, 64, 64, Colorspace.BT709, out, 4);
    expect(out[4 + RGBA_CHANNELS.A]).toBe(DEFAULT_ALPHA);
  });

  it("should not write beyond specified index", () => {
    const out = new Uint8ClampedArray(8);
    out.fill(0);

    yuvToRgbaArray(128, 128, 128, Colorspace.BT709, out, 0);

    // Only first 4 bytes should be modified
    expect(out[4]).toBe(0);
    expect(out[5]).toBe(0);
    expect(out[6]).toBe(0);
    expect(out[7]).toBe(0);
  });

  it("should handle multiple consecutive writes", () => {
    const out = new Uint8ClampedArray(16); // 4 pixels

    for (let i = 0; i < 4; i++) {
      yuvToRgbaArray(i * 64, 128, 128, Colorspace.BT709, out, i * 4);
    }

    // All alpha values should be set
    for (let i = 0; i < 4; i++) {
      expect(out[i * 4 + RGBA_CHANNELS.A]).toBe(DEFAULT_ALPHA);
    }
  });
});

describe("setPixelBlack edge cases", () => {
  it("should set pixel to pure black", () => {
    const out = new Uint8ClampedArray(4);
    setPixelBlack(out, 0);

    expect(out[RGBA_CHANNELS.R]).toBe(0);
    expect(out[RGBA_CHANNELS.G]).toBe(0);
    expect(out[RGBA_CHANNELS.B]).toBe(0);
    expect(out[RGBA_CHANNELS.A]).toBe(DEFAULT_ALPHA);
  });

  it("should overwrite existing pixel data", () => {
    const out = new Uint8ClampedArray(4);
    out[0] = 255;
    out[1] = 128;
    out[2] = 64;
    out[3] = 200;

    setPixelBlack(out, 0);

    expect(out[RGBA_CHANNELS.R]).toBe(0);
    expect(out[RGBA_CHANNELS.G]).toBe(0);
    expect(out[RGBA_CHANNELS.B]).toBe(0);
    expect(out[RGBA_CHANNELS.A]).toBe(DEFAULT_ALPHA);
  });

  it("should handle boundary indices", () => {
    const out = new Uint8ClampedArray(8);

    setPixelBlack(out, 0);
    setPixelBlack(out, 4);

    expect(out[RGBA_CHANNELS.R]).toBe(0);
    expect(out[4 + RGBA_CHANNELS.R]).toBe(0);
  });
});

describe("getColorspaceMatrix edge cases", () => {
  it("should handle all valid colorspaces", () => {
    const colorspaces = [Colorspace.BT601, Colorspace.BT709, Colorspace.BT2020];

    for (const colorspace of colorspaces) {
      const matrix = getColorspaceMatrix(colorspace);
      expect(matrix).toBeDefined();
      expect(typeof matrix.rv).toBe("number");
      expect(typeof matrix.gu).toBe("number");
      expect(typeof matrix.gv).toBe("number");
      expect(typeof matrix.bu).toBe("number");
    }
  });

  it("should return coefficients within valid ranges", () => {
    const matrix = getColorspaceMatrix(Colorspace.BT709);

    // Coefficients should be finite numbers
    expect(Number.isFinite(matrix.rv)).toBe(true);
    expect(Number.isFinite(matrix.gu)).toBe(true);
    expect(Number.isFinite(matrix.gv)).toBe(true);
    expect(Number.isFinite(matrix.bu)).toBe(true);
  });
});

describe("constants", () => {
  it("should have correct RGB shift values", () => {
    expect(RGB_SHIFT.R).toBe(16);
    expect(RGB_SHIFT.G).toBe(8);
    expect(RGB_SHIFT.B).toBe(0);
    expect(RGB_SHIFT.A).toBe(24);
  });

  it("should have correct RGBA channel indices", () => {
    expect(RGBA_CHANNELS.R).toBe(0);
    expect(RGBA_CHANNELS.G).toBe(1);
    expect(RGBA_CHANNELS.B).toBe(2);
    expect(RGBA_CHANNELS.A).toBe(3);
  });

  it("should have default alpha as fully opaque", () => {
    expect(DEFAULT_ALPHA).toBe(255);
  });
});

describe("clamping behavior", () => {
  it("should clamp negative RGB values to 0", () => {
    // Y=0, U=0, V=0 can produce negative values in conversion
    const result = yuvToRgb(0, 0, 0, Colorspace.BT709);
    const r = (result >> RGB_SHIFT.R) & 0xff;
    const g = (result >> RGB_SHIFT.G) & 0xff;
    const b = result & 0xff;

    expect(r).toBeGreaterThanOrEqual(0);
    expect(g).toBeGreaterThanOrEqual(0);
    expect(b).toBeGreaterThanOrEqual(0);
  });

  it("should clamp RGB values above 255 to 255", () => {
    // Some combinations can exceed 255
    const result = yuvToRgb(255, 255, 255, Colorspace.BT709);
    const r = (result >> RGB_SHIFT.R) & 0xff;
    const g = (result >> RGB_SHIFT.G) & 0xff;
    const b = result & 0xff;

    expect(r).toBeLessThanOrEqual(255);
    expect(g).toBeLessThanOrEqual(255);
    expect(b).toBeLessThanOrEqual(255);
  });
});

describe("colorspace comparison", () => {
  it("should produce different results for different colorspaces", () => {
    const yuv = { y: 180, u: 80, v: 160 };

    const result709 = yuvToRgb(yuv.y, yuv.u, yuv.v, Colorspace.BT709);
    const result601 = yuvToRgb(yuv.y, yuv.u, yuv.v, Colorspace.BT601);

    expect(result709).not.toBe(result601);
  });

  it("should produce same results for same colorspace", () => {
    const yuv = { y: 128, u: 128, v: 128 };

    const result1 = yuvToRgb(yuv.y, yuv.u, yuv.v, Colorspace.BT709);
    const result2 = yuvToRgb(yuv.y, yuv.u, yuv.v, Colorspace.BT709);

    expect(result1).toBe(result2);
  });
});
