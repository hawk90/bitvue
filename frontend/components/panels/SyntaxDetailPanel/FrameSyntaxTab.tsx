/**
 * Frame Syntax Tab Component
 *
 * Displays syntax tree information for the current frame
 * Shows frame properties in expandable tree format
 */

import { memo, useCallback, useEffect, useRef, useState } from "react";
import { useSyntaxHexLink } from "../../../contexts/SyntaxHexLinkContext";
import {
  getFrameSyntax,
  type BridgeSyntaxNode,
} from "../../../services/electronBridgeService";

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
  byte_offset?: number;
}

/** `BridgeSyntaxNode` (sidecar's JSON shape, flat `value: string | null`) -> this tab's local
 *  `SyntaxNode` (discriminated `SyntaxValue` union). `byte_offset` is a real derived value
 *  (`bit_range.start_bit / 8`), not fabricated -- `bit_range` is itself a real absolute
 *  file-bit-offset (see bitvue-indexer's `get_frame_syntax` doc on why that's not just the raw
 *  byte offset passed in). `description` has no sidecar equivalent, left undefined. */
function bridgeNodeToLocal(node: BridgeSyntaxNode): SyntaxNode {
  return {
    name: node.name,
    value: node.value !== null ? { String: node.value } : undefined,
    children: node.children.map(bridgeNodeToLocal),
    byte_offset: Math.floor(node.bit_range.start_bit / 8),
  };
}

// Helper to get display value from SyntaxValue
function getDisplayValue(value?: SyntaxValue): string | number | undefined {
  if (!value) return undefined;
  if (value.String !== undefined) return value.String;
  if (value.Number !== undefined) return value.Number;
  if (value.Float !== undefined) return value.Float.toFixed(2);
  if (value.Boolean !== undefined) return String(value.Boolean);
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
  const { setHighlightedByteOffset } = useSyntaxHexLink();

  // Fetch real syntax tree from backend
  useEffect(() => {
    if (!frame || !filePath) {
      setSyntaxTree(null);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    getFrameSyntax("A", frame.frame_index)
      .then((node) => setSyntaxTree(bridgeNodeToLocal(node)))
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
        value: { Array: (frame.ref_frames ?? []).map(String) },
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
        {flattenVisible(frameSyntax, "", 0, expandedNodes).length >
        VIRTUALIZATION_THRESHOLD ? (
          <VirtualSyntaxTree
            rootNode={frameSyntax}
            expandedNodes={expandedNodes}
            onToggle={onToggleNode}
            onJumpToHex={setHighlightedByteOffset}
          />
        ) : (
          <SyntaxTreeNode
            node={frameSyntax}
            path=""
            depth={0}
            expandedNodes={expandedNodes}
            onToggle={onToggleNode}
            onJumpToHex={setHighlightedByteOffset}
          />
        )}
      </div>
    </div>
  );
});

// ─── Virtual list for large syntax trees ─────────────────────────────────────

const ITEM_HEIGHT = 24; // px per visible row
const OVERSCAN = 15; // extra rows rendered above and below the visible window
/** Below this threshold we use the plain recursive renderer (faster for small trees). */
const VIRTUALIZATION_THRESHOLD = 120;

interface FlatNode {
  node: SyntaxNode;
  path: string;
  depth: number;
}

/** Flatten the tree respecting the current expand/collapse state. */
function flattenVisible(
  node: SyntaxNode,
  path: string,
  depth: number,
  expandedNodes: Set<string>,
  out: FlatNode[] = [],
): FlatNode[] {
  const currentPath = path ? `${path}/${node.name}` : node.name;
  out.push({ node, path: currentPath, depth });
  if (
    node.children &&
    node.children.length > 0 &&
    expandedNodes.has(currentPath)
  ) {
    for (const child of node.children) {
      flattenVisible(child, currentPath, depth + 1, expandedNodes, out);
    }
  }
  return out;
}

interface VirtualSyntaxTreeProps {
  rootNode: SyntaxNode;
  expandedNodes: Set<string>;
  onToggle: (path: string) => void;
  onJumpToHex?: (offset: number) => void;
}

/** Virtual-scroll tree renderer — only mounts DOM nodes for the visible window. */
const VirtualSyntaxTree = memo(function VirtualSyntaxTree({
  rootNode,
  expandedNodes,
  onToggle,
  onJumpToHex,
}: VirtualSyntaxTreeProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [containerHeight, setContainerHeight] = useState(400);

  // Keep container height in sync with ResizeObserver
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      setContainerHeight(entry.contentRect.height);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
  }, []);

  const flatNodes = flattenVisible(rootNode, "", 0, expandedNodes);
  const totalHeight = flatNodes.length * ITEM_HEIGHT;
  const startIdx = Math.max(0, Math.floor(scrollTop / ITEM_HEIGHT) - OVERSCAN);
  const endIdx = Math.min(
    flatNodes.length,
    Math.ceil((scrollTop + containerHeight) / ITEM_HEIGHT) + OVERSCAN,
  );
  const paddingTop = startIdx * ITEM_HEIGHT;
  const visibleSlice = flatNodes.slice(startIdx, endIdx);

  return (
    <div
      ref={containerRef}
      className="syntax-virtual-scroll"
      onScroll={handleScroll}
    >
      <div style={{ height: totalHeight, position: "relative" }}>
        <div style={{ position: "absolute", top: paddingTop, width: "100%" }}>
          {visibleSlice.map(({ node, path, depth }) => {
            const hasChildren = !!node.children && node.children.length > 0;
            const isExpanded = expandedNodes.has(path);
            const displayValue = getDisplayValue(node.value);
            return (
              <div
                key={path}
                className="syntax-node"
                title={node.description}
                style={{ height: ITEM_HEIGHT }}
              >
                <div
                  className="syntax-node-item"
                  style={{ paddingLeft: `${depth * 12 + 8}px` }}
                >
                  {hasChildren ? (
                    <span
                      className={`codicon codicon-${isExpanded ? "chevron-down" : "chevron-right"} expand-toggle`}
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggle(path);
                      }}
                    />
                  ) : (
                    <span className="expand-placeholder">▪</span>
                  )}
                  <span className="syntax-label">{node.name}</span>
                  {displayValue !== undefined && (
                    <span className="syntax-value">
                      = {String(displayValue)}
                    </span>
                  )}
                  {node.byte_offset !== undefined && onJumpToHex && (
                    <span
                      className="syntax-hex-jump"
                      title={`Jump to byte 0x${node.byte_offset.toString(16).toUpperCase()}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        onJumpToHex(node.byte_offset!);
                      }}
                    >
                      ⇥
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
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
  onJumpToHex?: (offset: number) => void;
}

const SyntaxTreeNode = memo(function SyntaxTreeNode({
  node,
  path,
  depth,
  expandedNodes,
  onToggle,
  onJumpToHex,
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
        {node.byte_offset !== undefined && onJumpToHex && (
          <span
            className="syntax-hex-jump"
            title={`Jump to byte 0x${node.byte_offset.toString(16).toUpperCase()} in HEX view`}
            onClick={(e) => {
              e.stopPropagation();
              onJumpToHex(node.byte_offset!);
            }}
          >
            ⇥
          </span>
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
              onJumpToHex={onJumpToHex}
            />
          ))}
        </div>
      )}
    </div>
  );
});
