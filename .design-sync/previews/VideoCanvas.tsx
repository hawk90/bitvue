import { VideoCanvas } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 420,
  height: 360,
};

// VideoCanvas draws directly to a <canvas> 2D context on mount/update using the `yuvData` prop (the
// real in-app path -- decoded frames arrive as raw YUV planes via `getDecodedFrameYuv`). No bridge
// mocking needed here since rendering is prop-driven, not fetched internally. Per the task notes, a
// small synthetic buffer is enough -- we don't attempt a photorealistic decoded frame, just plausible
// solid/gradient planes at a real clip resolution (320x240, matching bitvue's own AOM test fixtures).

const WIDTH = 320;
const HEIGHT = 240;

function makeSolidYuvFrame(yFill: number, uFill: number, vFill: number) {
  const yStride = WIDTH;
  const uStride = WIDTH / 2;
  const vStride = WIDTH / 2;
  return {
    y: new Uint8Array(yStride * HEIGHT).fill(yFill),
    u: new Uint8Array(uStride * (HEIGHT / 2)).fill(uFill),
    v: new Uint8Array(vStride * (HEIGHT / 2)).fill(vFill),
    width: WIDTH,
    height: HEIGHT,
    yStride,
    uStride,
    vStride,
    chromaSubsampling: "420" as const,
  };
}

function makeGradientYuvFrame() {
  const yStride = WIDTH;
  const uStride = WIDTH / 2;
  const vStride = WIDTH / 2;
  const y = new Uint8Array(yStride * HEIGHT);
  for (let py = 0; py < HEIGHT; py++) {
    for (let px = 0; px < WIDTH; px++) {
      y[py * yStride + px] = Math.round((px / WIDTH) * 200 + (py / HEIGHT) * 30);
    }
  }
  const u = new Uint8Array(uStride * (HEIGHT / 2)).fill(118);
  const v = new Uint8Array(vStride * (HEIGHT / 2)).fill(140);
  return {
    y,
    u,
    v,
    width: WIDTH,
    height: HEIGHT,
    yStride,
    uStride,
    vStride,
    chromaSubsampling: "420" as const,
  };
}

function buildQpGrid() {
  const block = 8;
  const grid_w = WIDTH / block;
  const grid_h = HEIGHT / block;
  const qp: number[] = [];
  for (let gy = 0; gy < grid_h; gy++) {
    for (let gx = 0; gx < grid_w; gx++) {
      qp.push(16 + ((gx * 3 + gy * 7) % 36));
    }
  }
  return { grid_w, grid_h, block_w: block, block_h: block, qp, qp_min: 16, qp_max: 51 };
}

function buildPartitionGrid() {
  const blocks: Array<{
    x: number;
    y: number;
    width: number;
    height: number;
    partition: number;
    depth: number;
  }> = [];
  for (let y = 0; y < HEIGHT; y += 32) {
    for (let x = 0; x < WIDTH; x += 32) {
      const splitLeft = (x / 32 + y / 32) % 3 === 0;
      if (splitLeft) {
        blocks.push({ x, y, width: 16, height: 16, partition: 3, depth: 2 });
        blocks.push({ x: x + 16, y, width: 16, height: 16, partition: 3, depth: 2 });
        blocks.push({ x, y: y + 16, width: 16, height: 16, partition: 3, depth: 2 });
        blocks.push({ x: x + 16, y: y + 16, width: 16, height: 16, partition: 3, depth: 2 });
      } else {
        blocks.push({ x, y, width: 32, height: 32, partition: 0, depth: 1 });
      }
    }
  }
  return { coded_width: WIDTH, coded_height: HEIGHT, sb_size: 64, blocks };
}

const noop = () => {};

export const Default = () => (
  <div style={previewBg}>
    <VideoCanvas
      frameImage={null}
      currentFrameIndex={0}
      currentFrame={{ frame_index: 0, frame_type: "KEY", size: 42000, width: WIDTH, height: HEIGHT }}
      currentMode={"yuv" as never}
      zoom={1}
      pan={{ x: 0, y: 0 }}
      onWheel={noop}
      onMouseDown={noop}
      onMouseMove={noop}
      onMouseUp={noop}
      isDragging={false}
      yuvData={makeSolidYuvFrame(96, 128, 128) as never}
    />
  </div>
);

export const ZoomedAndPanned = () => (
  <div style={previewBg}>
    <VideoCanvas
      frameImage={null}
      currentFrameIndex={47}
      currentFrame={{ frame_index: 47, frame_type: "P", size: 14200, width: WIDTH, height: HEIGHT }}
      currentMode={"yuv" as never}
      zoom={2.5}
      pan={{ x: 80, y: -30 }}
      onWheel={noop}
      onMouseDown={noop}
      onMouseMove={noop}
      onMouseUp={noop}
      isDragging={false}
      yuvData={makeGradientYuvFrame() as never}
    />
  </div>
);

export const QpMapOverlay = () => (
  <div style={previewBg}>
    <VideoCanvas
      frameImage={null}
      currentFrameIndex={12}
      currentFrame={{
        frame_index: 12,
        frame_type: "P",
        size: 18400,
        width: WIDTH,
        height: HEIGHT,
        qp_grid: buildQpGrid() as never,
      }}
      currentMode={"yuv" as never}
      zoom={1}
      pan={{ x: 0, y: 0 }}
      onWheel={noop}
      onMouseDown={noop}
      onMouseMove={noop}
      onMouseUp={noop}
      isDragging={false}
      yuvData={makeGradientYuvFrame() as never}
      activeOverlays={new Set(["qp-map"]) as never}
    />
  </div>
);

export const CodingFlowDragging = () => (
  <div style={previewBg}>
    <VideoCanvas
      frameImage={null}
      currentFrameIndex={5}
      currentFrame={{
        frame_index: 5,
        frame_type: "B",
        size: 6100,
        width: WIDTH,
        height: HEIGHT,
        partition_grid: buildPartitionGrid() as never,
      }}
      currentMode={"coding-flow" as never}
      zoom={1}
      pan={{ x: 0, y: 0 }}
      onWheel={noop}
      onMouseDown={noop}
      onMouseMove={noop}
      onMouseUp={noop}
      isDragging={true}
      yuvData={makeSolidYuvFrame(64, 128, 128) as never}
    />
  </div>
);
