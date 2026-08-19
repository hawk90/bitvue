import { Av1BlockTypeOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Av1BlockTypeOverlay is not a React component -- it's an imperative canvas draw
// function `(props: OverlayRendererProps) => void` (see
// frontend/components/panels/OverlayRenderer/renderers/Av1BlockTypeRenderer.tsx). This local
// CanvasStage hosts a <canvas>, grabs its 2D context in a layout effect (so the draw happens
// before paint), and invokes the imported function directly with synthetic props.
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
    const canvas = ref.current;
    const ctx = canvas?.getContext("2d");
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
const GRID_W = WIDTH / BLOCK; // 30
const GRID_H = HEIGHT / BLOCK; // 17

// BlockMode discriminants: 0=None 1=Inter 2=Intra 3=Skip 4=IntraBc 5=Compound
function buildModeGrid(pattern: (row: number, col: number) => number) {
  const mode: number[] = [];
  for (let row = 0; row < GRID_H; row++) {
    for (let col = 0; col < GRID_W; col++) {
      mode.push(pattern(row, col));
    }
  }
  return { grid_w: GRID_W, grid_h: GRID_H, block_w: BLOCK, block_h: BLOCK, mode };
}

function makeFrame(frameIndex: number, mvGrid: ReturnType<typeof buildModeGrid>) {
  return {
    frame_index: frameIndex,
    frame_type: "P",
    size: 48213,
    mv_grid: {
      coded_width: WIDTH,
      coded_height: HEIGHT,
      ...mvGrid,
      mv_l0: [],
      mv_l1: [],
    },
  } as unknown as Parameters<typeof Av1BlockTypeOverlay>[0]["frame"];
}

export const InterHeavy = () => {
  const frame = makeFrame(112, buildModeGrid((row, col) => {
    if ((row + col) % 11 === 0) return 3; // sprinkled SKIP
    if ((row * 7 + col) % 13 === 0) return 5; // occasional COMPOUND
    return 1; // mostly INTER
  }));
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1BlockTypeOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })}
      />
    </div>
  );
};

export const MixedIntraInterBc = () => {
  const frame = makeFrame(4, buildModeGrid((row, col) => {
    const zone = col / GRID_W;
    if (zone < 0.3) return 2; // INTRA band on the left
    if (zone < 0.4 && row % 4 === 0) return 4; // sparse INTRA_BC strip
    if ((row + col) % 5 === 0) return 3; // SKIP scattered
    return 1; // INTER elsewhere
  }));
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1BlockTypeOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })}
      />
    </div>
  );
};
