import { ReferenceOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// ReferenceOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/ReferenceRenderer.tsx). CanvasStage hosts
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

const WIDTH = 420;
const HEIGHT = 300;

export const KeyFrameNoReferences = () => {
  const frame: any = { frame_index: 0, frame_type: "I", size: 210442, ref_frames: [] };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => ReferenceOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const BFrameMultipleReferences = () => {
  const frame: any = { frame_index: 91, frame_type: "B", size: 14830, ref_frames: [88, 90, 92, 94] };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => ReferenceOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
