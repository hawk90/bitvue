/**
 * Reference List Tab
 *
 * Displays L0 and L1 reference frame lists for the current frame.
 * Shows frame index, POC delta, frame type, long-term flag,
 * and optional weighted prediction parameters.
 */

import { memo, useState, useEffect } from "react";
import { getCodecExtendedInfo } from "../../../services/electronBridgeService";

interface RefEntry {
  list_idx: number;
  slot: number;
  poc: number;
  frame_index: number;
  frame_type: string;
  long_term: boolean;
  weight: number | null;
  offset: number | null;
}

interface RefListTabProps {
  filePath?: string;
  frameIndex: number;
  /** Current frame type for display context */
  frameType?: string;
}

const FRAME_TYPE_COLORS: Record<string, string> = {
  I: "#4fc3f7",
  KEY: "#4fc3f7",
  IDR: "#4fc3f7",
  P: "#66bb6a",
  B: "#ffa726",
  BI: "#ff7043",
};

function frameColor(type: string): string {
  return FRAME_TYPE_COLORS[type.toUpperCase()] ?? "#78909c";
}

const RefRow = memo(function RefRow({ entry }: { entry: RefEntry }) {
  const pocStr = entry.poc > 0 ? `+${entry.poc}` : String(entry.poc);
  return (
    <div className="reflist-row">
      <span className="reflist-slot">[{entry.slot}]</span>
      <span className="reflist-frame">#{entry.frame_index}</span>
      <span
        className="reflist-type"
        style={{ color: frameColor(entry.frame_type) }}
      >
        {entry.frame_type}
      </span>
      <span className="reflist-poc" title="POC delta from current frame">
        POC {pocStr}
      </span>
      {entry.long_term && (
        <span className="reflist-lt" title="Long-term reference">
          LT
        </span>
      )}
      {entry.weight !== null && (
        <span className="reflist-weight" title="Weighted pred">
          w={entry.weight}
        </span>
      )}
      {entry.offset !== null && (
        <span className="reflist-offset" title="Weighted offset">
          o={entry.offset}
        </span>
      )}
    </div>
  );
});

export const RefListTab = memo(function RefListTab({
  filePath,
  frameIndex,
  frameType,
}: RefListTabProps) {
  const [l0, setL0] = useState<RefEntry[]>([]);
  const [l1, setL1] = useState<RefEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!filePath) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    getCodecExtendedInfo(frameIndex)
      .then((info) => {
        if (!cancelled) {
          setL0(info.l0_refs);
          setL1(info.l1_refs);
        }
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
        <span>Loading reference lists…</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-error" />
        <span>{error}</span>
      </div>
    );
  }

  const ft = (frameType ?? "").toUpperCase();
  const isIntra = ft === "I" || ft === "KEY" || ft === "IDR" || ft === "CRA";

  if (isIntra || (l0.length === 0 && l1.length === 0)) {
    return (
      <div className="syntax-tab-content">
        <div className="syntax-empty" style={{ paddingTop: 24 }}>
          <span className="codicon codicon-circle-slash" />
          <span>
            {isIntra
              ? `${frameType ?? "Frame"} — no reference frames (intra-coded)`
              : "No reference frames for this frame"}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="syntax-tab-content reflist-tab">
      {l0.length > 0 && (
        <div className="reflist-section">
          <div className="reflist-header">
            <span className="reflist-list-badge reflist-list-l0">L0</span>
            <span className="reflist-count">
              {l0.length} reference{l0.length !== 1 ? "s" : ""}
            </span>
          </div>
          {l0.map((e) => (
            <RefRow key={`l0-${e.slot}`} entry={e} />
          ))}
        </div>
      )}

      {l1.length > 0 && (
        <div className="reflist-section">
          <div className="reflist-header">
            <span className="reflist-list-badge reflist-list-l1">L1</span>
            <span className="reflist-count">
              {l1.length} reference{l1.length !== 1 ? "s" : ""}
            </span>
          </div>
          {l1.map((e) => (
            <RefRow key={`l1-${e.slot}`} entry={e} />
          ))}
        </div>
      )}
    </div>
  );
});

export default RefListTab;
