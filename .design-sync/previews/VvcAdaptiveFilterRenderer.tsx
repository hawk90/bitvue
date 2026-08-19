import { VvcAdaptiveFilterRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// VvcAdaptiveFilterRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/VvcAdaptiveFilterRenderer.tsx).
// CanvasStage hosts a <canvas>, grabs its 2D context in a layout effect, and invokes the
// function directly. Note it derives cell size from `width/grid_w` and `height/grid_h` directly
// (no block_w/block_h), so any grid_w/grid_h works against the fixed canvas size below.
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
const GRID_W = 24;
const GRID_H = 14;

function buildQp(qpAt: (r: number, c: number) => number) {
  const qp: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) qp.push(qpAt(row, col));
  }
  return { grid_w: GRID_W, grid_h: GRID_H, block_w: WIDTH / GRID_W, block_h: HEIGHT / GRID_H, qp, qp_min: 18, qp_max: 40 };
}

export const AlfMostlyApplied = () => {
  const qp_grid = buildQp((r, c) => 20 + ((r + c) % 6));
  const frame: any = { frame_index: 40, frame_type: "P", size: 41200, qp_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => VvcAdaptiveFilterRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const AlfSparseHighQpRegion = () => {
  const qp_grid = buildQp((r, c) => (c > GRID_W * 0.5 ? 38 + (r % 5) : 20 + (c % 4)));
  const frame: any = { frame_index: 5, frame_type: "I", size: 189200, qp_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => VvcAdaptiveFilterRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
