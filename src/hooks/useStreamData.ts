/**
 * Stream Data Hook
 *
 * Manages stream data from the Rust backend
 * Provides data to panels and handles updates
 */

import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '../utils/logger';

const logger = createLogger('useStreamData');

export interface FrameData {
  frame_index: number;
  frame_type: string;
  size: number;
  poc?: number;
  pts?: number;
  key_frame?: boolean;
  display_order?: number;
  coding_order: number;
  temporal_id?: number;
  spatial_id?: number;
  ref_frames?: number[];
  ref_slots?: number[];
  duration?: number;
}

export interface StreamData {
  frames: FrameData[];
  frameCount: number;
  currentFrame: FrameData | null;
}

export function useStreamData() {
  const [frames, setFrames] = useState<FrameData[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load frames from backend
  const loadFrames = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FrameData[]>('get_frames');
      setFrames(result || []);
      return result || [];
    } catch (err) {
      const errorMsg = err as string;
      setError(errorMsg);
      logger.error('Failed to load frames:', errorMsg);
      return [];
    } finally {
      setLoading(false);
    }
  }, []);

  // Get current frame data
  const getCurrentFrame = useCallback((frameIndex: number): FrameData | null => {
    return frames[frameIndex] || null;
  }, [frames]);

  // Get frame statistics
  const getFrameStats = useCallback(() => {
    const stats = {
      totalFrames: frames.length,
      frameTypes: {} as Record<string, number>,
      totalSize: 0,
      avgSize: 0,
      keyFrames: 0,
    };

    frames.forEach(frame => {
      stats.frameTypes[frame.frame_type] = (stats.frameTypes[frame.frame_type] || 0) + 1;
      stats.totalSize += frame.size;
      if (frame.key_frame) {
        stats.keyFrames++;
      }
    });

    stats.avgSize = stats.totalFrames > 0 ? stats.totalSize / stats.totalFrames : 0;

    return stats;
  }, [frames]);

  return {
    frames,
    loading,
    error,
    loadFrames,
    getCurrentFrame,
    getFrameStats,
  };
}
