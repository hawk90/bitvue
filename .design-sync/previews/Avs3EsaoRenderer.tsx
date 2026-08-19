import { Avs3EsaoRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Avs3EsaoRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Avs3EsaoRenderer.tsx). CanvasStage hosts a
// <canvas>, grabs its 2D context in a layout effect, and invokes the function directly.
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

export const EsaoAllClasses = () => {
  const esao_type: number[] = [];
  const esao_class: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) {
      esao_type.push((row * GRID_W + col) % 6);
      esao_class.push((row + col) % 3);
    }
  }
  const frame: any = {
    frame_index: 33,
    frame_type: "P",
    size: 39810,
    esao_map: { grid_w: GRID_W, grid_h: GRID_H, ctu_size: CTU, esao_type, esao_class },
  };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => Avs3EsaoRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// No esao_map on the frame -- exercises the "disabled or unavailable" branch.
export const EsaoUnavailable = () => {
  const frame: any = { frame_index: 1, frame_type: "I", size: 202113 };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => Avs3EsaoRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
