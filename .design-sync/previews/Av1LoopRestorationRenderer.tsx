import { Av1LoopRestorationRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Av1LoopRestorationRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Av1LoopRestorationRenderer.tsx).
// CanvasStage hosts a <canvas>, grabs its 2D context in a layout effect, and invokes the
// function directly.
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
const UNIT = 60;

function buildUnits(typeAt: (r: number, c: number) => number) {
  const cols = Math.ceil(WIDTH / UNIT);
  const rows = Math.ceil(HEIGHT / UNIT);
  const units = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      units.push({ x: c * UNIT, y: r * UNIT, size: UNIT, restorationType: typeAt(r, c) });
    }
  }
  return { width: WIDTH, height: HEIGHT, unitSize: UNIT, yType: 3, units };
}

const frame: any = { frame_index: 60, frame_type: "P", size: 54012 };

export const MixedRestorationTypes = () => {
  const loopRestoration = buildUnits((r, c) => (r + c) % 4);
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1LoopRestorationRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, loopRestoration } as any)}
      />
    </div>
  );
};

export const MostlyWiener = () => {
  const loopRestoration = buildUnits((r, c) => ((r * 3 + c) % 7 === 0 ? 2 : 1));
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1LoopRestorationRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, loopRestoration } as any)}
      />
    </div>
  );
};
