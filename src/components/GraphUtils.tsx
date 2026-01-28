/**
 * Graph Utilities Component
 *
 * Shared utilities for rendering filmstrip visualizations
 * HRD Buffer, Enhanced views, and other graph-based displays
 */

import { ReactNode, memo } from 'react';

/**
 * Render prop function type for custom graph rendering
 */
export interface GraphRenderProps {
  xScale: (x: number) => number;
  yScale: (y: number) => number;
  plotWidth: number;
  plotHeight: number;
  padding: { top: number; right: number; bottom: number; left: number };
}

export interface DataPoint {
  x: number;
  y: number;
  value: number;
  color?: string;
  label?: string;
}

export interface GraphConfig {
  width: number;
  height: number;
  padding?: { top: number; right: number; bottom: number; left: number };
  showGrid?: boolean;
  gridLines?: number;
  showAxis?: boolean;
  axisLabels?: boolean;
  xDomain?: [number, number];
  yDomain?: [number, number];
}

export interface GraphProps {
  data: DataPoint[];
  config: GraphConfig;
  children?: ReactNode | ((props: GraphRenderProps) => ReactNode);
  className?: string;
  onClick?: (index: number, point: DataPoint) => void;
  onHover?: (index: number, point: DataPoint | null) => void;
  hoveredIndex?: number | null;
}

/**
 * Calculate scales for mapping data to screen coordinates
 */
export function calculateScales(
  data: DataPoint[],
  config: GraphConfig
): { xScale: (x: number) => number; yScale: (y: number) => number } {
  const { width, height, padding = { top: 20, right: 20, bottom: 30, left: 50 } } = config;

  const plotWidth = width - padding.left - padding.right;
  const plotHeight = height - padding.top - padding.bottom;

  // Handle empty data array to avoid Math.min/Math.max on empty arrays
  const xDomain = config.xDomain || (data.length > 0 ? [
    Math.min(...data.map((d) => d.x)),
    Math.max(...data.map((d) => d.x)),
  ] : [0, 1]);
  const yDomain = config.yDomain || (data.length > 0 ? [
    Math.min(...data.map((d) => d.value)),
    Math.max(...data.map((d) => d.value)),
  ] : [0, 1]);

  const xRange = xDomain[1] - xDomain[0] || 1;
  const yRange = yDomain[1] - yDomain[0] || 1;

  const xScale = (x: number) => padding.left + ((x - xDomain[0]) / xRange) * plotWidth;
  const yScale = (y: number) => padding.top + plotHeight - ((y - yDomain[0]) / yRange) * plotHeight;

  return { xScale, yScale };
}

/**
 * Generate SVG path for line chart
 */
export function generateLinePath(
  data: DataPoint[],
  xScale: (x: number) => number,
  yScale: (y: number) => number
): string {
  if (data.length === 0) return '';

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
  paddingBottom: number
): string {
  if (data.length === 0) return '';

  const linePath = generateLinePath(data, xScale, yScale);
  const first = data[0];
  const last = data[data.length - 1];

  const bottomY = height - paddingBottom;

  return `${linePath} L ${xScale(last.x)} ${bottomY} L ${xScale(first.x)} ${bottomY} Z`;
}

/**
 * Graph container component with axes and grid
 */
export const Graph = memo(function Graph({
  data,
  config,
  children,
  className = '',
  onClick,
  onHover,
  hoveredIndex,
}: GraphProps) {
  const {
    width,
    height,
    padding = { top: 20, right: 20, bottom: 30, left: 50 },
    showGrid = true,
    gridLines = 5,
    showAxis = true,
    axisLabels = true,
  } = config;

  const { xScale, yScale } = calculateScales(data, config);

  const plotWidth = width - padding.left - padding.right;
  const plotHeight = height - padding.top - padding.bottom;

  const yDomain = config.yDomain || [
    Math.min(...data.map((d) => d.value)),
    Math.max(...data.map((d) => d.value)),
  ];

  // Generate grid lines
  const gridYPositions = showGrid
    ? Array.from({ length: gridLines }, (_, i) => {
        const value = yDomain[0] + (yDomain[1] - yDomain[0]) * (i / (gridLines - 1));
        return { value, y: yScale(value) };
      })
    : [];

  return (
    <svg
      width={width}
      height={height}
      className={`graph-container ${className}`}
      style={{ overflow: 'visible' }}
    >
      {/* Background */}
      <rect
        x={padding.left}
        y={padding.top}
        width={plotWidth}
        height={plotHeight}
        fill="rgba(30, 30, 35, 0.8)"
        rx={4}
      />

      {/* Grid lines */}
      {showGrid &&
        gridYPositions.map(({ value, y }, i) => (
          <g key={`grid-${i}`}>
            <line
              x1={padding.left}
              y1={y}
              x2={width - padding.right}
              y2={y}
              stroke="rgba(255, 255, 255, 0.1)"
              strokeWidth={1}
            />
            {axisLabels && (
              <text
                x={padding.left - 8}
                y={y}
                textAnchor="end"
                dominantBaseline="middle"
                fill="rgba(255, 255, 255, 0.5)"
                fontSize={9}
                fontFamily="monospace"
              >
                {value.toFixed(1)}
              </text>
            )}
          </g>
        ))}

      {/* Y-axis */}
      {showAxis && (
        <line
          x1={padding.left}
          y1={padding.top}
          x2={padding.left}
          y2={height - padding.bottom}
          stroke="rgba(255, 255, 255, 0.3)"
          strokeWidth={1}
        />
      )}

      {/* X-axis */}
      {showAxis && (
        <line
          x1={padding.left}
          y1={height - padding.bottom}
          x2={width - padding.right}
          y2={height - padding.bottom}
          stroke="rgba(255, 255, 255, 0.3)"
          strokeWidth={1}
        />
      )}

      {/* Interactive data points */}
      {data.map((point, index) => {
        const cx = xScale(point.x);
        const cy = yScale(point.value);
        const isHovered = hoveredIndex === index;

        return (
          <g key={`point-${index}`}>
            <circle
              cx={cx}
              cy={cy}
              r={isHovered ? 8 : 5}
              fill={point.color || 'rgba(255, 255, 255, 0.3)'}
              className="graph-point"
              style={{ cursor: onClick ? 'pointer' : 'default' }}
              onClick={() => onClick?.(index, point)}
              onMouseEnter={() => onHover?.(index, point)}
              onMouseLeave={() => onHover?.(index, null)}
            />
            {isHovered && point.label && (
              <text
                x={cx}
                y={cy - 12}
                textAnchor="middle"
                fill="white"
                fontSize={10}
                fontWeight={600}
              >
                {point.label}
              </text>
            )}
          </g>
        );
      })}

      {/* Custom children rendering */}
      {typeof children === 'function'
        ? (children as (props: GraphRenderProps) => ReactNode)({ xScale, yScale, plotWidth, plotHeight, padding })
        : children}
    </svg>
  );
});

/**
 * Rolling average calculation for smoothing
 */
export function calculateRollingAverage(data: number[], window: number): number[] {
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
