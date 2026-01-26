import React, { useState, useCallback } from 'react';

interface QualityMetrics {
  psnr: number;
  ssim: number;
  vmaf?: number;
}

interface FrameData {
  index: number;
  width: number;
  height: number;
  size: number;
}

export const QualityComparisonPanel: React.FC = () => {
  const [referenceFile, setReferenceFile] = useState<File | null>(null);
  const [distortedFile, setDistortedFile] = useState<File | null>(null);
  const [metrics, setMetrics] = useState<QualityMetrics | null>(null);
  const [isCalculating, setIsCalculating] = useState(false);

  const handleCalculate = useCallback(async () => {
    if (!referenceFile || !distortedFile) {
      return;
    }

    setIsCalculating(true);
    try {
      // TODO: Implement actual quality calculation
      // For now, show mock data
      await new Promise(resolve => setTimeout(resolve, 1000));

      setMetrics({
        psnr: 35.2,
        ssim: 0.92,
        vmaf: 78.5,
      });
    } catch (error) {
      console.error('Failed to calculate metrics:', error);
    } finally {
      setIsCalculating(false);
    }
  }, [referenceFile, distortedFile]);

  return (
    <div className="quality-comparison-panel p-4">
      <h3 className="text-lg font-semibold mb-4">Quality Comparison</h3>

      {/* File Selection */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="border rounded p-4">
          <label className="block text-sm font-medium mb-2">Reference File (Original)</label>
          <input
            type="file"
            accept="video/*,.yuv,.y4m"
            onChange={(e) => setReferenceFile(e.target.files?.[0] || null)}
            className="block w-full text-sm file:mr-4 file:mt-2 rounded"
          />
          {referenceFile && (
            <p className="text-sm text-gray-600 mt-2">{referenceFile.name}</p>
          )}
        </div>

        <div className="border rounded p-4">
          <label className="block text-sm font-medium mb-2">Distorted File (Encoded)</label>
          <input
            type="file"
            accept="video/*,.yuv,.y4m"
            onChange={(e) => setDistortedFile(e.target.files?.[0] || null)}
            className="block w-full text-sm file:mr-4 file:mt-2 rounded"
          />
          {distortedFile && (
            <p className="text-sm text-gray-600 mt-2">{distortedFile.name}</p>
          )}
        </div>
      </div>

      {/* Calculate Button */}
      <button
        onClick={handleCalculate}
        disabled={!referenceFile || !distortedFile || isCalculating}
        className="w-full bg-blue-500 text-white py-2 px-4 rounded hover:bg-blue-600 disabled:bg-gray-300 disabled:cursor-not-allowed mb-4"
      >
        {isCalculating ? 'Calculating...' : 'Calculate Quality Metrics'}
      </button>

      {/* Results */}
      {metrics && (
        <div className="border rounded p-4 bg-gray-50">
          <h4 className="font-semibold mb-3">Quality Metrics</h4>
          <div className="grid grid-cols-3 gap-4">
            <div className="text-center">
              <div className="text-2xl font-bold text-green-600">{metrics.psnr.toFixed(2)}</div>
              <div className="text-sm text-gray-600">PSNR (dB)</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-blue-600">{metrics.ssim.toFixed(4)}</div>
              <div className="text-sm text-gray-600">SSIM</div>
            </div>
            <div className="text-center">
              {metrics.vmaf !== undefined && (
                <>
                  <div className="text-2xl font-bold text-purple-600">{metrics.vmaf.toFixed(2)}</div>
                  <div className="text-sm text-gray-600">VMAF</div>
                </>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Info */}
      <div className="mt-4 text-sm text-gray-500">
        <p>Upload two video files to compare their quality using PSNR, SSIM, and VMAF metrics.</p>
        <p className="mt-1">Supported formats: MP4, YUV, Y4M</p>
      </div>
    </div>
  );
};

export default QualityComparisonPanel;
