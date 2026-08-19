import { TransformOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// TransformOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/TransformRenderer.tsx). CanvasStage hosts
// a <canvas>, grabs its 2D context in a layout effect, and invokes the function directly.
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
const HEIGHT = 256;
const BLOCK = 32;
const GRID_W = WIDTH / BLOCK;
const GRID_H = HEIGHT / BLOCK;

function buildTxSizes(txAt: (r: number, c: number) => number) {
  const tx_sizes: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) tx_sizes.push(txAt(row, col));
  }
  return { coded_width: WIDTH, coded_height: HEIGHT, block_w: BLOCK, block_h: BLOCK, grid_w: GRID_W, grid_h: GRID_H, tx_sizes };
}

// Flat, low-detail region -- large 32x32/64x64 transforms dominate
export const LargeTransformsFlatScene = () => {
  const grid = buildTxSizes((r, c) => ((r + c) % 6 === 0 ? 4 : 3));
  const frame: any = { frame_index: 15, frame_type: "P", size: 9820, transform_grid: grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => TransformOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// High-detail region -- small 4x4/8x8 transforms dominate
export const SmallTransformsDetailedScene = () => {
  const grid = buildTxSizes((r, c) => (r * 3 + c) % 5);
  const frame: any = { frame_index: 0, frame_type: "I", size: 254100, transform_grid: grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => TransformOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
