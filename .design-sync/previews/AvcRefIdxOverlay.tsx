import { AvcRefIdxOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// AvcRefIdxOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/AvcRefIdxRenderer.tsx). CanvasStage hosts
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
const HEIGHT = 272;
const BLOCK = 16;
const GRID_W = WIDTH / BLOCK;
const GRID_H = HEIGHT / BLOCK;

function buildRefIdx(refAt: (r: number, c: number) => number | null) {
  const ref_idx_l0: (number | null)[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) ref_idx_l0.push(refAt(row, col));
  }
  return { grid_w: GRID_W, grid_h: GRID_H, block_w: BLOCK, block_h: BLOCK, ref_idx_l0, ref_idx_l1: [] };
}

// P-frame referencing mostly ref0 (previous frame) with a scattered ref1/ref2 and an intra patch
export const PFrameFewRefs = () => {
  const ref_idx_grid = buildRefIdx((r, c) => {
    if (c < GRID_W * 0.15) return null; // intra strip on the left edge
    if ((r + c) % 9 === 0) return 1;
    if ((r * 3 + c) % 17 === 0) return 2;
    return 0;
  });
  const frame: any = { frame_index: 30, frame_type: "P", size: 24310, ref_idx_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => AvcRefIdxOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// Long-GOP frame drawing from up to 6 reference slots
export const ManyReferenceSlots = () => {
  const ref_idx_grid = buildRefIdx((r, c) => (r + c) % 7 === 6 ? null : (r * 5 + c) % 6);
  const frame: any = { frame_index: 118, frame_type: "P", size: 18920, ref_idx_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => AvcRefIdxOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
