/**
 * Graph Utilities Edge Case Tests
 *
 * Tests boundary conditions, abnormal inputs, and edge cases
 * for graph rendering utilities.
 */

import { describe, it, expect } from "vitest";
import {
  calculateScales,
  generateLinePath,
  generateAreaPath,
  calculateRollingAverage,
  type DataPoint,
} from "@/utils/graphUtils";

describe("calculateScales edge cases", () => {
  describe("empty data", () => {
    it("should handle empty data array", () => {
      const result = calculateScales([], {
        width: 100,
        height: 100,
      });

      expect(result.xDomain).toEqual([0, 1]);
      expect(result.yDomain).toEqual([0, 1]);
      expect(typeof result.xScale).toBe("function");
      expect(typeof result.yScale).toBe("function");
    });

    it("should handle empty data with custom domains", () => {
      const result = calculateScales([], {
        width: 100,
        height: 100,
        xDomain: [0, 100],
        yDomain: [-50, 50],
      });

      expect(result.xDomain).toEqual([0, 100]);
      expect(result.yDomain).toEqual([-50, 50]);
    });
  });

  describe("single data point", () => {
    it("should handle single data point", () => {
      const data: DataPoint[] = [{ x: 50, y: 50, value: 25 }];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      // With single point, min equals max
      expect(result.xDomain[0]).toBe(result.xDomain[1]);
      expect(result.yDomain[0]).toBe(result.yDomain[1]);
    });

    it("should handle single point at origin", () => {
      const data: DataPoint[] = [{ x: 0, y: 0, value: 0 }];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain).toEqual([0, 0]);
      expect(result.yDomain).toEqual([0, 0]);
    });

    it("should handle single point at large values", () => {
      const data: DataPoint[] = [{ x: 1e10, y: 1e10, value: 1e10 }];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain[0]).toBe(1e10);
      expect(result.yDomain[0]).toBe(1e10);
    });
  });

  describe("extreme values", () => {
    it("should handle all zero values", () => {
      const data: DataPoint[] = [
        { x: 0, y: 0, value: 0 },
        { x: 0, y: 0, value: 0 },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain).toEqual([0, 0]);
      expect(result.yDomain).toEqual([0, 0]);
    });

    it("should handle negative values", () => {
      const data: DataPoint[] = [
        { x: -100, y: -50, value: -75 },
        { x: 100, y: 50, value: 75 },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain[0]).toBeLessThan(0);
      expect(result.yDomain[0]).toBeLessThan(0);
      expect(result.xDomain[1]).toBeGreaterThan(0);
      expect(result.yDomain[1]).toBeGreaterThan(0);
    });

    it("should handle very large values", () => {
      const data: DataPoint[] = [
        {
          x: Number.MAX_SAFE_INTEGER,
          y: Number.MAX_SAFE_INTEGER,
          value: Number.MAX_SAFE_INTEGER,
        },
        { x: 0, y: 0, value: 0 },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain[1]).toBe(Number.MAX_SAFE_INTEGER);
      expect(result.yDomain[1]).toBe(Number.MAX_SAFE_INTEGER);
    });

    it("should handle very small positive values", () => {
      const data: DataPoint[] = [
        { x: Number.MIN_VALUE, y: Number.MIN_VALUE, value: Number.MIN_VALUE },
        { x: 0, y: 0, value: 0 },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain[0]).toBe(0);
      expect(result.yDomain[0]).toBe(0);
    });

    it("should handle Infinity values", () => {
      const data: DataPoint[] = [
        { x: 0, y: 0, value: 0 },
        { x: Infinity, y: Infinity, value: Infinity },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain[1]).toBe(Infinity);
      expect(result.yDomain[1]).toBe(Infinity);
    });

    it("should handle -Infinity values", () => {
      const data: DataPoint[] = [
        { x: 0, y: 0, value: 0 },
        { x: -Infinity, y: -Infinity, value: -Infinity },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      expect(result.xDomain[0]).toBe(-Infinity);
      expect(result.yDomain[0]).toBe(-Infinity);
    });
  });

  describe("identical values", () => {
    it("should handle all identical x values", () => {
      const data: DataPoint[] = [
        { x: 50, y: 10, value: 20 },
        { x: 50, y: 30, value: 40 },
        { x: 50, y: 50, value: 60 },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      // When all x values are same, range should be 0 or 1
      const xRange = result.xDomain[1] - result.xDomain[0];
      expect(xRange === 0 || xRange === 1).toBe(true);
    });

    it("should handle all identical y values", () => {
      const data: DataPoint[] = [
        { x: 10, y: 50, value: 50 },
        { x: 30, y: 50, value: 50 },
        { x: 50, y: 50, value: 50 },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      // When all y values are same, range should be 0 or 1
      const yRange = result.yDomain[1] - result.yDomain[0];
      expect(yRange === 0 || yRange === 1).toBe(true);
    });
  });

  describe("extreme dimensions", () => {
    it("should handle zero width", () => {
      const data: DataPoint[] = [{ x: 0, y: 0, value: 0 }];

      const result = calculateScales(data, {
        width: 0,
        height: 100,
      });

      expect(typeof result.xScale).toBe("function");
    });

    it("should handle zero height", () => {
      const data: DataPoint[] = [{ x: 0, y: 0, value: 0 }];

      const result = calculateScales(data, {
        width: 100,
        height: 0,
      });

      expect(typeof result.yScale).toBe("function");
    });

    it("should handle very large dimensions", () => {
      const data: DataPoint[] = [{ x: 50, y: 50, value: 25 }];

      const result = calculateScales(data, {
        width: 1000000,
        height: 1000000,
      });

      expect(typeof result.xScale).toBe("function");
      expect(typeof result.yScale).toBe("function");
    });

    it("should handle negative dimensions", () => {
      const data: DataPoint[] = [{ x: 50, y: 50, value: 25 }];

      const result = calculateScales(data, {
        width: -100,
        height: -100,
      });

      // Should still return functions, behavior undefined
      expect(typeof result.xScale).toBe("function");
    });
  });

  describe("extreme padding", () => {
    it("should handle zero padding", () => {
      const data: DataPoint[] = [{ x: 50, y: 50, value: 25 }];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
        padding: { top: 0, right: 0, bottom: 0, left: 0 },
      });

      expect(typeof result.xScale).toBe("function");
    });

    it("should handle padding larger than dimensions", () => {
      const data: DataPoint[] = [{ x: 50, y: 50, value: 25 }];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
        padding: { top: 200, right: 200, bottom: 200, left: 200 },
      });

      expect(typeof result.xScale).toBe("function");
    });

    it("should handle negative padding", () => {
      const data: DataPoint[] = [{ x: 50, y: 50, value: 25 }];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
        padding: { top: -10, right: -10, bottom: -10, left: -10 },
      });

      expect(typeof result.xScale).toBe("function");
    });
  });

  describe("scale function edge cases", () => {
    it("should map domain boundaries correctly", () => {
      const data: DataPoint[] = [
        { x: 0, y: 0, value: 0 },
        { x: 100, y: 100, value: 100 },
      ];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
        padding: { top: 0, right: 0, bottom: 0, left: 0 },
      });

      // xScale should map 0 to 0 and 100 to 100
      expect(result.xScale(0)).toBe(0);
      expect(result.xScale(100)).toBe(100);

      // yScale should map 0 to 100 and 100 to 0 (inverted)
      expect(result.yScale(0)).toBe(100);
      expect(result.yScale(100)).toBe(0);
    });

    it("should handle values outside domain", () => {
      const data: DataPoint[] = [{ x: 50, y: 50, value: 50 }];

      const result = calculateScales(data, {
        width: 100,
        height: 100,
      });

      // Values outside domain should still scale
      expect(result.xScale(-1000)).toBeDefined();
      expect(result.xScale(1000)).toBeDefined();
      expect(result.yScale(-1000)).toBeDefined();
      expect(result.yScale(1000)).toBeDefined();
    });
  });
});

