import { Av1CdefRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Av1CdefRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Av1CdefRenderer.tsx). CanvasStage hosts a
// <canvas>, grabs its 2D context in a layout effect (runs before paint), and invokes the
// imported function directly with synthetic props.
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
const BLOCK_SIZE = 60;

function buildCdef(strengthAt: (r: number, c: number) => number) {
  const cols = Math.ceil(WIDTH / BLOCK_SIZE);
  const rows = Math.ceil(HEIGHT / BLOCK_SIZE);
  const blocks = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      blocks.push({
        x: c * BLOCK_SIZE,
        y: r * BLOCK_SIZE,
        size: BLOCK_SIZE,
        direction: (r * cols + c) % 8,
        strength: strengthAt(r, c),
      });
    }
  }
  return {
    width: WIDTH,
    height: HEIGHT,
    blockSize: BLOCK_SIZE,
    blocks,
    damping: 4,
    yPrimaryStrength: 6,
    ySecondaryStrength: 2,
  };
}

const frame: any = { frame_index: 30, frame_type: "P", size: 61204 };

export const StrongFiltering = () => {
  const cdef = buildCdef((r, c) => 30 + ((r * 37 + c * 53) % 200));
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1CdefRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, cdef } as any)}
      />
    </div>
  );
};

export const MostlyUnfiltered = () => {
  const cdef = buildCdef((r, c) => ((r + c) % 5 === 0 ? 90 : 0));
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1CdefRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, cdef } as any)}
      />
    </div>
  );
};
