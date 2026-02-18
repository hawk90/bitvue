/**
 * Frame Sizes Legend Component
 *
 * Draggable floating panel for metrics toggle
 */

import { useState, useRef, useCallback, useEffect, memo } from "react";
import type { SizeMetrics } from "./Filmstrip/views/FrameSizesView";

interface FrameSizesLegendProps {
  sizeMetrics: SizeMetrics;
  onToggleMetric: (metric: keyof SizeMetrics) => void;
}

interface Position {
  x: number;
  y: number;
}

interface MetricItem {
  key: keyof SizeMetrics;
  label: string;
  color: string;
}

const METRICS: MetricItem[] = [
  {
    key: "showBitrateBar",
    label: "Bitrate Bar",
    color: "rgba(0, 200, 200, 0.9)",
  },
  {
    key: "showBitrateCurve",
    label: "Bitrate Curve",
    color: "rgba(0, 220, 220, 1)",
  },
  { key: "showAvgSize", label: "Avg Size", color: "rgba(255, 255, 0, 0.7)" },
  { key: "showMinSize", label: "Min Size", color: "rgba(100, 150, 255, 0.7)" },
  { key: "showMaxSize", label: "Max Size", color: "rgba(255, 0, 0, 0.7)" },
  {
    key: "showMovingAvg",
    label: "Moving Avg",
    color: "rgba(0, 150, 255, 0.7)",
  },
  {
    key: "showBlockMinQP",
    label: "Block Min QP",
    color: "rgba(180, 100, 255, 0.9)",
  },
  {
    key: "showBlockMaxQP",
    label: "Block Max QP",
    color: "rgba(255, 100, 0, 0.9)",
  },
];

export const FrameSizesLegend = memo(function FrameSizesLegend({
  sizeMetrics,
  onToggleMetric,
}: FrameSizesLegendProps) {
  const [position, setPosition] = useState<Position>({
    x: window.innerWidth - 220,
    y: 100,
  });
  const [isDragging, setIsDragging] = useState(false);
  const dragOffset = useRef<Position>({ x: 0, y: 0 });

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if ((e.target as HTMLElement).closest(".legend-drag-handle")) {
        setIsDragging(true);
        dragOffset.current = {
          x: e.clientX - position.x,
          y: e.clientY - position.y,
        };
        e.preventDefault();
        e.stopPropagation();
      }
    },
    [position],
  );

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newX = e.clientX - dragOffset.current.x;
      const newY = e.clientY - dragOffset.current.y;

      setPosition({
        x: Math.max(0, Math.min(newX, window.innerWidth - 200)),
        y: Math.max(0, Math.min(newY, window.innerHeight - 400)),
      });
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging]);

  return (
    <div
      className="frame-sizes-legend floating"
      style={{
        position: "fixed",
        left: `${position.x}px`,
        top: `${position.y}px`,
        zIndex: 1000,
        cursor: isDragging ? "grabbing" : "default",
      }}
      onMouseDown={handleMouseDown}
    >
      {/* Drag Handle */}
      <div className="legend-drag-handle">
        <div className="legend-drag-gripper">
          <span className="codicon codicon-gripper"></span>
        </div>
      </div>

      {/* Metrics */}
      <div className="legend-content">
        {METRICS.map((metric) => (
          <div
            key={metric.key}
            className={`legend-item ${sizeMetrics[metric.key] ? "active" : ""}`}
            onClick={() => onToggleMetric(metric.key)}
          >
            <div
              className="legend-color-indicator"
              style={{ backgroundColor: metric.color }}
            />
            <span className="legend-label">{metric.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
});
