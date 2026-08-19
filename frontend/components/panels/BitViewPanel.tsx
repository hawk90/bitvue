/**
 * Bit View Panel
 *
 * Displays the bitstream syntax tree for the currently selected frame.
 * Shows codec-specific syntax elements (OBU, NAL, CTU, etc.) in a
 * collapsible tree view.
 */

import { useState, useEffect, useCallback, memo } from "react";
import { useFileState } from "../../contexts/StreamDataContext";
import { useCurrentFrame } from "../../contexts/StreamDataContext";
import {
  getFrameSyntax,
  type BridgeSyntaxNode,
} from "../../services/electronBridgeService";
import "./BitViewPanel.css";

interface SyntaxNode {
  name: string;
  value?: string | number | boolean | string[] | null;
  children: SyntaxNode[];
  description?: string | null;
}

/** `BridgeSyntaxNode` (sidecar's JSON shape) -> this panel's local `SyntaxNode`. `value` is
 *  already a plain string in the bridge shape, so this is mostly a field-name/nesting mapping,
 *  not a real transformation. `description` has no sidecar equivalent -- left undefined rather
 *  than fabricated. */
function bridgeNodeToLocal(node: BridgeSyntaxNode): SyntaxNode {
  return {
    name: node.name,
    value: node.value,
    children: node.children.map(bridgeNodeToLocal),
    description: undefined,
  };
}

function formatValue(value: SyntaxNode["value"]): string {
  if (value === null || value === undefined) return "";
  if (Array.isArray(value)) return `[${value.join(", ")}]`;
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}

function valueClass(value: SyntaxNode["value"]): string {
  if (typeof value === "boolean") return "sv-bool";
  if (typeof value === "number") return "sv-num";
  if (Array.isArray(value)) return "sv-arr";
  return "sv-str";
}

interface TreeNodeProps {
  node: SyntaxNode;
  depth: number;
  defaultOpen?: boolean;
}

const TreeNode = memo(function TreeNode({
  node,
  depth,
  defaultOpen = true,
}: TreeNodeProps) {
  const [open, setOpen] = useState(defaultOpen);
  const hasChildren = node.children.length > 0;
  const formatted = formatValue(node.value);

  return (
    <div className="sv-node" style={{ paddingLeft: depth * 14 }}>
      <div
        className={`sv-row${hasChildren ? " sv-expandable" : ""}`}
        onClick={hasChildren ? () => setOpen((o) => !o) : undefined}
        title={node.description ?? undefined}
      >
        {hasChildren ? (
          <span className="sv-toggle">{open ? "▾" : "▸"}</span>
        ) : (
          <span className="sv-toggle sv-leaf" />
        )}
        <span className="sv-name">{node.name}</span>
        {formatted !== "" && (
          <>
            <span className="sv-eq"> = </span>
            <span className={`sv-value ${valueClass(node.value)}`}>
              {formatted}
            </span>
          </>
        )}
      </div>
      {hasChildren && open && (
        <div className="sv-children">
          {node.children.map((child, i) => (
            <TreeNode
              key={i}
              node={child}
              depth={depth + 1}
              defaultOpen={depth < 1}
            />
          ))}
        </div>
      )}
    </div>
  );
});

export const BitViewPanel = memo(function BitViewPanel() {
  const { filePath } = useFileState();
  const { currentFrameIndex } = useCurrentFrame();

  const [syntax, setSyntax] = useState<SyntaxNode | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pinnedFrame, setPinnedFrame] = useState<number | null>(null);

  const frameIndex = pinnedFrame ?? currentFrameIndex;

  const load = useCallback(async (idx: number) => {
    setLoading(true);
    setError(null);
    try {
      const node = await getFrameSyntax("A", idx);
      setSyntax(bridgeNodeToLocal(node));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSyntax(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (filePath) {
      load(frameIndex);
    } else {
      setSyntax(null);
      setError(null);
    }
  }, [filePath, frameIndex, load]);

  if (!filePath) {
    return (
      <div className="bitview-panel">
        <div className="bitview-empty">No stream loaded</div>
      </div>
    );
  }

  return (
    <div className="bitview-panel">
      <div className="bitview-header">
        <h3>Bit View</h3>
        <div className="bitview-controls">
          <span className="bitview-frame-label">
            Frame {frameIndex}
            {pinnedFrame !== null && (
              <span className="bitview-pinned"> (pinned)</span>
            )}
          </span>
          {pinnedFrame !== null ? (
            <button
              className="bitview-btn"
              onClick={() => setPinnedFrame(null)}
            >
              Unpin
            </button>
          ) : (
            <button
              className="bitview-btn"
              onClick={() => setPinnedFrame(currentFrameIndex)}
            >
              Pin
            </button>
          )}
          <button
            className="bitview-btn"
            onClick={() => filePath && load(frameIndex)}
            disabled={loading}
          >
            {loading ? "..." : "Reload"}
          </button>
        </div>
      </div>

      {error && <div className="bitview-error">{error}</div>}

      <div className="bitview-tree">
        {loading && <div className="bitview-loading">Loading syntax...</div>}
        {!loading && syntax && (
          <TreeNode node={syntax} depth={0} defaultOpen={true} />
        )}
        {!loading && !syntax && !error && (
          <div className="bitview-empty">No syntax data</div>
        )}
      </div>
    </div>
  );
});
