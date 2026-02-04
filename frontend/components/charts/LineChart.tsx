/**
 * LineChart Component
 *
 * Reusable line chart for RD curves and other metric visualizations
 * Supports multiple series, custom colors, and interactive tooltips
 */

import { memo, useRef, useState, useMemo } from 'react';
import { getCssVar } from '../../utils/css';

export interface DataPoint {
  x: number;
  y: number;
}

export interface Series {
  name: string;
  data: DataPoint[];
  color?: string;
  lineWidth?: number;
  showPoints?: boolean;
}

export interface LineChartProps {
  /** Array of data series to display */
  series: Series[];
  /** X-axis label */
  xAxisLabel?: string;
  /** Y-axis label */
  yAxisLabel?: string;
  /** Chart title */
  title?: string;
  /** Show grid lines */
  showGrid?: boolean;
  /** Show legend */
  showLegend?: boolean;
  /** Custom width in pixels (default: 100% of container) */
  width?: number;
  /** Custom height in pixels (default: 300px) */
  height?: number;
  /** Padding around chart area in pixels */
  padding?: { top: number; right: number; bottom: number; left: number };
  /** CSS class name for custom styling */
  className?: string;
}

/**
 * Default colors for series
 */
const DEFAULT_COLORS = [
  getCssVar('--accent-primary'),
  getCssVar('--accent-secondary'),
  '#10b981', // green
  '#f59e0b', // amber
  '#ef4444', // red
  '#8b5cf6', // purple
];

/**
 * Get color for a series index
 */
function getSeriesColor(index: number, customColor?: string): string {
  if (customColor) {
    return customColor;
  }
  return DEFAULT_COLORS[index % DEFAULT_COLORS.length];
}

/**
 * Format number for display
 */
function formatNumber(num: number, decimals = 2): string {
  return num.toFixed(decimals);
}

