import { Av1FilmGrainRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Av1FilmGrainRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Av1FilmGrainRenderer.tsx). CanvasStage
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
const frame: any = { frame_index: 21, frame_type: "P", size: 88120 };

export const GrainEnabled = () => {
  const filmGrain = {
    enabled: true,
    seed: 4271,
    scalingShift: 10,
    arCoeffLag: 3,
    chromaScalingFromLuma: false,
    overlap: true,
  };
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1FilmGrainRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, filmGrain } as any)}
      />
    </div>
  );
};

export const GrainDisabled = () => {
  const filmGrain = { enabled: false, seed: 0, scalingShift: 8, arCoeffLag: 0, chromaScalingFromLuma: false, overlap: false };
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1FilmGrainRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, filmGrain } as any)}
      />
    </div>
  );
};
