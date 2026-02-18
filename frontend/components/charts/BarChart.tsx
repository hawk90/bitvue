/**
 * BarChart Component
 *
 * Reusable horizontal bar chart for statistics visualization
 * Supports custom colors and dynamic max value calculation
 */

import { memo, useMemo } from "react";
import { getCssVar } from "../../utils/css";

export interface BarChartProps {
  /** Data mapping labels to values */
  data: Record<string, number>;
  /** Maximum value for bar width calculation (auto-calculated if not provided) */
  maxValue?: number;
  /** Custom colors for each bar (defaults to frame type colors) */
  colors?: Record<string, string>;
  /** Minimum bar height percentage */
  minBarHeight?: number;
  /** Custom bar height in pixels */
  barHeight?: number;
  /** CSS class name for custom styling */
  className?: string;
}

/**
 * Default colors for frame types
 */
const DEFAULT_FRAME_COLORS: Record<string, string> = {
  I: getCssVar("--frame-i"),
  KEY: getCssVar("--frame-i"),
  INTRA: getCssVar("--frame-i"),
  P: getCssVar("--frame-p"),
  INTER: getCssVar("--frame-p"),
  B: getCssVar("--frame-b"),
};

/**
 * Get color for a bar label with fallback
 */
function getBarColor(
  label: string,
  customColors?: Record<string, string>,
): string {
  if (customColors && customColors[label]) {
    return customColors[label];
  }
  if (DEFAULT_FRAME_COLORS[label]) {
    return DEFAULT_FRAME_COLORS[label];
  }
  return getCssVar("--text-secondary");
}

export const BarChart = memo(function BarChart({
  data,
  maxValue,
  colors,
  minBarHeight = 1,
  className = "bar-chart",
}: BarChartProps) {
  // Memoize expensive operations: sorting and max calculation
  const { entries, maxVal } = useMemo(() => {
    // Sort entries by value (descending) for better visual comparison
    const sortedEntries = Object.entries(data).sort(([, a], [, b]) => b - a);

    // Calculate max value if not provided
    const calculatedMax =
      maxValue ?? Math.max(...sortedEntries.map(([, v]) => v), 1);

    return { entries: sortedEntries, maxVal: calculatedMax };
  }, [data, maxValue]);

  if (entries.length === 0) {
    return null;
  }

  return (
    <div className={className}>
      {entries.map(([label, value]) => {
        const width =
          maxVal > 0 ? Math.max((value / maxVal) * 100, minBarHeight) : 0;
        const color = getBarColor(label, colors);

        return (
          <div key={label} className="bar-chart-row">
            <span className="bar-chart-label">{label}:</span>
            <div className="bar-chart-bar-container">
              <div
                className="bar-chart-bar"
                style={{ width: `${width}%`, backgroundColor: color }}
                title={`${label}: ${value}`}
              />
            </div>
            <span className="bar-chart-value">{value}</span>
          </div>
        );
      })}
    </div>
  );
});

BarChart.displayName = "BarChart";
