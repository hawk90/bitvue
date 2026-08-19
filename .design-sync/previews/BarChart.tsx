import { BarChart } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const FrameTypeDistribution = () => (
  <div style={previewBg}>
    <BarChart data={{ I: 42, P: 968, B: 472 }} />
  </div>
);

export const QpHistogram = () => (
  <div style={previewBg}>
    <BarChart
      data={{
        "QP 0-16": 12,
        "QP 17-32": 184,
        "QP 33-48": 620,
        "QP 49-63": 96,
      }}
      colors={{
        "QP 0-16": "#10b981",
        "QP 17-32": "#3b82f6",
        "QP 33-48": "#f59e0b",
        "QP 49-63": "#ef4444",
      }}
    />
  </div>
);

export const CustomMax = () => (
  <div style={previewBg}>
    <BarChart
      data={{ INTRA_BC: 3, SINGLE_REF: 1240, COMPOUND: 318 }}
      maxValue={2000}
      minBarHeight={2}
    />
  </div>
);
