import { MVFieldOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// MVFieldOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/MVFieldRenderer.tsx). CanvasStage hosts a
// <canvas>, grabs its 2D context in a layout effect, and invokes the function directly. No
// `webglCanvas` is passed -- the grid below stays well under WEBGL_MV_THRESHOLD so the plain 2D
// arrow path is exercised, matching most real streams' block counts.
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
const BLOCK = 32;
const GRID_W = WIDTH / BLOCK; // 15
const GRID_H = HEIGHT / BLOCK; // 8.5 -> renderer floors via loop bound, fine

export const PanningMotion = () => {
  const mv_l0 = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) {
      mv_l0.push({ dx_qpel: 24 + (row % 3) * 4, dy_qpel: -4 + (col % 2) * 2 });
    }
  }
  const frame: any = {
    frame_index: 40,
    frame_type: "P",
    size: 25100,
    mv_grid: { coded_width: WIDTH, coded_height: HEIGHT, block_w: BLOCK, block_h: BLOCK, grid_w: GRID_W, grid_h: GRID_H, mv_l0, mv_l1: [] },
  };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => MVFieldOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const IntraFrameNoMotion = () => {
  const frame: any = { frame_index: 0, frame_type: "I", size: 210044 };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => MVFieldOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
