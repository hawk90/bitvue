/**
 * VVC APS Tab
 *
 * Displays VVC (H.266) Adaptation Parameter Set entries:
 * ALF (Adaptive Loop Filter), LMCS (Luma Mapping with Chroma Scaling),
 * and Scaling List APS.
 */

import { memo } from "react";

interface ApsEntry {
  aps_id: number;
  aps_type: string;
  enabled: boolean;
  summary: string;
}

interface VvcApsData {
  aps_list: ApsEntry[];
}

/**
 * Data source: none yet. `bitvue-sidecar`'s `get_codec_extended_info` is AV1-only and answers
 * `{ l0_refs, l1_refs, qp_histogram }` (see its module doc) -- it has no `vvc_aps` field and no
 * other sidecar command carries this data. Until one does, callers pass no `data` and the tab
 * shows its "not available" state.
 */
interface ApsTabProps {
  filePath?: string;
  /** APS entries for the current frame; `null`/omitted when no source provides them. */
  data?: VvcApsData | null;
}

const APS_TYPE_COLORS: Record<string, string> = {
  ALF: "#4fc3f7",
  LMCS: "#66bb6a",
  SCALING_LIST: "#ffa726",
};

export const ApsTab = memo(function ApsTab({
  filePath,
  data = null,
}: ApsTabProps) {
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
        <span>APS data not available for this codec.</span>
      </div>
    );
  }

  const byType = data.aps_list.reduce<Record<string, ApsEntry[]>>((acc, e) => {
    (acc[e.aps_type] ??= []).push(e);
    return acc;
  }, {});

  return (
    <div className="syntax-tab-content aps-tab">
      {Object.entries(byType).map(([type, entries]) => (
        <div key={type} className="aps-section">
          <div
            className="aps-section-header"
            style={{ borderColor: APS_TYPE_COLORS[type] ?? "#888" }}
          >
            <span
              className="aps-type-badge"
              style={{ background: APS_TYPE_COLORS[type] ?? "#888" }}
            >
              {type}
            </span>
            <span className="aps-section-count">
              {entries.length} APS entr{entries.length === 1 ? "y" : "ies"}
            </span>
          </div>
          {entries.map((e, i) => (
            <div
              key={i}
              className={`aps-entry${e.enabled ? " aps-entry--enabled" : " aps-entry--disabled"}`}
            >
              <div className="aps-entry-header">
                <span className="aps-id">APS ID {e.aps_id}</span>
                <span
                  className={`aps-status${e.enabled ? " aps-status--on" : " aps-status--off"}`}
                >
                  {e.enabled ? "Active" : "Inactive"}
                </span>
              </div>
              <div className="aps-summary">{e.summary}</div>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
});

export default ApsTab;
