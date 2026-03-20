/**
 * RD Curves Panel
 *
 * Rate-Distortion curves visualization with BD-Rate calculation
 * Compare multiple encoders or encoding parameters
 *
 * Reference: RD curves & BD-Rate functionality
 */

import { useState, useCallback, useMemo } from "react";
import { memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LineChart, type Series, type DataPoint } from "../charts/LineChart";
import "./RDCurvesPanel.css";

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

  // Mock data for demonstration (DEV only)
  const mockCurves: RDCurveData[] = useMemo(
    () =>
      import.meta.env.DEV
        ? [
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
          ]
        : [],
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
      const errorMsg = err instanceof Error ? err.message : String(err);
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
    <div className="rd-curves-panel">
      <div className="rd-curves-header">
        <h3>Rate-Distortion Curves</h3>
        <div className="rd-curves-actions">
          {import.meta.env.DEV && (
            <button onClick={handleLoadMockData} className="rd-btn rd-btn-blue">
              Load Demo Data
            </button>
          )}
          <button
            onClick={handleCalculateBDRate}
            disabled={curves.length < 2 || isCalculating}
            className="rd-btn rd-btn-green"
          >
            {isCalculating ? "Calculating..." : "Calculate BD-Rate"}
          </button>
          <button onClick={handleClear} className="rd-btn rd-btn-gray">
            Clear
          </button>
        </div>
      </div>

      {/* Metric Selection */}
      <div className="rd-metric-row">
        <span className="rd-metric-label">Quality Metric:</span>
        <div className="rd-metric-buttons">
          {(["PSNR", "SSIM", "VMAF"] as const).map((metric) => (
            <button
              key={metric}
              onClick={() => setSelectedMetric(metric)}
              className={`rd-metric-btn${selectedMetric === metric ? " active" : ""}`}
            >
              {metric}
            </button>
          ))}
        </div>
      </div>

      {/* Error Message */}
      {error && <div className="rd-error">{error}</div>}

      {/* RD Curve Chart */}
      {curves.length > 0 ? (
        <div className="rd-chart-container">
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
        <div className="rd-empty">
          <p>No data loaded</p>
          <p>
            {import.meta.env.DEV
              ? 'Click "Load Demo Data" to see example RD curves, or load your own encoding data'
              : "Load your own encoding data to compare codec performance"}
          </p>
        </div>
      )}

      {/* BD-Rate Results */}
      {bdRateResult && (
        <div className="rd-bdrate-results">
          <h4>BD-Rate Calculation Results</h4>
          <div className="rd-bdrate-grid">
            <div className="rd-bdrate-item">
              <label>Anchor</label>
              <span className="value">{bdRateResult.anchor_name}</span>
            </div>
            <div className="rd-bdrate-item">
              <label>Test</label>
              <span className="value">{bdRateResult.test_name}</span>
            </div>
            <div className="rd-bdrate-item">
              <label>BD-Rate</label>
              <span
                className={`value ${bdRateResult.bd_rate < 0 ? "negative" : "positive"}`}
              >
                {bdRateResult.bd_rate > 0 ? "+" : ""}
                {bdRateResult.bd_rate.toFixed(2)}%
              </span>
            </div>
            <div className="rd-bdrate-item">
              <label>BD-PSNR</label>
              <span className="value blue">
                {bdRateResult.bd_psnr > 0 ? "+" : ""}
                {bdRateResult.bd_psnr.toFixed(2)} dB
              </span>
            </div>
            <div className="rd-bdrate-item span-2">
              <label>Interpretation</label>
              <div className="rd-bdrate-interpretation">
                {bdRateResult.interpretation}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Info */}
      <div className="rd-info">
        <p>
          <strong>BD-Rate</strong> (Bjøntegaard Delta Rate) measures the bitrate
          savings between two codecs for equivalent quality.
        </p>
        <p>
          Negative values indicate the test codec is more efficient (lower
          bitrate for same quality).
        </p>
        <p>
          Load multiple RD curves to compare encoder performance across
          different bitrates.
        </p>
      </div>
    </div>
  );
});
