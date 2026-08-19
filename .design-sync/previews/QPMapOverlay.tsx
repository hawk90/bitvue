import { QPMapOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// QPMapOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/QPMapRenderer.tsx). Notably it destructures
// only `{ ctx, frame }` -- it ignores the `width`/`height` props entirely and draws each cell at
// `col*block_w, row*block_h` in raw grid-pixel space, so the canvas below is sized to exactly
// `grid_w*block_w` x `grid_h*block_h` (no separate scale factor).
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

const BLOCK = 16;
const GRID_W = 20;
const GRID_H = 12;
const WIDTH = GRID_W * BLOCK; // 320
const HEIGHT = GRID_H * BLOCK; // 192

function buildQpGrid(qpAt: (r: number, c: number) => number) {
  const qp: number[] = [];
  let qp_min = 63;
  let qp_max = 0;
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) {
      const v = qpAt(row, col);
      qp.push(v);
      qp_min = Math.min(qp_min, v);
      qp_max = Math.max(qp_max, v);
    }
  }
  return { grid_w: GRID_W, grid_h: GRID_H, block_w: BLOCK, block_h: BLOCK, qp, qp_min, qp_max };
}

// Sharp, low-QP scene (fine detail region) -- QP 18-28
export const HighQualityLowQp = () => {
  const qp_grid = buildQpGrid((r, c) => 18 + Math.round(10 * Math.abs(Math.sin(r * 0.6 + c * 0.3))));
  const frame: any = { frame_index: 0, frame_type: "I", size: 251034, qp_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => QPMapOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// Heavily compressed scene with a bitrate-starved region -- QP 32-51
export const LowBitrateHighQp = () => {
  const qp_grid = buildQpGrid((r, c) => (c > GRID_W * 0.6 ? 44 + (r % 8) : 32 + ((r + c) % 6)));
  const frame: any = { frame_index: 88, frame_type: "P", size: 8120, qp_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => QPMapOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
