import { LineChart } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const RdCurveSingleSeries = () => (
  <div style={previewBg}>
    <LineChart
      title="RD Curve — sample.ivf"
      series={[
        {
          name: "AV1 (svt-av1)",
          data: [
            { x: 800, y: 34.2 },
            { x: 1500, y: 37.1 },
            { x: 2500, y: 39.6 },
            { x: 4000, y: 41.8 },
            { x: 6000, y: 43.5 },
          ],
        },
      ]}
    />
  </div>
);

export const RdCurveMultiSeriesComparison = () => (
  <div style={previewBg}>
    <LineChart
      title="Codec Comparison"
      xAxisLabel="Bitrate (kbps)"
      yAxisLabel="Quality (VMAF)"
      series={[
        {
          name: "AV1",
          data: [
            { x: 1000, y: 78 },
            { x: 2000, y: 88 },
            { x: 4000, y: 94 },
            { x: 8000, y: 97 },
          ],
        },
        {
          name: "HEVC",
          data: [
            { x: 1000, y: 71 },
            { x: 2000, y: 82 },
            { x: 4000, y: 90 },
            { x: 8000, y: 95 },
          ],
          color: "#f59e0b",
        },
        {
          name: "AVC",
          data: [
            { x: 1000, y: 60 },
            { x: 2000, y: 73 },
            { x: 4000, y: 84 },
            { x: 8000, y: 91 },
          ],
          color: "#ef4444",
          showPoints: false,
        },
      ]}
    />
  </div>
);

export const NoData = () => (
  <div style={previewBg}>
    <LineChart series={[]} title="Bitrate over time" />
  </div>
);
