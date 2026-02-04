// Test fixtures for Bitvue tests
// These provide consistent mock data for unit and integration tests

import type { FrameInfo, VideoInfo, StreamInfo, UnitData } from '../../types/video';

/**
 * Mock frame data for testing
 */
export const mockFrames: FrameInfo[] = [
  {
    frameNumber: 0,
    frameType: 'I',
    poc: 0,
    pts: 0,
    size: 50000,
    qp: 26,
    temporalId: 0,
    spatialId: 0,
    refFrames: [],
    refSlots: [],
    nalType: 'IDR',
  },
  {
    frameNumber: 1,
    frameType: 'P',
    poc: 1,
    pts: 1,
    size: 25000,
    qp: 28,
    temporalId: 0,
    spatialId: 0,
    refFrames: [0],
    refSlots: [0],
    nalType: 'TRAIL_R',
  },
  {
    frameNumber: 2,
    frameType: 'B',
    poc: 2,
    pts: 2,
    size: 15000,
    qp: 30,
    temporalId: 1,
    spatialId: 0,
    refFrames: [0, 1],
    refSlots: [0, 1],
    nalType: 'TRAIL_R',
  },
  {
    frameNumber: 3,
    frameType: 'B',
    poc: 3,
    pts: 3,
    size: 15000,
    qp: 30,
    temporalId: 1,
    spatialId: 0,
    refFrames: [1, 4],
    refSlots: [1, 2],
    nalType: 'TRAIL_R',
  },
  {
    frameNumber: 4,
    frameType: 'P',
    poc: 4,
    pts: 4,
    size: 28000,
    qp: 27,
    temporalId: 0,
    spatialId: 0,
    refFrames: [1],
    refSlots: [1],
    nalType: 'TRAIL_R',
  },
  {
    frameNumber: 5,
    frameType: 'I',
    poc: 5,
    pts: 5,
    size: 52000,
    qp: 26,
    temporalId: 0,
    spatialId: 0,
    refFrames: [],
    refSlots: [],
    nalType: 'IDR',
  },
];

/**
 * Generate mock frames for a given count
 */
export function generateMockFrames(count: number): FrameInfo[] {
  const frames: FrameInfo[] = [];
  let gopCounter = 0;

  for (let i = 0; i < count; i++) {
    const isI = i % 12 === 0;
    const isP = !isI && i % 3 === 0;

    frames.push({
      frameNumber: i,
      frameType: isI ? 'I' : isP ? 'P' : 'B',
      poc: i,
      pts: i,
      size: isI ? 50000 + Math.floor(Math.random() * 5000) :
        isP ? 25000 + Math.floor(Math.random() * 5000) :
        15000 + Math.floor(Math.random() * 3000),
      qp: 26 + Math.floor(Math.random() * 10),
      temporalId: isI ? 0 : (isP ? 0 : 1),
      spatialId: 0,
      refFrames: isI ? [] : isP ? [i - 3] : [i - 3, i],
      refSlots: isI ? [] : isP ? [0] : [0, 1],
      nalType: isI ? 'IDR' : 'TRAIL_R',
    });
  }

  return frames;
}

/**
 * Mock video info
 */
export const mockVideoInfo: VideoInfo = {
  codec: 'hevc',
  width: 1920,
  height: 1080,
  frameRate: 30,
  bitDepth: 8,
  profile: 'Main',
  level: 4.1,
  chromaFormat: '4:2:0',
};

/**
 * Mock stream info
 */
export const mockStreamInfo: StreamInfo = {
  codec: 'hevc',
  width: 1920,
  height: 1080,
  frameRate: 30,
  bitDepth: 8,
  profile: 'Main',
  level: 4.1,
  numFrames: 100,
  duration: 3.33,
};

/**
 * Mock unit data
 */
export const mockUnitData: UnitData = {
  offset: 0,
  size: 150000,
  data: new Uint8Array(150000),
  type: 'frame',
  width: 1920,
  height: 1080,
  poc: 0,
  temporalId: 0,
  spatialId: 0,
  typeId: 1,
};

/**
 * Mock analysis data
 */
export const mockAnalysisData = {
  frameIndex: 0,
  width: 1920,
  height: 1080,
  qpGrid: {
    gridW: 120,
    gridH: 68,
    blockW: 16,
    blockH: 16,
    qp: new Array(120 * 68).fill(26),
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
    mvL0: new Array(120 * 68).fill({ dxQpel: 0, dyQpel: 0 }),
    mvL1: new Array(120 * 68).fill({ dxQpel: 0, dyQpel: 0 }),
    mode: new Array(120 * 68).fill(1),
  },
  partitionGrid: {
    codedWidth: 1920,
    codedHeight: 1080,
    sbSize: 64,
    blocks: [
      { x: 0, y: 0, width: 64, height: 64, partition: 0, depth: 0 },
      { x: 64, y: 0, width: 64, height: 64, partition: 0, depth: 0 },
      { x: 0, y: 64, width: 64, height: 64, partition: 0, depth: 0 },
    ],
  },
};

/**
 * Mock thumbnails map
 */
export const mockThumbnails = new Map<number, string>([
  [0, 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYwIiBoZWlnaHQ9IjkwIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxyZWN0IHdpZHRoPSIxNjAiIGhlaWdodD0iOTAiIGZpbGw9IiNmZjZiNmIiLz48L3N2Zz4='],
  [5, 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYwIiBoZWlnaHQ9IjkwIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxyZWN0IHdpZHRoPSIxNjAiIGhlaWdodD0iOTAiIGZpbGw9IiM1MWNmNjYiLz48L3N2Zz4='],
]);

/**
 * Create a loading thumbnails set
 */
export function createLoadingThumbnails(indices: number[]): Set<number> {
  return new Set(indices);
}

/**
 * Mock file info
 */
export const mockFileInfo = {
  path: '/path/to/video.hevc',
  size: 15000000,
  codec: 'hevc',
  success: true,
  error: undefined,
};

/**
 * Mock error file info
 */
export const mockErrorFileInfo = {
  path: '/path/to/invalid.video',
  size: 0,
  codec: 'unknown',
  success: false,
  error: 'Failed to open file',
};

/**
 * Test frame sequences with specific patterns
 */
export const testFrameSequences = {
  gop12: generateMockFrames(12),
  allI: Array(10).fill(null).map((_, i) => ({
    frameNumber: i,
    frameType: 'I' as const,
    poc: i,
    pts: i,
    size: 50000,
    qp: 26,
    temporalId: 0,
    spatialId: 0,
    refFrames: [],
    refSlots: [],
    nalType: 'IDR' as const,
  })),
  highQp: Array(10).fill(null).map((_, i) => ({
    frameNumber: i,
    frameType: 'P' as const,
    poc: i,
    pts: i,
    size: 25000,
    qp: 45 + (i % 5),
    temporalId: 0,
    spatialId: 0,
    refFrames: i > 0 ? [i - 1] : [],
    refSlots: i > 0 ? [0] : [],
    nalType: 'TRAIL_R' as const,
  })),
};
