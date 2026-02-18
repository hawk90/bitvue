/**
 * Frame Syntax Tab Component
 *
 * Displays syntax tree information for the current frame
 * Shows frame properties in expandable tree format
 */

import { memo, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SyntaxValue {
  String?: string;
  Number?: number;
  Float?: number;
  Boolean?: boolean;
  Array?: string[];
}

export interface SyntaxNode {
  name: string;
  value?: SyntaxValue;
  children?: SyntaxNode[];
  description?: string;
}

// Helper to get display value from SyntaxValue
function getDisplayValue(value?: SyntaxValue): string | number | undefined {
  if (!value) return undefined;
  if (value.String !== undefined) return value.String;
  if (value.Number !== undefined) return value.Number;
  if (value.Float !== undefined) return value.Float.toFixed(2);
  if (value.Boolean !== undefined) return value.Boolean;
  if (value.Array !== undefined) return `[${value.Array.join(", ")}]`;
  return undefined;
}

interface FrameSyntaxTabProps {
  frame: {
    frame_index: number;
    frame_type: string;
    size: number;
    pts?: number;
    temporal_id?: number;
    display_order?: number;
    coding_order?: number;
    ref_frames?: number[];
  } | null;
  expandedNodes: Set<string>;
  onToggleNode: (path: string) => void;
  filePath?: string;
}

export const FrameSyntaxTab = memo(function FrameSyntaxTab({
  frame,
  expandedNodes,
  onToggleNode,
  filePath,
}: FrameSyntaxTabProps) {
  const [syntaxTree, setSyntaxTree] = useState<SyntaxNode | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch real syntax tree from backend
  useEffect(() => {
    if (!frame || !filePath) {
      setSyntaxTree(null);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    invoke<SyntaxNode>("get_frame_syntax", {
      path: filePath,
      frameIndex: frame.frame_index,
    })
      .then(setSyntaxTree)
      .catch((err) => {
        console.error("Failed to fetch frame syntax:", err);
        setError(err?.toString?.() || "Failed to load syntax data");
      })
      .finally(() => setLoading(false));
  }, [frame, filePath]);

  if (!frame) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-file-code"></span>
        <span>No frame selected</span>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-loading codicon-spin"></span>
        <span>Loading syntax data...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-error"></span>
        <span>{error}</span>
      </div>
    );
  }

  // Use fetched syntax tree or fall back to basic frame info
  const frameSyntax = syntaxTree || {
    name: `Frame ${frame.frame_index}`,
    children: [
      { name: "frame_type", value: { String: frame.frame_type } },
      { name: "frame_index", value: { Number: frame.frame_index } },
      { name: "pts", value: { Number: frame.pts ?? 0 } },
      { name: "size", value: { String: `${frame.size} bytes` } },
      {
        name: "temporal_id",
        value: { Number: frame.temporal_id ?? -1 },
        description: "Temporal layer identifier",
      },
      {
        name: "display_order",
        value: { Number: frame.display_order ?? -1 },
        description: "Display order in sequence",
      },
      {
        name: "coding_order",
        value: { Number: frame.coding_order ?? -1 },
        description: "Coding order in sequence",
      },
      {
        name: "ref_frames",
        value: { Array: frame.ref_frames ?? [] },
        description: "Reference frame indices",
        children: (frame.ref_frames ?? []).map((ref, idx) => ({
          name: `ref[${idx}]`,
          value: { Number: ref },
        })),
      },
    ],
  };

  return (
    <div className="syntax-tab-content">
      <div className="syntax-info">
        <span className="syntax-info-label">Frame:</span>
        <span className="syntax-info-value">{frame.frame_index}</span>
        <span className="syntax-info-label" style={{ marginLeft: 16 }}>
          Type:
        </span>
        <span
          className={`syntax-value frame-type-${frame.frame_type.toLowerCase()}`}
        >
          {frame.frame_type}
        </span>
      </div>
      <div className="panel-divider"></div>
      <div className="syntax-tree">
        <SyntaxTreeNode
          node={frameSyntax}
          path=""
          depth={0}
          expandedNodes={expandedNodes}
          onToggle={onToggleNode}
        />
      </div>
    </div>
  );
});

/**
 * Recursive syntax tree node component
 */
interface SyntaxTreeNodeProps {
  node: SyntaxNode;
  path: string;
  depth: number;
  expandedNodes: Set<string>;
  onToggle: (path: string) => void;
}

const SyntaxTreeNode = memo(function SyntaxTreeNode({
  node,
  path,
  depth,
  expandedNodes,
  onToggle,
}: SyntaxTreeNodeProps) {
  const currentPath = path ? `${path}/${node.name}` : node.name;
  const isExpanded = expandedNodes.has(currentPath);
  const hasChildren = node.children && node.children.length > 0;
  const displayValue = getDisplayValue(node.value);

  return (
    <div className="syntax-node" title={node.description}>
      <div
        className="syntax-node-item"
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        {hasChildren ? (
          <span
            className={`codicon codicon-${isExpanded ? "chevron-down" : "chevron-right"} expand-toggle`}
            onClick={(e) => {
              e.stopPropagation();
              onToggle(currentPath);
            }}
          />
        ) : (
          <span className="expand-placeholder">▪</span>
        )}
        <span className="syntax-label">{node.name}</span>
        {displayValue !== undefined && (
          <span className="syntax-value">= {String(displayValue)}</span>
        )}
      </div>
      {hasChildren && isExpanded && (
        <div className="syntax-children">
          {node.children?.map((child) => (
            <SyntaxTreeNode
              key={`${currentPath}/${child.name}`}
              node={child}
              path={currentPath}
              depth={depth + 1}
              expandedNodes={expandedNodes}
              onToggle={onToggle}
            />
          ))}
        </div>
      )}
    </div>
  );
});
