/**
 * Graph Utilities
 *
 * Pure utility functions for graph rendering and data processing.
 * Extracted from GraphUtils.tsx for better separation of concerns.
 */

/**
 * Graph rendering properties passed to render functions
 */
export interface GraphRenderProps {
  xScale: (x: number) => number;
  yScale: (y: number) => number;
  plotWidth: number;
  plotHeight: number;
  padding: { top: number; right: number; bottom: number; left: number };
}

/**
 * Data point for graph rendering
 */
export interface DataPoint {
  x: number;
  y: number;
  value: number;
  color?: string;
  label?: string;
}

/**
 * Calculate scales for mapping data to screen coordinates
 * Optimized to use single-pass iteration for min/max calculation
 */
export function calculateScales(
  data: DataPoint[],
  config: {
    width: number;
    height: number;
    padding?: { top: number; right: number; bottom: number; left: number };
    xDomain?: [number, number];
    yDomain?: [number, number];
  },
): {
  xScale: (x: number) => number;
  yScale: (y: number) => number;
  xDomain: [number, number];
  yDomain: [number, number];
} {
  const {
    width,
    height,
    padding = { top: 20, right: 20, bottom: 30, left: 50 },
  } = config;

  const plotWidth = width - padding.left - padding.right;
  const plotHeight = height - padding.top - padding.bottom;

  // Handle empty data array to avoid Math.min/Math.max on empty arrays
  // OPTIMIZATION: Single-pass iteration to find all min/max values (O(n) instead of O(4n))
  const xDomain =
    config.xDomain ||
    (data.length > 0
      ? (() => {
          let minX = Infinity,
            maxX = -Infinity;
          let minVal = Infinity,
            maxVal = -Infinity;
          for (const d of data) {
            if (d.x < minX) minX = d.x;
            if (d.x > maxX) maxX = d.x;
            if (d.value < minVal) minVal = d.value;
            if (d.value > maxVal) maxVal = d.value;
          }
          return [minX, maxX];
        })()
      : [0, 1]);

  const yDomain =
    config.yDomain ||
    (data.length > 0
      ? (() => {
          let minVal = Infinity,
            maxVal = -Infinity;
          for (const d of data) {
            if (d.value < minVal) minVal = d.value;
            if (d.value > maxVal) maxVal = d.value;
          }
          return [minVal, maxVal];
        })()
      : [0, 1]);

  const xRange = xDomain[1] - xDomain[0] || 1;
  const yRange = yDomain[1] - yDomain[0] || 1;

  const xScale = (x: number) =>
    padding.left + ((x - xDomain[0]) / xRange) * plotWidth;
  const yScale = (y: number) =>
    padding.top + plotHeight - ((y - yDomain[0]) / yRange) * plotHeight;

  return { xScale, yScale, xDomain, yDomain };
}

/**
 * Generate SVG path for line chart
 */
export function generateLinePath(
  data: DataPoint[],
  xScale: (x: number) => number,
  yScale: (y: number) => number,
): string {
  if (data.length === 0) return "";

  const first = data[0];
  let path = `M ${xScale(first.x)} ${yScale(first.value)}`;

  for (let i = 1; i < data.length; i++) {
    const point = data[i];
    path += ` L ${xScale(point.x)} ${yScale(point.value)}`;
  }

  return path;
}

/**
 * Generate SVG path for area chart (filled under line)
 */
export function generateAreaPath(
  data: DataPoint[],
  xScale: (x: number) => number,
  yScale: (y: number) => number,
  height: number,
  paddingBottom: number,
): string {
  if (data.length === 0) return "";

  const linePath = generateLinePath(data, xScale, yScale);
  const first = data[0];
  const last = data[data.length - 1];

  const bottomY = height - paddingBottom;

  return `${linePath} L ${xScale(last.x)} ${bottomY} L ${xScale(first.x)} ${bottomY} Z`;
}

/**
 * Rolling average calculation for smoothing
 */
export function calculateRollingAverage(
  data: number[],
  window: number,
): number[] {
  if (data.length === 0 || window < 2) return data;

  const result: number[] = [];
  for (let i = 0; i < data.length; i++) {
    const start = Math.max(0, i - Math.floor(window / 2));
    const end = Math.min(data.length, i + Math.ceil(window / 2));
    const slice = data.slice(start, end);
    const avg = slice.reduce((a, b) => a + b, 0) / slice.length;
    result.push(avg);
  }
  return result;
}
