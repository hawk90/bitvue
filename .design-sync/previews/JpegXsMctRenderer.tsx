import { JpegXsMctRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// JpegXsMctRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/JpegXsMctRenderer.tsx). CanvasStage hosts
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

export const ReversibleRct = () => {
  const mct = { mct_type: "Rct", num_comps: 3, comp_labels: ["Y", "Cg", "Co"], reversible: true };
  const frame: any = { frame_index: 0, frame_type: "I", size: 143210, mct_info: mct };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => JpegXsMctRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const IrreversibleIct = () => {
  const mct = { mct_type: "Ict", num_comps: 3, comp_labels: ["Y", "Cb", "Cr"], reversible: false };
  const frame: any = { frame_index: 5, frame_type: "I", size: 118904, mct_info: mct };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => JpegXsMctRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