export const LineChart = memo(function LineChart({
  series,
  xAxisLabel = 'Bitrate (kbps)',
  yAxisLabel = 'Quality (PSNR)',
  title,
  showGrid = true,
  showLegend = true,
  width,
  height = 350,
  padding = { top: 20, right: 20, bottom: 40, left: 60 },
  className = 'line-chart',
}: LineChartProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [hoveredPoint, setHoveredPoint] = useState<{ seriesIndex: number; pointIndex: number } | null>(null);

  // Calculate chart dimensions
  const chartWidth = width ?? '100%';
  const innerWidth = typeof width === 'number' ? width - padding.left - padding.right : 600;
  const innerHeight = height - padding.top - padding.bottom;

  // Find min/max values across all series
  const { xMin, xMax, yMin, yMax } = useMemo(() => {
    let xMin = Infinity;
    let xMax = -Infinity;
    let yMin = Infinity;
    let yMax = -Infinity;

    series.forEach(s => {
      s.data.forEach(point => {
        xMin = Math.min(xMin, point.x);
        xMax = Math.max(xMax, point.x);
        yMin = Math.min(yMin, point.y);
        yMax = Math.max(yMax, point.y);
      });
    });

    // Add some padding to the range
    const xPadding = (xMax - xMin) * 0.05 || 1;
    const yPadding = (yMax - yMin) * 0.05 || 1;

    return {
      xMin: xMin - xPadding,
      xMax: xMax + xPadding,
      yMin: yMin - yPadding,
      yMax: yMax + yPadding,
    };
  }, [series]);

  // Scale functions
  const scaleX = (x: number) => {
    if (xMax === xMin) return 0;
    return padding.left + ((x - xMin) / (xMax - xMin)) * innerWidth;
  };

  const scaleY = (y: number) => {
    if (yMax === yMin) return innerHeight;
    return innerHeight - padding.top - ((y - yMin) / (yMax - yMin)) * innerHeight;
  };

  // Generate path data for a series
  const generatePath = (data: DataPoint[]) => {
    if (data.length === 0) return '';
    let path = `M ${scaleX(data[0].x)} ${scaleY(data[0].y)}`;
    for (let i = 1; i < data.length; i++) {
      path += ` L ${scaleX(data[i].x)} ${scaleY(data[i].y)}`;
    }
    return path;
  };

  // Generate grid lines
  const gridLines = useMemo(() => {
    if (!showGrid) return [];

    const lines: JSX.Element[] = [];

    // Vertical grid lines (X-axis)
    for (let i = 0; i <= 5; i++) {
      const x = padding.left + (innerWidth / 5) * i;
      const value = xMin + ((xMax - xMin) / 5) * i;
      lines.push (
        <g key={`vgrid-${i}`}>
          <line
            x1={x}
            y1={padding.top}
            x2={x}
            y2={innerHeight + padding.top}
            stroke="var(--border-color)"
            strokeWidth="1"
            strokeDasharray="4"
          />
          <text
            x={x}
            y={innerHeight + padding.top + 15}
            textAnchor="middle"
            fontSize="10"
            fill="var(--text-secondary)"
          >
            {formatNumber(value)}
          </text>
        </g>
      );
    }

    // Horizontal grid lines (Y-axis)
    for (let i = 0; i <= 5; i++) {
      const y = padding.top + (innerHeight / 5) * i;
      const value = yMax - ((yMax - yMin) / 5) * i;
      lines.push(
        <g key={`hgrid-${i}`}>
          <line
            x1={padding.left}
            y1={y}
            x2={innerWidth + padding.left}
            y2={y}
            stroke="var(--border-color)"
            strokeWidth="1"
            strokeDasharray="4"
          />
          <text
            x={padding.left - 10}
            y={y + 4}
            textAnchor="end"
            fontSize="10"
            fill="var(--text-secondary)"
          >
            {formatNumber(value)}
          </text>
        </g>
      );
    }

    return lines;
  }, [showGrid, innerWidth, innerHeight, padding, xMin, xMax, yMin, yMax]);

  // Generate axis labels
  const axisLabels = useMemo(() => {
    return (
      <>
        {/* X-axis label */}
        <text
          x={padding.left + innerWidth / 2}
          y={height - 5}
          textAnchor="middle"
          fontSize="12"
          fill="var(--text-primary)"
          fontWeight="500"
        >
          {xAxisLabel}
        </text>

        {/* Y-axis label (rotated) */}
        <text
          x={15}
          y={padding.top + innerHeight / 2}
          textAnchor="middle"
          fontSize="12"
          fill="var(--text-primary)"
          fontWeight="500"
          transform={`rotate(-90, 15, ${padding.top + innerHeight / 2})`}
        >
          {yAxisLabel}
        </text>
      </>
    );
  }, [xAxisLabel, yAxisLabel, innerWidth, innerHeight, padding, height]);

  if (series.length === 0 || series.every(s => s.data.length === 0)) {
    return (
      <div className={className} style={{ width: chartWidth, height }}>
        <div className="flex items-center justify-center h-full text-text-secondary">
          No data to display
        </div>
      </div>
    );
  }

  return (
    <div className={className} style={{ width: chartWidth, height }}>
      {title && (
        <h3 className="text-sm font-semibold mb-2 text-text-primary">{title}</h3>
      )}
      <svg
        ref={svgRef}
        width="100%"
        height="100%"
        viewBox={`0 0 ${typeof width === 'number' ? width : innerWidth + padding.left + padding.right} ${height}`}
        preserveAspectRatio="xMidYMid meet"
      >
        {gridLines}
        {axisLabels}

        {/* Data series */}
        {series.map((s, seriesIndex) => {
          const color = getSeriesColor(seriesIndex, s.color);
          const pathData = generatePath(s.data);

          return (
            <g key={s.name}>
              {/* Line */}
              <path
                d={pathData}
                fill="none"
                stroke={color}
                strokeWidth={s.lineWidth ?? 2}
                strokeLinecap="round"
                strokeLinejoin="round"
              />

              {/* Data points */}
              {s.showPoints !== false && s.data.map((point, pointIndex) => {
                const cx = scaleX(point.x);
                const cy = scaleY(point.y);
                const isHovered = hoveredPoint?.seriesIndex === seriesIndex && hoveredPoint?.pointIndex === pointIndex;

                return (
                  <g key={pointIndex}>
                    <circle
                      cx={cx}
                      cy={cy}
                      r={isHovered ? 6 : 3}
                      fill={color}
                      stroke="var(--bg-primary)"
                      strokeWidth="1"
                      style={{ cursor: 'pointer' }}
                      onMouseEnter={() => setHoveredPoint({ seriesIndex, pointIndex })}
                      onMouseLeave={() => setHoveredPoint(null)}
                    />

                    {/* Tooltip */}
                    {isHovered && (
                      <g>
                        <rect
                          x={cx + 8}
                          y={cy - 30}
                          width={80}
                          height={24}
                          fill="var(--bg-secondary)"
                          stroke={color}
                          strokeWidth="1"
                          rx="3"
                        />
                        <text
                          x={cx + 48}
                          y={cy - 14}
                          textAnchor="middle"
                          fontSize="10"
                          fill="var(--text-primary)"
                        >
                          {formatNumber(point.y)} @ {formatNumber(point.x)}
                        </text>
                      </g>
                    )}
                  </g>
                );
              })}
            </g>
          );
        })}
      </svg>

      {/* Legend */}
      {showLegend && (
        <div className="flex flex-wrap gap-4 mt-3 justify-center">
          {series.map((s, i) => {
            const color = getSeriesColor(i, s.color);
            return (
              <div key={s.name} className="flex items-center gap-2">
                <div
                  className="w-3 h-3 rounded-full"
                  style={{ backgroundColor: color }}
                />
                <span className="text-xs text-text-secondary">{s.name}</span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
});

LineChart.displayName = 'LineChart';
