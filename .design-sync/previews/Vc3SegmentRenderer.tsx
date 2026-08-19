import { Vc3SegmentRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// Vc3SegmentRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/Vc3SegmentRenderer.tsx). CanvasStage
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
const MB = 16;
const COLS = Math.floor(WIDTH / MB);
const ROWS = Math.floor(HEIGHT / MB);

function buildMbGrid(bitAt: (r: number, c: number) => number) {
  const bits: number[] = [];
  for (let row = 0; row < ROWS; row++) {
    for (let col = 0; col < COLS; col++) bits.push(bitAt(row, col));
  }
  return { cols: COLS, rows: ROWS, bits };
}

export const DnxhdBroadcast422 = () => {
  const mb_grid = buildMbGrid((r, c) => 400 + ((r * 5 + c * 3) % 300));
  const frame: any = {
    frame_index: 10,
    frame_type: "I",
    size: 918200,
    mb_grid,
    comp_id: "DNxHD 1080p 185Mbps",
    chroma_sampling: 422,
    bits_per_component: 8,
    is_dnxhr: false,
  };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => Vc3SegmentRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const DnxhrHq444 = () => {
  const mb_grid = buildMbGrid((r, c) => 200 + ((r * 7 + c) % 800));
  const frame: any = {
    frame_index: 3,
    frame_type: "I",
    size: 1542000,
    mb_grid,
    comp_id: "DNxHR HQX",
    chroma_sampling: 444,
    bits_per_component: 10,
    is_dnxhr: true,
  };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => Vc3SegmentRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
