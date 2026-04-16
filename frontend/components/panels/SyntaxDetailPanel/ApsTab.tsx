/**
 * VVC APS Tab
 *
 * Displays VVC (H.266) Adaptation Parameter Set entries:
 * ALF (Adaptive Loop Filter), LMCS (Luma Mapping with Chroma Scaling),
 * and Scaling List APS.
 */

import { memo, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ApsEntry {
  aps_id: number;
  aps_type: string;
  enabled: boolean;
  summary: string;
}

interface VvcApsData {
  aps_list: ApsEntry[];
}

interface CodecExtendedInfo {
  vvc_aps: VvcApsData | null;
}

interface ApsTabProps {
  filePath?: string;
  frameIndex: number;
}

const APS_TYPE_COLORS: Record<string, string> = {
  ALF: "#4fc3f7",
  LMCS: "#66bb6a",
  SCALING_LIST: "#ffa726",
};

export const ApsTab = memo(function ApsTab({
  filePath,
  frameIndex,
}: ApsTabProps) {
  const [data, setData] = useState<VvcApsData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!filePath) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    invoke<CodecExtendedInfo>("get_codec_extended_info", {
      path: filePath,
      frameIndex,
    })
      .then((info) => {
        if (!cancelled) setData(info.vvc_aps);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [filePath, frameIndex]);

  if (!filePath) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-file-code" />
        <span>No file loaded</span>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-loading codicon-spin" />
        <span>Loading APS data…</span>
      </div>
    );
  }

  if (error || !data) {
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
