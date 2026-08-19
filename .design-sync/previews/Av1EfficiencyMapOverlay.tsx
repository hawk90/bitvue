import { Av1EfficiencyMapOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Av1EfficiencyMapOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Av1EfficiencyMapRenderer.tsx). CanvasStage
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
const GRID_W = 20;
const GRID_H = 12;

// Real per-CU residual-energy path (frame.energy_grid): a busy scene with a quiet sky band and
// a complex, high-bpp foreground.
export const EnergyGridBpp = () => {
  const energy_bpp: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) {
      const skyBand = row < GRID_H * 0.3;
      energy_bpp.push(skyBand ? 0.05 + (col % 3) * 0.02 : 0.8 + ((row * col) % 9) * 0.4);
    }
  }
  const frame: any = {
    frame_index: 55,
    frame_type: "P",
    size: 71230,
    energy_grid: { grid_w: GRID_W, grid_h: GRID_H, block_w: WIDTH / GRID_W, block_h: HEIGHT / GRID_H, energy_bpp },
  };
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1EfficiencyMapOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })}
      />
    </div>
  );
};

// Fallback path when no energy_grid is present (older extraction / non-AV1 path): QP-derived
// bpp proxy.
export const QpFallback = () => {
  const qp: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) {
      qp.push(20 + Math.round(20 * Math.abs(Math.sin((row + col) * 0.4))));
    }
  }
  const frame: any = {
    frame_index: 8,
    frame_type: "I",
    size: 152040,
    qp_grid: { grid_w: GRID_W, grid_h: GRID_H, block_w: WIDTH / GRID_W, block_h: HEIGHT / GRID_H, qp, qp_min: 20, qp_max: 40 },
  };
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1EfficiencyMapOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })}
      />
    </div>
  );
};
