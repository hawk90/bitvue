import { VvcInverseMapRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// VvcInverseMapRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/VvcInverseMapRenderer.tsx). CanvasStage
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
const GRID_W = 24;
const GRID_H = 14;

function buildQp(qpAt: (r: number, c: number) => number) {
  const qp: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) qp.push(qpAt(row, col));
  }
  return { grid_w: GRID_W, grid_h: GRID_H, block_w: WIDTH / GRID_W, block_h: HEIGHT / GRID_H, qp, qp_min: 16, qp_max: 42 };
}

// Bright highlight region (low QP -> positive LMCS shift) against a dim midtone background
export const BrightHighlightRegion = () => {
  const qp_grid = buildQp((r, c) => (r < GRID_H * 0.35 && c > GRID_W * 0.5 ? 16 : 28 + ((r + c) % 6)));
  const frame: any = { frame_index: 18, frame_type: "I", size: 178200, qp_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => VvcInverseMapRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// Dark shadow region (high QP -> negative LMCS shift) with scattered midtone noise
export const DarkShadowRegion = () => {
  const qp_grid = buildQp((r, c) => (r > GRID_H * 0.6 ? 42 : 24 + ((r * 3 + c) % 8)));
  const frame: any = { frame_index: 60, frame_type: "P", size: 21044, qp_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => VvcInverseMapRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
