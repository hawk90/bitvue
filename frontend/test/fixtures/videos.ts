// Test fixtures for video data

import type { VideoInfo, StreamInfo } from '../../types/video';

/**
 * Mock video info for different codecs
 */
export const mockVideoInfo = {
  hevc: {
    codec: 'hevc',
    width: 1920,
    height: 1080,
    frameRate: 30,
    bitDepth: 8,
    profile: 'Main',
    level: 4.1,
    chromaFormat: '4:2:0',
  },
  avc: {
    codec: 'avc',
    width: 1280,
    height: 720,
    frameRate: 30,
    bitDepth: 8,
    profile: 'High',
    level: 4.0,
    chromaFormat: '4:2:0',
  },
  av1: {
    codec: 'av1',
    width: 3840,
    height: 2160,
    frameRate: 60,
    bitDepth: 10,
    profile: 'Main',
    level: undefined,
    chromaFormat: '4:2:0',
  },
  vp9: {
    codec: 'vp9',
    width: 1920,
    height: 1080,
    frameRate: 30,
    bitDepth: 8,
    profile: 'Profile 0',
    level: undefined,
    chromaFormat: '4:2:0',
  },
  vvc: {
    codec: 'vvc',
    width: 3840,
    height: 2160,
    frameRate: 60,
    bitDepth: 10,
    profile: 'Main',
    level: 5.0,
    chromaFormat: '4:2:0',
  },
  av3: {
    codec: 'av3',
    width: 1920,
    height: 1080,
    frameRate: 30,
    bitDepth: 10,
    profile: 'Main',
    level: undefined,
    chromaFormat: '4:2:0',
  },
} as const;

/**
 * Mock stream info
 */
export const mockStreamInfo = {
  basic: {
    codec: 'hevc',
    width: 1920,
    height: 1080,
    frameRate: 30,
    bitDepth: 8,
    profile: 'Main',
    level: 4.1,
    numFrames: 100,
    duration: 3.33,
  },
  highRes: {
    codec: 'av1',
    width: 3840,
    height: 2160,
    frameRate: 60,
    bitDepth: 10,
    profile: 'Main',
    level: undefined,
    numFrames: 300,
    duration: 5.0,
  },
  lowRes: {
    codec: 'avc',
    width: 640,
    height: 480,
    frameRate: 24,
    bitDepth: 8,
    profile: 'Baseline',
    level: 3.0,
    numFrames: 50,
    duration: 2.08,
  },
};

/**
 * Video file paths for testing
 */
export const mockVideoPaths = {
  hevc: '/test/path/video.hevc',
  avc: '/test/path/video.avc',
  av1: '/test/path/video.ivf',
  vp9: '/test/path/video.webm',
  vvc: '/test/path/video.vvc',
  mkv: '/test/path/video.mkv',
  mp4: '/test/path/video.mp4',
};

/**
 * Codec detection test cases
 */
export const codecDetectionTests = [
  { input: 'video.hevc', expected: 'hevc' },
  { input: 'video.h265', expected: 'hevc' },
  { input: 'video.avc', expected: 'avc' },
  { input: 'video.h264', expected: 'avc' },
  { input: 'video.264', expected: 'avc' },
  { input: 'video.av1', expected: 'av1' },
  { input: 'video.ivf', expected: 'av1' },
  { input: 'video.vp9', expected: 'vp9' },
  { input: 'video.webm', expected: 'vp9' },
  { input: 'video.vvc', expected: 'vvc' },
  { input: 'video.h266', expected: 'vvc' },
  { input: 'video.266', expected: 'vvc' },
  { input: 'video.av3', expected: 'av3' },
];
