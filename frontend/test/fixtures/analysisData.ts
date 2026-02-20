// Test fixtures for analysis data

import type { FrameAnalysisData } from "../../types/video";

/**
 * Mock frame analysis data for different codecs
 */
export const mockFrameAnalysisData: Record<string, FrameAnalysisData> = {
  hevc: {
    frameIndex: 0,
    width: 1920,
    height: 1080,
    qpGrid: {
      gridW: 120,
      gridH: 68,
      blockW: 16,
      blockH: 16,
      qp: Array(120 * 68).fill(26),
      qpMin: 22,
      qpMax: 38,
    },
    mvGrid: {
      codedWidth: 1920,
      codedHeight: 1080,
      blockW: 16,
      blockH: 16,
      gridW: 120,
      gridH: 68,
      mvL0: Array(120 * 68).fill({ dxQpel: 0, dyQpel: 0 }),
      mvL1: Array(120 * 68).fill({ dxQpel: 0, dyQpel: 0 }),
      mode: Array(120 * 68).fill(1),
    },
    partitionGrid: {
      codedWidth: 1920,
      codedHeight: 1080,
      sbSize: 64,
      blocks: [
        { x: 0, y: 0, width: 64, height: 64, partition: 0, depth: 0 },
        { x: 64, y: 0, width: 64, height: 64, partition: 0, depth: 0 },
        { x: 0, y: 64, width: 64, height: 64, partition: 0, depth: 0 },
        { x: 64, y: 64, width: 64, height: 64, partition: 0, depth: 0 },
      ],
    },
    predictionModeGrid: {
      codedWidth: 1920,
      codedHeight: 1080,
      blockW: 16,
      blockH: 16,
      gridW: 120,
      gridH: 68,
      modes: Array(120 * 68).fill(0),
    },
    transformGrid: {
      codedWidth: 1920,
      codedHeight: 1080,
      blockW: 16,
      blockH: 16,
      gridW: 120,
      gridH: 68,
      txSizes: Array(120 * 68).fill(0),
    },
  },
  av1: {
    frameIndex: 0,
    width: 3840,
    height: 2160,
    qpGrid: {
      gridW: 240,
      gridH: 136,
      blockW: 16,
      blockH: 16,
      qp: Array(240 * 136).fill(32),
      qpMin: 20,
      qpMax: 51,
    },
    mvGrid: {
      codedWidth: 3840,
      codedHeight: 2160,
      blockW: 16,
      blockH: 16,
      gridW: 240,
      gridH: 136,
      mvL0: Array(240 * 136).fill({ dxQpel: 0, dyQpel: 0 }),
      mvL1: Array(240 * 136).fill({ dxQpel: 0, dyQpel: 0 }),
      mode: Array(240 * 136).fill(1),
    },
    partitionGrid: {
      codedWidth: 3840,
      codedHeight: 2160,
      sbSize: 128,
      blocks: [
        { x: 0, y: 0, width: 128, height: 128, partition: 0, depth: 0 },
        { x: 128, y: 0, width: 128, height: 128, partition: 0, depth: 0 },
        { x: 0, y: 128, width: 128, height: 128, partition: 0, depth: 0 },
        { x: 128, y: 128, width: 128, height: 128, partition: 0, depth: 0 },
      ],
    },
    predictionModeGrid: {
      codedWidth: 3840,
      codedHeight: 2160,
      blockW: 16,
      blockH: 16,
      gridW: 240,
      gridH: 136,
      modes: Array(240 * 136).fill(0),
    },
    transformGrid: {
      codedWidth: 3840,
      codedHeight: 2160,
      blockW: 16,
      blockH: 16,
      gridW: 240,
      gridH: 136,
      txSizes: Array(240 * 136).fill(0),
    },
  },
};

/**
 * Generate mock analysis data for a specific resolution
 */
