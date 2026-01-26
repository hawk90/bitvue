/**
 * Compare Context - Manages A/B compare workspace state
 */

import { createContext, useContext, useState, useCallback, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type {
  CompareWorkspace,
  SyncMode,
  AlignmentQuality,
} from '../types/video';

interface CompareContextType {
  // Compare workspace state
  workspace: CompareWorkspace | null;
  isLoading: boolean;
  error: string | null;

  // Stream paths
  pathA: string | null;
  pathB: string | null;

  // Current frames for each stream
  currentFrameA: number;
  currentFrameB: number;

  // Actions
  createWorkspace: (pathA: string, pathB: string) => Promise<void>;
  closeWorkspace: () => void;
  setFrameA: (index: number) => void;
  setFrameB: (index: number) => void;
  setSyncMode: (mode: SyncMode) => Promise<void>;
  setManualOffset: (offset: number) => Promise<void>;
  resetOffset: () => Promise<void>;
  getAlignedFrame: (streamAIdx: number) => Promise<{ bIdx: number | null; quality: AlignmentQuality }>;
}

const CompareContext = createContext<CompareContextType | null>(null);

export function CompareProvider({ children }: { children: ReactNode }) {
  const [workspace, setWorkspace] = useState<CompareWorkspace | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pathA, setPathA] = useState<string | null>(null);
  const [pathB, setPathB] = useState<string | null>(null);
  const [currentFrameA, setCurrentFrameA] = useState(0);
  const [currentFrameB, setCurrentFrameB] = useState(0);

  const createWorkspace = useCallback(async (pA: string, pB: string) => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await invoke<CompareWorkspace>('create_compare_workspace', {
        pathA: pA,
        pathB: pB,
      });
      setWorkspace(result);
      setPathA(pA);
      setPathB(pB);
      setCurrentFrameA(0);
      setCurrentFrameB(0);
    } catch (err) {
      setError(err as string);
      console.error('Failed to create compare workspace:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const closeWorkspace = useCallback(() => {
    setWorkspace(null);
    setPathA(null);
    setPathB(null);
    setCurrentFrameA(0);
    setCurrentFrameB(0);
    setError(null);
  }, []);

  const setFrameA = useCallback((index: number) => {
    setCurrentFrameA(index);
  }, []);

  const setFrameB = useCallback((index: number) => {
    setCurrentFrameB(index);
  }, []);

  const setSyncMode = useCallback(async (mode: SyncMode) => {
    try {
      await invoke('set_sync_mode', { mode });
      if (workspace) {
        setWorkspace({ ...workspace, sync_mode: mode });
      }
    } catch (err) {
      console.error('Failed to set sync mode:', err);
    }
  }, [workspace]);

  const setManualOffset = useCallback(async (offset: number) => {
    try {
      await invoke('set_manual_offset', { offset });
      if (workspace) {
        setWorkspace({ ...workspace, manual_offset: offset });
      }
    } catch (err) {
      console.error('Failed to set manual offset:', err);
    }
  }, [workspace]);

  const resetOffset = useCallback(async () => {
    try {
      await invoke('reset_offset');
      if (workspace) {
        setWorkspace({ ...workspace, manual_offset: 0 });
      }
    } catch (err) {
      console.error('Failed to reset offset:', err);
    }
  }, [workspace]);

  const getAlignedFrame = useCallback(async (streamAIdx: number) => {
    try {
      const result = await invoke<[number, string]>('get_aligned_frame', {
        streamAIdx,
      });
      return {
        bIdx: result[0],
        quality: result[1] as AlignmentQuality,
      };
    } catch (err) {
      console.error('Failed to get aligned frame:', err);
      return { bIdx: null, quality: AlignmentQuality.Gap };
    }
  }, []);

  const value: CompareContextType = {
    workspace,
    isLoading,
    error,
    pathA,
    pathB,
    currentFrameA,
    currentFrameB,
    createWorkspace,
    closeWorkspace,
    setFrameA,
    setFrameB,
    setSyncMode,
    setManualOffset,
    resetOffset,
    getAlignedFrame,
  };

  return <CompareContext.Provider value={value}>{children}</CompareContext.Provider>;
}

export function useCompare(): CompareContextType {
  const context = useContext(CompareContext);
  if (!context) {
    throw new Error('useCompare must be used within CompareProvider');
  }
  return context;
}
