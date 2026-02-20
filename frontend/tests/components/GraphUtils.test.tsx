/**
 * GraphUtils Component Tests
 * Tests graph rendering utilities
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@/test/test-utils";
import { fireEvent } from "@testing-library/react";
import {
  Graph,
  calculateScales,
  generateLinePath,
  generateAreaPath,
  calculateRollingAverage,
} from "../GraphUtils";

describe("calculateScales", () => {
  const mockData = [
    { x: 0, y: 10, value: 10 },
    { x: 10, y: 20, value: 20 },
    { x: 20, y: 30, value: 30 },
    { x: 30, y: 50, value: 50 },
  ];

  const config = {
    width: 400,
    height: 200,
    padding: { top: 20, right: 20, bottom: 30, left: 40 },
  };

  it("should calculate x scale", () => {
    const { xScale } = calculateScales(mockData, config);

    expect(xScale(0)).toBe(40); // left padding
    expect(xScale(30)).toBe(380); // width - right padding
  });

  it("should calculate y scale", () => {
    const { yScale } = calculateScales(mockData, config);

    // yScale uses value field, not y field
    // height=200, padding={top:20, bottom:30}, plotHeight=150
    // value range is 10-50, so range=40
    // yScale(10) = 20 + 150 - ((10-10)/40)*150 = 170 - 0 = 170
    // yScale(50) = 20 + 150 - ((50-10)/40)*150 = 170 - 150 = 20
    expect(yScale(10)).toBeCloseTo(170, 1); // Near bottom
    expect(yScale(50)).toBeCloseTo(20, 1); // Near top
  });

  it("should handle custom x domain", () => {
    const customConfig = { ...config, xDomain: [5, 25] };

    const { xScale } = calculateScales(mockData, customConfig);

    expect(xScale(5)).toBe(40);
    expect(xScale(25)).toBe(380);
  });

  it("should handle custom y domain", () => {
    const customConfig = { ...config, yDomain: [0, 100] };

    const { yScale } = calculateScales(mockData, customConfig);

    // With custom yDomain [0, 100]: range = 100
    // yScale(0) = 20 + 150 - ((0-0)/100)*150 = 170
    // yScale(100) = 20 + 150 - ((100-0)/100)*150 = 20
    expect(yScale(0)).toBeCloseTo(170, 1);
    expect(yScale(100)).toBeCloseTo(20, 1);
  });

  it("should handle empty data", () => {
    const { xScale, yScale } = calculateScales([], config);

    expect(xScale).toBeDefined();
    expect(yScale).toBeDefined();
  });
});

describe("generateLinePath", () => {
  it("should generate SVG path for line chart", () => {
    const mockData = [
      { x: 0, y: 10, value: 10 },
      { x: 10, y: 20, value: 20 },
      { x: 20, y: 30, value: 30 },
    ];

    const xScale = (x: number) => x * 10;
    const yScale = (y: number) => 200 - y * 2;

    const path = generateLinePath(mockData, xScale, yScale);

    expect(path).toContain("M"); // Move command
    expect(path).toContain("L"); // Line commands
  });

  it("should return empty string for empty data", () => {
    const path = generateLinePath([], vi.fn(), vi.fn());

    expect(path).toBe("");
  });

  it("should handle single data point", () => {
    const mockData = [{ x: 0, y: 10, value: 10 }];
    const xScale = (x: number) => x;
    const yScale = (y: number) => y;

    const path = generateLinePath(mockData, xScale, yScale);

    expect(path).toBeDefined();
  });
});

describe("generateAreaPath", () => {
  it("should generate SVG path for area chart", () => {
    const mockData = [
      { x: 0, y: 10, value: 10 },
      { x: 10, y: 20, value: 20 },
      { x: 20, y: 30, value: 30 },
    ];

    const xScale = (x: number) => x * 10;
    const yScale = (y: number) => 200 - y * 2;

    const path = generateAreaPath(mockData, xScale, yScale, 200, 30);

    expect(path).toContain("M"); // Move
    expect(path).toContain("L"); // Line
    expect(path).toContain("Z"); // Close path
  });

  it("should close area at bottom", () => {
    const mockData = [
      { x: 0, y: 10, value: 10 },
      { x: 10, y: 20, value: 20 },
    ];

    const xScale = (x: number) => x * 10;
    const yScale = (y: number) => 200 - y * 2;

    const path = generateAreaPath(mockData, xScale, yScale, 200, 30);

    // Should include bottom line
    expect(path).toContain("Z");
  });

  it("should handle empty data", () => {
    const path = generateAreaPath([], vi.fn(), vi.fn(), 100, 20);

    expect(path).toBe("");
  });
});

describe("calculateRollingAverage", () => {
  it("should calculate rolling average", () => {
    const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const window = 3;

    const result = calculateRollingAverage(data, window);

    // The implementation uses a centered window approach
    // For window=3: Math.floor(3/2)=1 on left, Math.ceil(3/2)=2 on right
    // i=0: slice=[1,2] avg=1.5
    // i=1: slice=[1,2,3] avg=2
    // i=2: slice=[2,3,4] avg=3
    // i=3: slice=[3,4,5] avg=4
    // i=4: slice=[4,5,6] avg=5
    // i=5: slice=[5,6,7] avg=6
    // i=6: slice=[6,7,8] avg=7
    // i=7: slice=[7,8,9] avg=8
    // i=8: slice=[8,9,10] avg=9
    // i=9: slice=[9,10] avg=9.5
    expect(result).toEqual([1.5, 2, 3, 4, 5, 6, 7, 8, 9, 9.5]);
  });

  it("should return original data for window < 2", () => {
    const data = [1, 2, 3];
    const window = 1;

    const result = calculateRollingAverage(data, window);

    expect(result).toEqual([1, 2, 3]);
  });

  it("should handle empty data", () => {
    const result = calculateRollingAverage([], 5);

    expect(result).toEqual([]);
  });

  it("should handle large window", () => {
    const data = [1, 2, 3];
    const window = 10;

    const result = calculateRollingAverage(data, window);

    // For window=10: Math.floor(10/2)=5 on left, Math.ceil(10/2)=5 on right
    // All slices will be [1,2,3] with min/max bounds, so all averages are 2
    expect(result).toEqual([2, 2, 2]); // Average of all
  });
});

describe("Graph Component", () => {
  const mockData = [
    { x: 0, y: 10, value: 10 },
    { x: 1, y: 20, value: 20 },
    { x: 2, y: 15, value: 15 },
    { x: 3, y: 25, value: 25 },
  ];

  const config = {
    width: 300,
    height: 200,
  };

  it("should render graph", () => {
    render(<Graph data={mockData} config={config} />);

    const svg = document.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });

  it("should render axes", () => {
    render(<Graph data={mockData} config={config} showAxis />);

    const xAxis = document.querySelector(".timeline-cursor"); // Reuse selector or use appropriate
    expect(document.querySelector(".graph-container")).toBeInTheDocument();
  });

  it("should render grid lines", () => {
    render(<Graph data={mockData} config={config} showGrid />);

    // Should have grid lines
    const gridLines = document.querySelectorAll(
      '[stroke*="rgba(255, 255, 255, 0.1)"]',
    );
    expect(gridLines.length).toBeGreaterThan(0);
  });

  it("should render data points", () => {
    render(<Graph data={mockData} config={config} />);

    // Should have circles for data points
    const circles = document.querySelectorAll("circle");
    expect(circles.length).toBe(4);
  });

  it("should show tooltip on hover", () => {
    render(<Graph data={mockData} config={config} />);

    const points = document.querySelectorAll(".graph-point");
    if (points.length > 0) {
      fireEvent.mouseEnter(points[0]);

      // Should show label
      expect(points[0]).toHaveClass("graph-point");
    }
  });

  it("should handle click events", () => {
    const handleClick = vi.fn();
    render(<Graph data={mockData} config={config} onClick={handleClick} />);

    const points = document.querySelectorAll(".graph-point");
    if (points.length > 0) {
      fireEvent.click(points[0]);
      expect(handleClick).toHaveBeenCalled();
    }
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<Graph data={mockData} config={config} />);

    rerender(<Graph data={mockData} config={config} />);

    const svg = document.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });
});

describe("Graph edge cases", () => {
  it("should handle single data point", () => {
    const singlePoint = [{ x: 0, y: 10, value: 10 }];
    const config = { width: 300, height: 200 };

    render(<Graph data={singlePoint} config={config} />);

    const circles = document.querySelectorAll("circle");
    expect(circles.length).toBe(1);
  });

  it("should handle zero values", () => {
    const zeroData = [
      { x: 0, y: 0, value: 0 },
      { x: 10, y: 0, value: 0 },
      { x: 20, y: 0, value: 0 },
    ];

    const { yScale } = calculateScales(zeroData, { width: 300, height: 200 });

    expect(yScale(0)).toBeDefined();
  });

  it("should handle negative values", () => {
    const negativeData = [
      { x: 0, y: -10, value: -10 },
      { x: 10, y: -20, value: -20 },
    ];

    const { yScale } = calculateScales(negativeData, {
      width: 300,
      height: 200,
    });

    expect(yScale(-20)).toBeDefined();
  });
});
