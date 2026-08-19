import { Av1SuperResRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Av1SuperResRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Av1SuperResRenderer.tsx). CanvasStage
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
const frame: any = { frame_index: 14, frame_type: "P", size: 39120 };

export const SuperResEnabled = () => {
  // downscaledWidth = upscaledWidth * 8 / scaleDenominator (AV1 spec) -- must land inside the
  // preview's WIDTH=480 canvas, not real 1920px stream dimensions, or the divider line draws
  // off-canvas and the story looks identical to "disabled".
  const superResolution = { enabled: true, scaleDenominator: 12, upscaledWidth: WIDTH, upscaledHeight: HEIGHT };
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1SuperResRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, superResolution } as any)}
      />
    </div>
  );
};

export const SuperResDisabled = () => {
  const superResolution = { enabled: false, scaleDenominator: 8, upscaledWidth: 1920, upscaledHeight: 1080 };
  return (
    <div style={previewBg}>
      <CanvasStage
        width={WIDTH}
        height={HEIGHT}
        draw={(ctx) => Av1SuperResRenderer({ ctx, width: WIDTH, height: HEIGHT, frame, superResolution } as any)}
      />
    </div>
  );
};
