/**
 * Graph Component
 *
 * Re-exports graph utilities and provides the Graph component.
 *
 * @deprecated Graph utilities moved to utils/graphUtils.ts
 * For new code, import from there: import { calculateScales } from './utils/graphUtils';
 */

import { ReactNode, memo } from "react";

// Re-export utilities from utils/graphUtils for backwards compatibility
export {
  calculateScales,
  generateLinePath,
  generateAreaPath,
  calculateRollingAverage,
  type GraphRenderProps,
  type DataPoint,
} from "../utils/graphUtils";

// Graph-specific types
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
 * Graph container component with axes and grid
 */
export const Graph = memo(function Graph({
  data,
  config,
  children,
  className = "",
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

  const { xScale, yScale, xDomain, yDomain } = calculateScales(data, config);

  const plotWidth = width - padding.left - padding.right;
  const plotHeight = height - padding.top - padding.bottom;

  // Generate grid lines
  const gridYPositions = showGrid
    ? Array.from({ length: gridLines }, (_, i) => {
        const value =
          yDomain[0] + (yDomain[1] - yDomain[0]) * (i / (gridLines - 1));
        return { value, y: yScale(value) };
      })
    : [];

  return (
    <svg
      width={width}
      height={height}
      className={`graph-container ${className}`}
      style={{ overflow: "visible" }}
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
              fill={point.color || "rgba(255, 255, 255, 0.3)"}
              className="graph-point"
              style={{ cursor: onClick ? "pointer" : "default" }}
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
      {typeof children === "function"
        ? (children as (props: GraphRenderProps) => ReactNode)({
            xScale,
            yScale,
            plotWidth,
            plotHeight,
            padding,
          })
        : children}
    </svg>
  );
});
