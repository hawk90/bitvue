import { VvcDualTreeRenderer } from "bitvue";
import type { CSSProperties } from "react";
import { useLayoutEffect, useRef } from "react";

// VvcDualTreeRenderer is an imperative canvas draw function, not a React component (see
// frontend/components/panels/OverlayRenderer/renderers/VvcDualTreeRenderer.tsx). CanvasStage
// hosts a <canvas>, grabs its 2D context in a layout effect, and invokes the function directly.
// It scales block coords by `width/coded_width`, `height/coded_height`, so coded_* is set equal
// to the canvas size below (scale factor 1) to keep the synthetic block coordinates simple.
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

// tree_type: 0/1 = luma (single/dual-tree), 2 = chroma
function buildDualTreeBlocks() {
  const blocks: { x: number; y: number; width: number; height: number; depth: number; tree_type: number }[] = [];
  for (let y = 0; y < HEIGHT; y += SB) {
    for (let x = 0; x < WIDTH; x += SB) {
      // Luma tree: split into 2 vertical halves
      blocks.push({ x, y, width: SB / 2, height: SB, depth: 1, tree_type: 1 });
      blocks.push({ x: x + SB / 2, y, width: SB / 2, height: SB, depth: 1, tree_type: 1 });
      // Chroma tree: single 4-way split overlapping the same area for a separate-tree look
      const half = SB / 2;
      blocks.push({ x, y, width: half, height: half, depth: 2, tree_type: 2 });
      blocks.push({ x: x + half, y: y + half, width: half, height: half, depth: 2, tree_type: 2 });
    }
  }
  return blocks;
}

function buildDepthOnlyBlocks() {
  const blocks: { x: number; y: number; width: number; height: number; depth: number }[] = [];
  for (let y = 0; y < HEIGHT; y += SB) {
    for (let x = 0; x < WIDTH; x += SB) {
      const half = SB / 2;
      blocks.push({ x, y, width: half, height: half, depth: 1 });
      blocks.push({ x: x + half, y, width: half, height: half, depth: 2 });
      blocks.push({ x, y: y + half, width: half, height: half, depth: 2 });
      blocks.push({ x: x + half, y: y + half, width: half, height: half, depth: 3 });
    }
  }
  return blocks;
}

export const LumaChromaDualTree = () => {
  const frame: any = {
    frame_index: 0,
    frame_type: "I",
    size: 210442,
    partition_grid: { coded_width: WIDTH, coded_height: HEIGHT, sb_size: SB, blocks: buildDualTreeBlocks() },
  };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => VvcDualTreeRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};

export const FallbackDepthShading = () => {
  const frame: any = {
    frame_index: 30,
    frame_type: "P",
    size: 22104,
    partition_grid: { coded_width: WIDTH, coded_height: HEIGHT, sb_size: SB, blocks: buildDepthOnlyBlocks() },
  };
  return (
    <div style={previewBg}>
      <CanvasStage width={WIDTH} height={HEIGHT} draw={(ctx) => VvcDualTreeRenderer({ ctx, width: WIDTH, height: HEIGHT, frame })} />
    </div>
  );
};
