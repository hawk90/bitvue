import { JpegXsDequantRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// JpegXsDequantRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/JpegXsDequantRenderer.tsx). CanvasStage
// hosts a <canvas>, grabs its 2D context in a layout effect, and invokes the function directly.
const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

function CanvasStage({
  width,
  height,
  draw,
}: {
  width: number;
  height: number;
  draw: (ctx: CanvasRenderingContext2D) => void;
}) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  useLayoutEffect(() => {
    const ctx = ref.current?.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, width, height);
    draw(ctx);
  });
  return (
    <canvas
      ref={ref}
      width={width}
      height={height}
      style={{ display: "block", background: "#0a0a0a", borderRadius: 4 }}
    />
  );
}

const WIDTH = 480;
const HEIGHT = 270;

// 2-level wavelet decomposition sub-band grid: LL, LH1/HL1/HH1 (level-1), LH2/HL2/HH2 (level-2)
const SUBBANDS = [
  { label: "LL", energy: 0.15 },
  { label: "LH1", energy: 0.35 },
  { label: "HL1", energy: 0.4 },
  { label: "HH1", energy: 0.55 },
  { label: "LH2", energy: 0.65 },
  { label: "HL2", energy: 0.7 },
  { label: "HH2", energy: 0.95 },
  { label: "LH3", energy: 0.2 },
];

export const TwoLevelDecomp = () => {
  const map = {
    grid_w: 4,
    grid_h: 2,
    energy: SUBBANDS.map((s) => s.energy),
    labels: SUBBANDS.map((s) => s.label),
  };
  const frame: any = { frame_index: 0, frame_type: "I", size: 98304, dequant_map: map };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => JpegXsDequantRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const HighEnergyBurst = () => {
  const energy = SUBBANDS.map((s, i) => Math.min(1, s.energy + (i % 2 === 0 ? 0.3 : 0)));
  const map = { grid_w: 4, grid_h: 2, energy, labels: SUBBANDS.map((s) => s.label) };
  const frame: any = { frame_index: 12, frame_type: "I", size: 121004, dequant_map: map };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => JpegXsDequantRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