describe("generateLinePath edge cases", () => {
  const mockXScale = (x: number) => x;
  const mockYScale = (y: number) => y;

  it("should handle empty data array", () => {
    const path = generateLinePath([], mockXScale, mockYScale);
    expect(path).toBe("");
  });

  it("should handle single data point", () => {
    const data: DataPoint[] = [{ x: 50, y: 50, value: 50 }];
    const path = generateLinePath(data, mockXScale, mockYScale);

    expect(path).toContain("M");
    expect(path).toContain("50");
  });

  it("should handle NaN values", () => {
    const data: DataPoint[] = [
      { x: 0, y: 0, value: 0 },
      { x: NaN, y: NaN, value: NaN },
      { x: 100, y: 100, value: 100 },
    ];

    const path = generateLinePath(data, mockXScale, mockYScale);

    // Should still generate a path with NaN converted to string
    expect(path).toBeDefined();
  });

  it("should handle Infinity values", () => {
    const data: DataPoint[] = [
      { x: 0, y: 0, value: 0 },
      { x: Infinity, y: Infinity, value: Infinity },
    ];

    const path = generateLinePath(data, mockXScale, mockYScale);

    // Should still generate a path with Infinity converted to string
    expect(path).toBeDefined();
  });

  it("should handle negative values", () => {
    const data: DataPoint[] = [
      { x: -50, y: -50, value: -50 },
      { x: 50, y: 50, value: 50 },
    ];

    const path = generateLinePath(data, mockXScale, mockYScale);

    expect(path).toContain("-50");
  });

  it("should handle very large number of points", () => {
    const data: DataPoint[] = Array.from({ length: 100000 }, (_, i) => ({
      x: i,
      y: i,
      value: i,
    }));

    const path = generateLinePath(data, mockXScale, mockYScale);

    expect(path.length).toBeGreaterThan(0);
  });
});

