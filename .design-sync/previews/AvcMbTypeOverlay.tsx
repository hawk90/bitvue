import { AvcMbTypeOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// AvcMbTypeOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/AvcMbTypeRenderer.tsx). CanvasStage hosts
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

function buildMbTypes(typeAt: (r: number, c: number) => number) {
  const mb_types: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) mb_types.push(typeAt(row, col));
  }
  return { grid_w: GRID_W, grid_h: GRID_H, block_w: BLOCK, block_h: BLOCK, mb_types };
}

// I-frame: only intra macroblock types (0=I4x4, 1=I16x16, 2=IPCM sprinkled)
export const IntraFrame = () => {
  const mb_type_grid = buildMbTypes((r, c) => {
    if ((r * 5 + c) % 23 === 0) return 2; // rare IPCM
    return (r + c) % 3 === 0 ? 0 : 1;
  });
  const frame: any = { frame_index: 0, frame_type: "I", size: 184320, mb_type_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => AvcMbTypeOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

// B-frame: full spread of P/B modes plus PSkip/BSkip
export const BFrameMixed = () => {
  const mb_type_grid = buildMbTypes((r, c) => (r * 7 + c * 3) % 12);
  const frame: any = { frame_index: 45, frame_type: "B", size: 21044, mb_type_grid };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => AvcMbTypeOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
