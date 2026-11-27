/**
 * Quantization Matrix Tab
 *
 * Displays HEVC/H.265 quantization scaling matrices with color-coded heat-map.
 * Supported sizes: 4×4, 8×8 (full values), 16×16 and 32×32 (heat-map only).
 */

import { memo, useState } from "react";

interface QmMatrix {
  name: string;
  size: number;
  pred_type: string;
  plane: string;
  values: number[];
}

interface HevcQmData {
  scaling_list_enabled: boolean;
  matrices: QmMatrix[];
}

/**
 * Data source: none yet. `bitvue-sidecar`'s `get_codec_extended_info` is AV1-only and answers
 * `{ l0_refs, l1_refs, qp_histogram }` (see its module doc) -- it has no `hevc_qm` field and no
 * other sidecar command carries this data. Until one does, callers pass no `data` and the tab
 * shows its "not available" state.
 */
interface QmTabProps {
  filePath?: string;
  /** Scaling matrices for the current frame; `null`/omitted when no source provides them. */
  data?: HevcQmData | null;
}

/** Map a scaling-list value to a heat-map colour (16 = neutral/flat). */
function heatColor(v: number): string {
  // 16 is the flat default. Range roughly 16–115 for default 8×8.
  const lo = 16;
  const hi = 115;
  const t = Math.max(0, Math.min(1, (v - lo) / (hi - lo)));
  // Blue (cold/flat) → Yellow → Red (high quantisation)
  const r = Math.round(t * 255);
  const g = Math.round((1 - Math.abs(t - 0.5) * 2) * 180);
  const b = Math.round((1 - t) * 200);
  return `rgb(${r},${g},${b})`;
}

const MatrixCell = memo(function MatrixCell({
  value,
  size,
}: {
  value: number;
  size: number;
}) {
  const cellPx = size <= 8 ? 22 : size <= 16 ? 12 : 6;
  const showText = size <= 8;
  return (
    <div
      style={{
        width: cellPx,
        height: cellPx,
        background: heatColor(value),
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: 7,
        color: "rgba(255,255,255,0.9)",
        borderRadius: 1,
        flexShrink: 0,
      }}
      title={`${value}`}
    >
      {showText ? value : ""}
    </div>
  );
});

const MatrixGrid = memo(function MatrixGrid({ matrix }: { matrix: QmMatrix }) {
  const isFlat = matrix.values.every((v) => v === 16);
  return (
    <div className="qm-matrix-block">
      <div className="qm-matrix-name">{matrix.name}</div>
      {isFlat ? (
        <div className="qm-matrix-flat">Flat (16)</div>
      ) : (
        <div
          className="qm-matrix-grid"
          style={{ gridTemplateColumns: `repeat(${matrix.size}, auto)` }}
        >
          {matrix.values.map((v, i) => (
            <MatrixCell key={i} value={v} size={matrix.size} />
          ))}
        </div>
      )}
    </div>
  );
});

export const QmTab = memo(function QmTab({
  filePath,
  data = null,
}: QmTabProps) {
  const [filter, setFilter] = useState<"all" | "intra" | "inter">("all");

  if (!filePath) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-file-code" />
        <span>No file loaded</span>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-info" />
        <span>QM data not available for this codec/frame.</span>
      </div>
    );
  }

  const visibleMatrices = data.matrices.filter((m) => {
    if (filter === "intra") return m.pred_type === "Intra";
    if (filter === "inter") return m.pred_type === "Inter";
    return true;
  });

  return (
    <div className="syntax-tab-content qm-tab">
      <div className="qm-header">
        <span className="qm-status">
          Scaling List:{" "}
          <strong>
            {data.scaling_list_enabled ? "Enabled" : "Disabled (flat)"}
          </strong>
        </span>
        <div className="qm-filter-row">
          {(["all", "intra", "inter"] as const).map((f) => (
            <button
              key={f}
              className={`qm-filter-btn${filter === f ? " active" : ""}`}
              onClick={() => setFilter(f)}
            >
              {f === "all" ? "All" : f === "intra" ? "Intra" : "Inter"}
            </button>
          ))}
        </div>
      </div>

      {/* Colour scale legend */}
      <div className="qm-legend">
        <span className="qm-legend-label">Low (16)</span>
        <div className="qm-legend-bar">
          {Array.from({ length: 20 }, (_, i) => (
            <div
              key={i}
              style={{
                flex: 1,
                background: heatColor(16 + Math.round((i / 19) * 99)),
              }}
            />
          ))}
        </div>
        <span className="qm-legend-label">High (115)</span>
      </div>

      <div className="qm-matrices">
        {visibleMatrices.map((m) => (
          <MatrixGrid key={m.name} matrix={m} />
        ))}
      </div>
    </div>
  );
});

export default QmTab;
