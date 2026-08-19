import { FrameSizesLegend } from "bitvue";
import { useRef, useState, type CSSProperties } from "react";

// FrameSizesLegend is `position: fixed` and draggable -- `transform` on this wrapper makes it the
// containing block for the fixed panel (same trick as FilmstripTooltip.tsx) so it renders inside
// the visible card. It also self-repositions via `anchorRef` shortly after mount (a macrotask
// `setTimeout`, see the component's own doc) to sit just below the chart element `anchorRef`
// points at -- so this preview renders a stand-in "chart" box and passes its ref, matching how
// FrameSizesView really uses it, instead of leaving the legend at its raw
// `window.innerWidth`-relative fallback position (which would land outside this card).
const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 520,
  height: 340,
};

const chartBoxStyle: CSSProperties = {
  position: "absolute",
  top: 24,
  left: 24,
  width: 360,
  height: 160,
  background: "#151515",
  border: "1px solid rgba(255,255,255,0.08)",
  borderRadius: 4,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  color: "rgba(255,255,255,0.35)",
  fontSize: 12,
  fontFamily: "sans-serif",
};

interface SizeMetrics {
  showBitrateBar: boolean;
  showBitrateCurve: boolean;
  showAvgSize: boolean;
  showMinSize: boolean;
  showMaxSize: boolean;
  showMovingAvg: boolean;
  showBlockMinQP: boolean;
  showBlockMaxQP: boolean;
}

export const Default = () => {
  const anchorRef = useRef<HTMLDivElement>(null);
  const [sizeMetrics, setSizeMetrics] = useState<SizeMetrics>({
    showBitrateBar: true,
    showBitrateCurve: true,
    showAvgSize: false,
    showMinSize: false,
    showMaxSize: true,
    showMovingAvg: true,
    showBlockMinQP: false,
    showBlockMaxQP: false,
  });
  return (
    <div style={previewBg}>
      <div ref={anchorRef} style={chartBoxStyle}>
        sample.ivf -- Frame Sizes chart (legend anchors below this)
      </div>
      <FrameSizesLegend
        sizeMetrics={sizeMetrics}
        onToggleMetric={(metric) =>
          setSizeMetrics((prev) => ({ ...prev, [metric]: !prev[metric] }))
        }
        anchorRef={anchorRef}
      />
    </div>
  );
};

export const AllMetricsEnabled = () => {
  const anchorRef = useRef<HTMLDivElement>(null);
  const [sizeMetrics, setSizeMetrics] = useState<SizeMetrics>({
    showBitrateBar: true,
    showBitrateCurve: true,
    showAvgSize: true,
    showMinSize: true,
    showMaxSize: true,
    showMovingAvg: true,
    showBlockMinQP: true,
    showBlockMaxQP: true,
  });
  return (
    <div style={previewBg}>
      <div ref={anchorRef} style={chartBoxStyle}>
        clip_4k_av1.ivf -- Frame Sizes chart (legend anchors below this)
      </div>
      <FrameSizesLegend
        sizeMetrics={sizeMetrics}
        onToggleMetric={(metric) =>
          setSizeMetrics((prev) => ({ ...prev, [metric]: !prev[metric] }))
        }
        anchorRef={anchorRef}
      />
    </div>
  );
};
