import { Graph } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const BitrateOverFrames = () => (
  <div style={previewBg}>
    <Graph
      data={[
        { x: 0, y: 0, value: 120, label: "frame 0" },
        { x: 1, y: 0, value: 480, label: "frame 30" },
        { x: 2, y: 0, value: 210, label: "frame 60" },
        { x: 3, y: 0, value: 610, label: "frame 90" },
        { x: 4, y: 0, value: 340, label: "frame 120" },
        { x: 5, y: 0, value: 720, label: "frame 150" },
      ]}
      config={{
        width: 480,
        height: 220,
        showGrid: true,
        showAxis: true,
        axisLabels: true,
        gridLines: 4,
      }}
    />
  </div>
);

export const QpHeatmapPoints = () => (
  <div style={previewBg}>
    <Graph
      data={[
        { x: 0, y: 0, value: 18, color: "#10b981", label: "QP 18" },
        { x: 1, y: 0, value: 24, color: "#3b82f6", label: "QP 24" },
        { x: 2, y: 0, value: 41, color: "#f59e0b", label: "QP 41" },
        { x: 3, y: 0, value: 55, color: "#ef4444", label: "QP 55" },
        { x: 4, y: 0, value: 22, color: "#3b82f6", label: "QP 22" },
      ]}
      config={{
        width: 480,
        height: 180,
        showGrid: false,
        showAxis: true,
        axisLabels: false,
        yDomain: [0, 63],
      }}
      hoveredIndex={2}
    />
  </div>
);
