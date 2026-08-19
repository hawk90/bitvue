import { Avs3CcsaoRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Avs3CcsaoRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Avs3CcsaoRenderer.tsx). CanvasStage hosts
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
const HEIGHT = 270;
const CTU = 60;
const GRID_W = Math.ceil(WIDTH / CTU);
const GRID_H = Math.ceil(HEIGHT / CTU);

export const CcsaoApplied = () => {
  const ccsao_applied: boolean[] = [];
  const luma_code: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) {
      const applied = (row + col) % 3 !== 0;
      ccsao_applied.push(applied);
      luma_code.push((row * 2 + col) % 5);
    }
  }
  const frame: any = {
    frame_index: 70,
    frame_type: "P",
    size: 42010,
    ccsao_map: { grid_w: GRID_W, grid_h: GRID_H, ctu_size: CTU, ccsao_applied, luma_code },
  };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => Avs3CcsaoRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// No ccsao_map on the frame at all -- exercises the "disabled or unavailable" branch.
export const CcsaoUnavailable = () => {
  const frame: any = { frame_index: 2, frame_type: "I", size: 190211 };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => Avs3CcsaoRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
