/**
 * App File Operations Hook
 *
 * Manages file operations specific to App.tsx including frame loading
 * and dependent file opening for comparison mode.
 */

import { useState, useCallback } from "react";
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { createLogger } from "../utils/logger";
import type { FileInfo } from "../types/video";
import { useFileState, useFrameData } from "../contexts/StreamDataContext";
import { useCompare } from "../contexts/CompareContext";

const logger = createLogger('useAppFileOperations');

export interface AppFileOperationsCallbacks {
  onError: (title: string, message: string, details?: string) => void;
}

export interface AppFileOperationsReturn {
  fileInfo: FileInfo | null;
  setFileInfo: (info: FileInfo | null) => void;
  openError: string | null;
  setOpenError: (error: string | null) => void;
  handleOpenFile: () => Promise<void>;
  handleCloseFile: () => Promise<void>;
  handleOpenDependentFile: () => Promise<void>;
}

/**
 * Hook for managing App-specific file operations
 */
export function useAppFileOperations(
  callbacks: AppFileOperationsCallbacks
): AppFileOperationsReturn {
  const { onError } = callbacks;
  const { setFilePath, refreshFrames, clearData } = useFileState();
  const { setFrames } = useFrameData();
  const { createWorkspace } = useCompare();

  const [fileInfo, setFileInfo] = useState<FileInfo | null>(null);
  const [openError, setOpenError] = useState<string | null>(null);

  /**
   * Handle closing the current file
   */
  const handleCloseFile = useCallback(async () => {
    try {
      await invoke('close_file');
      setFileInfo(null);
      setFilePath(null);
      clearData();
    } catch (err) {
      logger.error('Failed to close file:', err);
      onError('Failed to Close File', err as string);
    }
  }, [setFilePath, clearData, onError]);

  /**
   * Handle opening a file
   */
  const handleOpenFile = useCallback(async () => {
    try {
      setOpenError(null);

      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Video Files',
            extensions: ['ivf', 'av1', 'hevc', 'h265', 'vvc', 'h266', 'mp4', 'mkv', 'webm', 'ts']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });

      if (selected && typeof selected === 'string') {
        logger.debug('Opening file:', selected);

        // Call the Tauri command to open the file
        const result = await invoke<FileInfo>('open_file', { path: selected });

        setFileInfo(result);
        setFilePath(result.success ? selected : null);

        if (result.success) {
          logger.info('File opened successfully');
          // Refresh frames after opening file
          try {
            const loadedFrames = await refreshFrames();
            setFrames(loadedFrames);
          } catch (refreshErr) {
            logger.error('Failed to refresh frames after opening file:', refreshErr);
            // Non-blocking: file opened successfully but frames failed to load
            onError(
              'Frame Load Warning',
              'File opened but failed to load frame data. Please try refreshing.',
              refreshErr as string
            );
          }
        } else {
          onError('Failed to Open File', result.error || 'Unknown error', selected);
        }
      }
    } catch (err) {
      logger.error('Failed to open file:', err);
      onError('Failed to Open File', err as string);
    }
  }, [refreshFrames, setFilePath, onError]);

  /**
   * Handle opening dependent bitstream for comparison
   */
  const handleOpenDependentFile = useCallback(async () => {
    try {
      setOpenError(null);

      if (!fileInfo?.success) {
        setOpenError('Please open a primary bitstream first before opening a dependent bitstream for comparison.');
        return;
      }

      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Video Files',
          extensions: ['ivf', 'av1', 'hevc', 'h265', 'vvc', 'h266', 'mp4', 'mkv', 'webm', 'ts']
        }]
      });

      if (selected === null) {
        return; // User cancelled
      }

      const pathB = typeof selected === 'string' ? selected : selected.path;

      logger.info(`Opening dependent bitstream: ${pathB}`);

      // Create compare workspace with current file as Stream A and selected file as Stream B
      await createWorkspace(fileInfo.path, pathB);

      logger.info(`Compare workspace created successfully: ${fileInfo.path} vs ${pathB}`);
    } catch (err) {
      logger.error('Failed to open dependent bitstream:', err);
      setOpenError(err as string);
    }
  }, [fileInfo, createWorkspace]);

  return {
    fileInfo,
    setFileInfo,
    openError,
    setOpenError,
    handleOpenFile,
    handleCloseFile,
    handleOpenDependentFile,
  };
}

export default useAppFileOperations;
