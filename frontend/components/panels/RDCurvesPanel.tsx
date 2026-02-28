/**
 * RD Curves Panel
 *
 * Rate-Distortion curves visualization with BD-Rate calculation
 * Compare multiple encoders or encoding parameters
 *
 * Reference: RD curves & BD-Rate functionality
 */

import React, { useState, useCallback, useMemo } from "react";
import { memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LineChart, type Series, type DataPoint } from "../charts/LineChart";

interface RDCurveData {
  name: string;
  points: DataPoint[];
  color?: string;
}

interface BDRateResult {
  anchor_name: string;
  test_name: string;
  bd_rate: number;
  bd_psnr: number;
  interpretation: string;
}

export const RDCurvesPanel = memo(function RDCurvesPanel() {
  const [curves, setCurves] = useState<RDCurveData[]>([]);
  const [selectedMetric, setSelectedMetric] = useState<
    "PSNR" | "SSIM" | "VMAF"
  >("PSNR");
  const [bdRateResult, setBdRateResult] = useState<BDRateResult | null>(null);
  const [isCalculating, setIsCalculating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Mock data for demonstration
  const mockCurves: RDCurveData[] = useMemo(
    () => [
      {
        name: "H.264/AVC",
        points: [
          { x: 500, y: 36.5 },
          { x: 1000, y: 38.2 },
          { x: 2000, y: 40.1 },
          { x: 4000, y: 41.8 },
          { x: 8000, y: 43.2 },
        ],
        color: "#ef4444",
      },
      {
        name: "H.265/HEVC",
        points: [
          { x: 500, y: 38.1 },
          { x: 1000, y: 40.5 },
          { x: 2000, y: 42.8 },
          { x: 4000, y: 44.5 },
          { x: 8000, y: 45.8 },
        ],
        color: "#3b82f6",
      },
      {
        name: "H.266/VVC",
        points: [
          { x: 500, y: 39.2 },
          { x: 1000, y: 42.0 },
          { x: 2000, y: 44.5 },
          { x: 4000, y: 46.5 },
          { x: 8000, y: 47.9 },
        ],
        color: "#10b981",
      },
    ],
    [],
  );

  const handleLoadMockData = useCallback(() => {
    setCurves(mockCurves);
    setBdRateResult(null);
    setError(null);
  }, [mockCurves]);

  const handleCalculateBDRate = useCallback(async () => {
    if (curves.length < 2) {
      setError("Need at least 2 RD curves to calculate BD-Rate");
      return;
    }

    setIsCalculating(true);
    setError(null);
    try {
      // Convert RDCurveData to the format expected by the Tauri command
      const anchorCurve = {
        name: curves[0].name,
        points: curves[0].points.map((p) => ({ bitrate: p.x, quality: p.y })),
      };

      const testCurve = {
        name: curves[1].name,
        points: curves[1].points.map((p) => ({ bitrate: p.x, quality: p.y })),
      };

      const result = await invoke<BDRateResult>("calculate_bd_rate", {
        anchorCurve,
        testCurve,
      });

      setBdRateResult(result);
    } catch (err) {
      const errorMsg = err as string;
      console.error("Failed to calculate BD-Rate:", err);
      setError(errorMsg);
    } finally {
      setIsCalculating(false);
    }
  }, [curves]);

  const handleClear = useCallback(() => {
    setCurves([]);
    setBdRateResult(null);
    setError(null);
  }, []);

  // Convert curves to chart series
  const chartSeries: Series[] = useMemo(() => {
    return curves.map((curve) => ({
      name: curve.name,
      data: curve.points,
      color: curve.color,
      lineWidth: 2,
      showPoints: true,
    }));
  }, [curves]);

  // Get axis label based on selected metric
  const yAxisLabel = useMemo(() => {
    switch (selectedMetric) {
      case "PSNR":
        return "PSNR (dB)";
      case "SSIM":
        return "SSIM (index)";
      case "VMAF":
        return "VMAF (score)";
      default:
        return "Quality";
    }
  }, [selectedMetric]);

  return (
    <div className="rd-curves-panel p-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">Rate-Distortion Curves</h3>
        <div className="flex gap-2">
          <button
            onClick={handleLoadMockData}
            className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            Load Demo Data
          </button>
          <button
            onClick={handleCalculateBDRate}
            disabled={curves.length < 2 || isCalculating}
            className="px-3 py-1 text-sm bg-green-500 text-white rounded hover:bg-green-600 disabled:bg-gray-300 disabled:cursor-not-allowed"
          >
            {isCalculating ? "Calculating..." : "Calculate BD-Rate"}
          </button>
          <button
            onClick={handleClear}
            className="px-3 py-1 text-sm bg-gray-500 text-white rounded hover:bg-gray-600"
          >
            Clear
          </button>
        </div>
      </div>

      {/* Metric Selection */}
      <div className="mb-4 flex items-center gap-4">
        <label className="text-sm font-medium">Quality Metric:</label>
        <div className="flex gap-2">
          {(["PSNR", "SSIM", "VMAF"] as const).map((metric) => (
            <button
              key={metric}
              onClick={() => setSelectedMetric(metric)}
              className={`px-3 py-1 text-sm rounded ${
                selectedMetric === metric
                  ? "bg-blue-500 text-white"
                  : "bg-gray-200 text-gray-700 hover:bg-gray-300"
              }`}
            >
              {metric}
            </button>
          ))}
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-700 text-sm">
          {error}
        </div>
      )}

      {/* RD Curve Chart */}
      {curves.length > 0 ? (
        <div className="mb-4 p-4 border rounded bg-gray-50">
          <LineChart
            series={chartSeries}
            xAxisLabel="Bitrate (kbps)"
            yAxisLabel={yAxisLabel}
            title="Rate-Distortion Curves"
            showGrid={true}
            showLegend={true}
            height={350}
          />
        </div>
      ) : (
        <div className="mb-4 p-8 border rounded bg-gray-50 text-center text-text-secondary">
          <p className="mb-2">No data loaded</p>
          <p className="text-sm">
            Click "Load Demo Data" to see example RD curves, or load your own
            encoding data
          </p>
        </div>
      )}

      {/* BD-Rate Results */}
      {bdRateResult && (
        <div className="p-4 border rounded bg-green-50">
          <h4 className="font-semibold mb-3 text-green-800">
            BD-Rate Calculation Results
          </h4>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <div className="text-sm text-gray-600">Anchor</div>
              <div className="font-medium">{bdRateResult.anchor_name}</div>
            </div>
            <div>
              <div className="text-sm text-gray-600">Test</div>
              <div className="font-medium">{bdRateResult.test_name}</div>
            </div>
            <div>
              <div className="text-sm text-gray-600">BD-Rate</div>
              <div
                className={`font-bold text-lg ${bdRateResult.bd_rate < 0 ? "text-green-600" : "text-red-600"}`}
              >
                {bdRateResult.bd_rate > 0 ? "+" : ""}
                {bdRateResult.bd_rate.toFixed(2)}%
              </div>
            </div>
            <div>
              <div className="text-sm text-gray-600">BD-PSNR</div>
              <div className="font-bold text-lg text-blue-600">
                {bdRateResult.bd_psnr > 0 ? "+" : ""}
                {bdRateResult.bd_psnr.toFixed(2)} dB
              </div>
            </div>
            <div className="col-span-2">
              <div className="text-sm text-gray-600">Interpretation</div>
              <div className="text-sm mt-1 p-2 bg-white rounded border">
                {bdRateResult.interpretation}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Info */}
      <div className="mt-4 text-sm text-gray-500">
        <p>
          <strong>BD-Rate</strong> (Bjøntegaard Delta Rate) measures the bitrate
          savings between two codecs for equivalent quality.
        </p>
        <p className="mt-1">
          Negative values indicate the test codec is more efficient (lower
          bitrate for same quality).
        </p>
        <p className="mt-1">
          Load multiple RD curves to compare encoder performance across
          different bitrates.
        </p>
      </div>
    </div>
  );
});