describe("generateAreaPath edge cases", () => {
  const mockXScale = (x: number) => x;
  const mockYScale = (y: number) => y;

  it("should handle empty data array", () => {
    const path = generateAreaPath([], mockXScale, mockYScale, 100, 0);
    expect(path).toBe("");
  });

  it("should handle single data point", () => {
    const data: DataPoint[] = [{ x: 50, y: 50, value: 50 }];
    const path = generateAreaPath(data, mockXScale, mockYScale, 100, 0);

    expect(path).toContain("M");
    expect(path).toContain("L");
    expect(path).toContain("Z");
  });

  it("should handle zero height", () => {
    const data: DataPoint[] = [{ x: 50, y: 50, value: 50 }];
    const path = generateAreaPath(data, mockXScale, mockYScale, 0, 0);

    expect(path).toBeDefined();
  });

  it("should handle paddingBottom larger than height", () => {
    const data: DataPoint[] = [
      { x: 0, y: 0, value: 0 },
      { x: 100, y: 100, value: 100 },
    ];
    const path = generateAreaPath(data, mockXScale, mockYScale, 100, 200);

    expect(path).toBeDefined();
  });

  it("should handle negative paddingBottom", () => {
    const data: DataPoint[] = [{ x: 50, y: 50, value: 50 }];
    const path = generateAreaPath(data, mockXScale, mockYScale, 100, -10);

    expect(path).toBeDefined();
  });
});

describe("calculateRollingAverage edge cases", () => {
  it("should handle empty array", () => {
    const result = calculateRollingAverage([], 5);
    expect(result).toEqual([]);
  });

  it("should handle single element", () => {
    const result = calculateRollingAverage([42], 5);
    expect(result).toEqual([42]);
  });

  it("should handle window size of 1", () => {
    const data = [1, 2, 3, 4, 5];
    const result = calculateRollingAverage(data, 1);

    expect(result).toEqual([1, 2, 3, 4, 5]);
  });

  it("should handle window size of 0", () => {
    const data = [1, 2, 3, 4, 5];
    const result = calculateRollingAverage(data, 0);

    // Window < 2 returns original array
    expect(result).toEqual(data);
  });

  it("should handle window size larger than data", () => {
    const data = [1, 2, 3];
    const result = calculateRollingAverage(data, 10);

    // Should still calculate averages with available data
    expect(result).toHaveLength(3);
    expect(result[0]).toBeCloseTo(2);
    expect(result[1]).toBeCloseTo(2);
    expect(result[2]).toBeCloseTo(2);
  });

  it("should handle negative window size", () => {
    const data = [1, 2, 3, 4, 5];
    const result = calculateRollingAverage(data, -5);

    // Window < 2 returns original array
    expect(result).toEqual(data);
  });

  it("should handle all zeros", () => {
    const data = [0, 0, 0, 0, 0];
    const result = calculateRollingAverage(data, 3);

    expect(result).toEqual([0, 0, 0, 0, 0]);
  });

  it("should handle very large values", () => {
    const data = [Number.MAX_VALUE, Number.MAX_VALUE, Number.MAX_VALUE];
    const result = calculateRollingAverage(data, 3);

    // Summing MAX_VALUE multiple times causes overflow to Infinity
    expect(result[0]).toBe(Infinity);
    expect(result[1]).toBe(Infinity);
    expect(result[2]).toBe(Infinity);
  });

  it("should handle very small values", () => {
    const data = [Number.MIN_VALUE, Number.MIN_VALUE, Number.MIN_VALUE];
    const result = calculateRollingAverage(data, 3);

    expect(result).toEqual([
      Number.MIN_VALUE,
      Number.MIN_VALUE,
      Number.MIN_VALUE,
    ]);
  });

  it("should handle mixed positive and negative", () => {
    const data = [-10, 0, 10, -5, 5];
    const result = calculateRollingAverage(data, 3);

    // For window=3: floor(3/2)=1, ceil(3/2)=2
    // i=0: start=max(0,0-1)=0, end=min(5,0+2)=2, slice=[-10,0], avg=-5
    // i=1: start=max(0,1-1)=0, end=min(5,1+2)=3, slice=[-10,0,10], avg=0
    // i=2: start=max(0,2-1)=1, end=min(5,2+2)=4, slice=[0,10,-5], avg=1.67
    // i=3: start=max(0,3-1)=2, end=min(5,3+2)=5, slice=[10,-5,5], avg=3.33
    // i=4: start=max(0,4-1)=3, end=min(5,4+2)=5, slice=[-5,5], avg=0
    expect(result[0]).toBe(-5);
    expect(result[1]).toBe(0);
    expect(result[2]).toBeCloseTo(1.67, 1);
    expect(result[3]).toBeCloseTo(3.33, 1);
    expect(result[4]).toBe(0);
  });

  it("should handle NaN values", () => {
    const data = [1, NaN, 3];
    const result = calculateRollingAverage(data, 3);

    // NaN in calculation should propagate
    expect(result).toHaveLength(3);
  });

  it("should handle Infinity values", () => {
    const data = [1, Infinity, 3];
    const result = calculateRollingAverage(data, 3);

    expect(result).toHaveLength(3);
  });

  it("should handle very large array", () => {
    const data = Array.from({ length: 1000000 }, () => 5);
    const result = calculateRollingAverage(data, 100);

    expect(result).toHaveLength(1000000);
    expect(result[0]).toBeCloseTo(5);
  });
});
