import { JpegXsTransformRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// JpegXsTransformRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/JpegXsTransformRenderer.tsx). CanvasStage
// hosts a <canvas>, grabs its 2D context in a layout effect, and invokes the function directly.
// `nodes[].x/y/w/h` are normalized [0,1] fractions of the canvas.
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

// Classic 2-level dyadic wavelet quadtree: LL nests in the top-left corner, each level's
// LH/HL/HH bands wrap around it.
function twoLevelNodes() {
  return [
    { name: "LL", x: 0, y: 0, w: 0.25, h: 0.25, level: 1 },
    { name: "HL2", x: 0.25, y: 0, w: 0.25, h: 0.25, level: 1 },
    { name: "LH2", x: 0, y: 0.25, w: 0.25, h: 0.25, level: 1 },
    { name: "HH2", x: 0.25, y: 0.25, w: 0.25, h: 0.25, level: 1 },
    { name: "HL1", x: 0.5, y: 0, w: 0.5, h: 0.5, level: 0 },
    { name: "LH1", x: 0, y: 0.5, w: 0.5, h: 0.5, level: 0 },
    { name: "HH1", x: 0.5, y: 0.5, w: 0.5, h: 0.5, level: 0 },
  ];
}

// 3-level decomposition adds a finer level-2 subdivision within the LL band.
function threeLevelNodes() {
  return [
    { name: "LL", x: 0, y: 0, w: 0.125, h: 0.125, level: 2 },
    { name: "HL3", x: 0.125, y: 0, w: 0.125, h: 0.125, level: 2 },
    { name: "LH3", x: 0, y: 0.125, w: 0.125, h: 0.125, level: 2 },
    { name: "HH3", x: 0.125, y: 0.125, w: 0.125, h: 0.125, level: 2 },
    { name: "HL2", x: 0.25, y: 0, w: 0.25, h: 0.25, level: 1 },
    { name: "LH2", x: 0, y: 0.25, w: 0.25, h: 0.25, level: 1 },
    { name: "HH2", x: 0.25, y: 0.25, w: 0.25, h: 0.25, level: 1 },
    { name: "HL1", x: 0.5, y: 0, w: 0.5, h: 0.5, level: 0 },
    { name: "LH1", x: 0, y: 0.5, w: 0.5, h: 0.5, level: 0 },
    { name: "HH1", x: 0.5, y: 0.5, w: 0.5, h: 0.5, level: 0 },
  ];
}

export const TwoLevelDecomp = () => {
  const map = { decomp_h: 2, decomp_v: 2, nodes: twoLevelNodes() };
  const frame: any = { frame_index: 0, frame_type: "I", size: 143210, transform_map: map };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => JpegXsTransformRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const ThreeLevelDecomp = () => {
  const map = { decomp_h: 3, decomp_v: 3, nodes: threeLevelNodes() };
  const frame: any = { frame_index: 6, frame_type: "I", size: 161200, transform_map: map };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => JpegXsTransformRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
