/**
 * Quality Comparison Panel
 *
 * PSNR/SSIM/VMAF visualization for comparing two video files
 * Uses Tauri backend command for actual quality calculation
 *
 * Reference: src-tauri/src/commands/quality.rs
 */

import React, { useState, useCallback, memo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

interface QualityMetrics {
  frame_index: number;
  psnr_y?: number;
  psnr_u?: number;
  psnr_v?: number;
  psnr_avg?: number;
  ssim_y?: number;
  ssim_u?: number;
  ssim_v?: number;
  ssim_avg?: number;
  vmaf?: number;
}

interface BatchQualityMetrics {
  frames: QualityMetrics[];
  average_psnr?: number;
  average_ssim?: number;
  average_vmaf?: number;
}

export const QualityComparisonPanel = memo(function QualityComparisonPanel() {
  const [referencePath, setReferencePath] = useState<string | null>(null);
  const [distortedPath, setDistortedPath] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<BatchQualityMetrics | null>(null);
  const [isCalculating, setIsCalculating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSelectReference = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Video Files',
            extensions: ['ivf', 'mp4', 'mkv', 'webm', 'yuv', 'y4m']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });

      if (selected && typeof selected === 'string') {
        setReferencePath(selected);
        setError(null);
      }
    } catch (err) {
      console.error('Failed to open file dialog:', err);
      setError('Failed to open file dialog');
    }
  }, []);

  const handleSelectDistorted = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Video Files',
            extensions: ['ivf', 'mp4', 'mkv', 'webm', 'yuv', 'y4m']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });

      if (selected && typeof selected === 'string') {
        setDistortedPath(selected);
        setError(null);
      }
    } catch (err) {
      console.error('Failed to open file dialog:', err);
      setError('Failed to open file dialog');
    }
  }, []);

  const handleCalculate = useCallback(async () => {
    if (!referencePath || !distortedPath) {
      return;
    }

    setIsCalculating(true);
    setError(null);
    try {
      // Call Tauri backend command for actual quality calculation
      const result = await invoke<BatchQualityMetrics>('calculate_quality_metrics', {
        referencePath,
        distortedPath,
        frameIndices: null,  // Calculate for all frames
        calculatePsnr: true,
        calculateSsim: true,
        calculateVmaf: false,  // VMAF requires libvmaf feature (not yet enabled)
      });

      setMetrics(result);
    } catch (err) {
      const errorMsg = err as string;
      console.error('Failed to calculate quality metrics:', err);
      setError(errorMsg);
    } finally {
      setIsCalculating(false);
    }
  }, [referencePath, distortedPath]);

  // Get display values from metrics
  const displayPsnr = metrics?.average_psnr ?? metrics?.frames[0]?.psnr_avg;
  const displaySsim = metrics?.average_ssim ?? metrics?.frames[0]?.ssim_avg;

  return (
    <div className="quality-comparison-panel p-4">
      <h3 className="text-lg font-semibold mb-4">Quality Comparison</h3>

      {/* File Selection */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="border rounded p-4">
          <label className="block text-sm font-medium mb-2">Reference File (Original)</label>
          <button
            onClick={handleSelectReference}
            className="w-full px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded"
          >
            Select File...
          </button>
          {referencePath && (
            <p className="text-sm text-gray-600 mt-2 truncate" title={referencePath}>
              {referencePath.split('/').pop() || referencePath.split('\\').pop()}
            </p>
          )}
        </div>

        <div className="border rounded p-4">
          <label className="block text-sm font-medium mb-2">Distorted File (Encoded)</label>
          <button
            onClick={handleSelectDistorted}
            className="w-full px-3 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded"
          >
            Select File...
          </button>
          {distortedPath && (
            <p className="text-sm text-gray-600 mt-2 truncate" title={distortedPath}>
              {distortedPath.split('/').pop() || distortedPath.split('\\').pop()}
            </p>
          )}
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-700 text-sm">
          {error}
        </div>
      )}

      {/* Calculate Button */}
      <button
        onClick={handleCalculate}
        disabled={!referencePath || !distortedPath || isCalculating}
        className="w-full bg-blue-500 text-white py-2 px-4 rounded hover:bg-blue-600 disabled:bg-gray-300 disabled:cursor-not-allowed mb-4"
      >
        {isCalculating ? 'Calculating...' : 'Calculate Quality Metrics'}
      </button>

      {/* Results */}
      {metrics && metrics.frames.length > 0 && (
        <div className="border rounded p-4 bg-gray-50">
          <h4 className="font-semibold mb-3">
            Quality Metrics
            {metrics.frames.length > 1 && ` (${metrics.frames.length} frames)`}
          </h4>
          <div className="grid grid-cols-3 gap-4">
            <div className="text-center">
              <div className="text-2xl font-bold text-green-600">
                {displayPsnr !== undefined ? displayPsnr.toFixed(2) : 'N/A'}
              </div>
              <div className="text-sm text-gray-600">PSNR (dB)</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-blue-600">
                {displaySsim !== undefined ? displaySsim.toFixed(4) : 'N/A'}
              </div>
              <div className="text-sm text-gray-600">SSIM</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-gray-400">
                N/A
              </div>
              <div className="text-sm text-gray-600">VMAF</div>
            </div>
          </div>
        </div>
      )}

      {/* Info */}
      <div className="mt-4 text-sm text-gray-500">
        <p>Select two video files to compare their quality using PSNR and SSIM metrics.</p>
        <p className="mt-1">Supported formats: IVF, MP4, MKV, WebM, YUV, Y4M</p>
      </div>
    </div>
  );
});

export default QualityComparisonPanel;
