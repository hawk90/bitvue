import { PredictionOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// PredictionOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/PredictionRenderer.tsx). CanvasStage
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
const HEIGHT = 272;
const BLOCK = 16;
const GRID_W = WIDTH / BLOCK;
const GRID_H = HEIGHT / BLOCK;

function buildModes(modeAt: (r: number, c: number) => number) {
  const modes: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) modes.push(modeAt(row, col));
  }
  return { coded_width: WIDTH, coded_height: HEIGHT, block_w: BLOCK, block_h: BLOCK, grid_w: GRID_W, grid_h: GRID_H, modes };
}

// Key frame -- only INTRA directional modes (0-13)
export const IntraModes = () => {
  const grid = buildModes((r, c) => (r * 5 + c) % 14);
  const frame: any = { frame_index: 0, frame_type: "I", size: 198320, prediction_mode_grid: grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => PredictionOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// Inter frame -- mostly NewMv/NearestMv/GlobalMv (64-68) with a small intra patch
export const InterModes = () => {
  const grid = buildModes((r, c) => (c < GRID_W * 0.2 ? c % 3 : 64 + ((r * 3 + c) % 5)));
  const frame: any = { frame_index: 52, frame_type: "P", size: 19402, prediction_mode_grid: grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => PredictionOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
