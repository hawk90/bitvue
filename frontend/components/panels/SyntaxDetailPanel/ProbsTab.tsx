/**
 * VP9 Probabilities / Counts Tab
 *
 * Shows the VP9 context probability table for the current frame,
 * grouped by probability category.
 */

import { memo } from "react";

interface ProbEntry {
  group: string;
  label: string;
  probs: number[];
}

interface Vp9ProbsData {
  frame_index: number;
  is_key_frame: boolean;
  entries: ProbEntry[];
}

/**
 * Data source: none yet. `bitvue-sidecar`'s `get_codec_extended_info` is AV1-only and answers
 * `{ l0_refs, l1_refs, qp_histogram }` (see its module doc) -- it has no `vp9_probs` field and no
 * other sidecar command carries this data. Until one does, callers pass no `data` and the tab
 * shows its "not available" state.
 */
interface ProbsTabProps {
  filePath?: string;
  /** Probability table for the current frame; `null`/omitted when no source provides it. */
  data?: Vp9ProbsData | null;
}

function probColor(p: number): string {
  // 0 = certain false (blue), 128 = uncertain (yellow), 255 = certain true (red)
  const t = p / 255;
  const r = Math.round(t * 220);
  const g = Math.round(180 - Math.abs(t - 0.5) * 2 * 180);
  const b = Math.round((1 - t) * 220);
  return `rgb(${r},${g},${b})`;
}

const ProbBar = memo(function ProbBar({ value }: { value: number }) {
  return (
    <div className="prob-bar-wrap" title={`${value} / 255`}>
      <div
        className="prob-bar-fill"
        style={{
          width: `${(value / 255) * 100}%`,
          background: probColor(value),
        }}
      />
      <span className="prob-bar-label">{value}</span>
    </div>
  );
});

export const ProbsTab = memo(function ProbsTab({
  filePath,
  data = null,
}: ProbsTabProps) {
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
        <span>Probability data not available for this codec.</span>
      </div>
    );
  }

  // Group entries
  const grouped = data.entries.reduce<Record<string, ProbEntry[]>>((acc, e) => {
    (acc[e.group] ??= []).push(e);
    return acc;
  }, {});

  return (
    <div className="syntax-tab-content probs-tab">
      <div className="probs-header">
        <span>
          Frame #{data.frame_index} —{" "}
          <span style={{ color: data.is_key_frame ? "#4fc3f7" : "#66bb6a" }}>
            {data.is_key_frame ? "Key Frame" : "Inter Frame"}
          </span>
        </span>
        <span className="probs-hint">Bars: 0 = never, 255 = always</span>
      </div>

      {Object.entries(grouped).map(([group, entries]) => (
        <div key={group} className="probs-group">
          <div className="probs-group-header">{group}</div>
          {entries.map((e) => (
            <div key={e.label} className="probs-entry">
              <span className="probs-label">{e.label}</span>
              <div className="probs-bars">
                {e.probs.map((p, i) => (
                  <ProbBar key={i} value={p} />
                ))}
              </div>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
});

export default ProbsTab;