export function generateMockAnalysisData(
  width: number,
  height: number,
  frameIndex: number = 0,
): FrameAnalysisData {
  const blockSize = 16;
  const gridW = Math.ceil(width / blockSize);
  const gridH = Math.ceil(height / blockSize);

  return {
    frameIndex,
    width,
    height,
    qpGrid: {
      gridW,
      gridH,
      blockW: blockSize,
      blockH: blockSize,
      qp: Array(gridW * gridH).fill(26),
      qpMin: 22,
      qpMax: 38,
    },
    mvGrid: {
      codedWidth: width,
      codedHeight: height,
      blockW: blockSize,
      blockH: blockSize,
      gridW,
      gridH,
      mvL0: Array(gridW * gridH).fill({ dxQpel: 0, dyQpel: 0 }),
      mvL1: Array(gridW * gridH).fill({ dxQpel: 0, dyQpel: 0 }),
      mode: Array(gridW * gridH).fill(1),
    },
    partitionGrid: {
      codedWidth: width,
      codedHeight: height,
      sbSize: 64,
      blocks: [
        { x: 0, y: 0, width: 64, height: 64, partition: 0, depth: 0 },
        { x: 64, y: 0, width: 64, height: 64, partition: 0, depth: 0 },
        { x: 0, y: 64, width: 64, height: 64, partition: 0, depth: 0 },
        { x: 64, y: 64, width: 64, height: 64, partition: 0, depth: 0 },
      ],
    },
    predictionModeGrid: {
      codedWidth: width,
      codedHeight: height,
      blockW: blockSize,
      blockH: blockSize,
      gridW,
      gridH,
      modes: Array(gridW * gridH).fill(0),
    },
    transformGrid: {
      codedWidth: width,
      codedHeight: height,
      blockW: blockSize,
      blockH: blockSize,
      gridW,
      gridH,
      txSizes: Array(gridW * gridH).fill(0),
    },
  };
}

/**
 * Mock residual analysis data
 */
export const mockResidualAnalysisData = {
  frameIndex: 0,
  width: 1920,
  height: 1080,
  coefficientStats: {
    min: 0,
    max: 255,
    mean: 45.5,
    variance: 32.8,
    energy: 1500000,
    zeroCount: 1500000,
    nonZeroCount: 600000,
  },
  blockResiduals: Array(120 * 68)
    .fill(null)
    .map((_, i) => ({
      x: (i % 120) * 16,
      y: Math.floor(i / 120) * 16,
      width: 16,
      height: 16,
      energy: Math.random() * 100,
      maxCoeff: Math.random() * 255,
      nonZeros: Math.floor(Math.random() * 256),
    })),
};

/**
 * Mock deblocking analysis data
 */
export const mockDeblockingAnalysisData = {
  frameIndex: 0,
  width: 1920,
  height: 1080,
  boundaries: Array(240)
    .fill(null)
    .map((_, i) => ({
      x: (i % 120) * 16,
      y: Math.floor(i / 120) * 8,
      length: 16,
      orientation: i % 2 === 0 ? "vertical" : "horizontal",
      strength: Math.random() * 4,
      filtered: Math.random() > 0.3,
      bs: Math.floor(Math.random() * 5),
    })),
  params: {
    betaOffset: 0,
    tcOffset: 0,
    filterStrength: 1,
    chromaEdge: true,
  },
  stats: {
    totalBoundaries: 240,
    filteredBoundaries: 168,
    strongBoundaries: 48,
    weakBoundaries: 120,
  },
};

/**
 * Mock coding flow data
 */
export const mockCodingFlowData = {
  frameIndex: 0,
  stages: [
    { id: "input", label: "Input", completed: true, dataSize: null },
    { id: "prediction", label: "Prediction", completed: true, dataSize: 1024 },
    { id: "transform", label: "Transform", completed: true, dataSize: 2048 },
    {
      id: "quantization",
      label: "Quantization",
      completed: true,
      dataSize: 512,
    },
    { id: "entropy", label: "Entropy Coding", completed: true, dataSize: 256 },
  ],
  currentStage: "prediction",
  codecFeatures: ["35 Intra Modes", "Advanced Motion Vector Pred"],
};
