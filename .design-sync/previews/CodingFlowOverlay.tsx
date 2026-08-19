import { CodingFlowOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// CodingFlowOverlay is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/CodingFlowRenderer.tsx). CanvasStage
// hosts a <canvas>, grabs its 2D context in a layout effect, and invokes the function directly.
// Block x/y/width/height are consumed as raw canvas-pixel coordinates (no internal scaling), so
// the synthetic partition tree below is built directly in the canvas's own pixel space.
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
const HEIGHT = 256;
const SB = 64;

// Recursively split a superblock into a quadtree of leaf partitions, alternating split types.
function splitBlock(
  x: number,
  y: number,
  size: number,
  depth: number,
  maxDepth: number,
  out: { x: number; y: number; width: number; height: number; partition: number; depth: number }[],
) {
  const decision = (x * 7 + y * 13 + depth * 31) % 4;
  if (depth >= maxDepth || (decision === 0 && depth > 0)) {
    out.push({ x, y, width: size, height: size, partition: 0, depth });
    return;
  }
  if (decision === 1) {
    out.push({ x, y, width: size, height: size / 2, partition: 1, depth });
    out.push({ x, y: y + size / 2, width: size, height: size / 2, partition: 1, depth });
    return;
  }
  if (decision === 2) {
    out.push({ x, y, width: size / 2, height: size, partition: 2, depth });
    out.push({ x: x + size / 2, y, width: size / 2, height: size, partition: 2, depth });
    return;
  }
  const half = size / 2;
  splitBlock(x, y, half, depth + 1, maxDepth, out);
  splitBlock(x + half, y, half, depth + 1, maxDepth, out);
  splitBlock(x, y + half, half, depth + 1, maxDepth, out);
  splitBlock(x + half, y + half, half, depth + 1, maxDepth, out);
}

function buildPartitionGrid(maxDepth: number) {
  const blocks: { x: number; y: number; width: number; height: number; partition: number; depth: number }[] = [];
  for (let y = 0; y < HEIGHT; y += SB) {
    for (let x = 0; x < WIDTH; x += SB) {
      splitBlock(x, y, SB, 0, maxDepth, blocks);
    }
  }
  return { coded_width: WIDTH, coded_height: HEIGHT, sb_size: SB, blocks };
}

export const IntraFrameDeepSplit = () => {
  const frame: any = { frame_index: 0, frame_type: "I", size: 218204, partition_grid: buildPartitionGrid(3) };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => CodingFlowOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const InterFrameShallowSplit = () => {
  const frame: any = { frame_index: 74, frame_type: "P", size: 26310, partition_grid: buildPartitionGrid(1) };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => CodingFlowOverlay({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
