/**
 * Stream Tree Panel
 *
 * NAL/OBV tree structure view - codec-specific syntax hierarchy
 * Features:
 * - Hierarchical display of NAL/OBU units
 * - Frame type filtering (I/P/B/All, Headers, Frames)
 * - Search by type or offset
 *
 * Reference: crates/ui/src/panels/stream_tree.rs
 */

import { useState, useCallback, useMemo, memo, Fragment } from 'react';
import { useFrameData } from '../../contexts/FrameDataContext';
import './StreamTreePanel.css';

export interface UnitNode {
  key: string;
  unit_type: string;
  offset: number;
  size: number;
  frame_index?: number;
  pts?: number;
  children: UnitNode[];
  qp_avg?: number;
}

interface StreamTreePanelProps {
  units?: UnitNode[];
  selectedUnitKey?: string;
  onUnitSelect?: (unit: UnitNode) => void;
}

type FrameFilter = 'All' | 'KeyOnly' | 'InterOnly' | 'FramesOnly' | 'HeadersOnly';

const FRAME_FILTERS: { value: FrameFilter; label: string }[] = [
  { value: 'All', label: 'All' },
  { value: 'KeyOnly', label: 'Key (I)' },
  { value: 'InterOnly', label: 'Inter (P/B)' },
  { value: 'FramesOnly', label: 'Frames' },
  { value: 'HeadersOnly', label: 'Headers' },
];

export const StreamTreePanel = memo(function StreamTreePanel({
  units = [],
  selectedUnitKey,
  onUnitSelect,
}: StreamTreePanelProps) {
  const { frames } = useFrameData();
  const [filterEnabled, setFilterEnabled] = useState(false);
  const [frameFilter, setFrameFilter] = useState<FrameFilter>('All');
  const [search, setSearch] = useState('');
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set());

  // Toggle node expansion
  const toggleNode = useCallback((key: string) => {
    setExpandedNodes(prev => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }, []);

  const handleToggleNode = useCallback((key: string, e: React.MouseEvent) => {
    e.stopPropagation();
    toggleNode(key);
  }, [toggleNode]);

  const handleUnitSelect = useCallback((unit: UnitNode) => {
    onUnitSelect?.(unit);
  }, [onUnitSelect]);

  const handleFilterEnabledChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setFilterEnabled(e.target.checked);
  }, []);

  const handleFrameFilterChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    setFrameFilter(e.target.value as FrameFilter);
  }, []);

  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setSearch(e.target.value);
  }, []);

  const handleClearSearch = useCallback(() => {
    setSearch('');
  }, []);

  // Check if unit passes filter
  const passesFilter = useCallback((unit: UnitNode): boolean => {
    if (!filterEnabled) return true;

    // Search filter
    if (search) {
      const searchLower = search.toLowerCase();
      const typeMatch = unit.unit_type.toLowerCase().includes(searchLower);
      const offsetMatch = unit.offset.toString(16).includes(searchLower);
      if (!typeMatch && !offsetMatch) return false;
    }

    // Frame type filter
    switch (frameFilter) {
      case 'All':
        return true;
      case 'KeyOnly':
        return unit.unit_type.includes('KEY') ||
               unit.unit_type.includes('INTRA') ||
               unit.unit_type.includes('IDR');
      case 'InterOnly':
        return unit.frame_index !== undefined &&
               !unit.unit_type.includes('KEY') &&
               !unit.unit_type.includes('INTRA') &&
               !unit.unit_type.includes('IDR');
      case 'FramesOnly':
        return unit.frame_index !== undefined;
      case 'HeadersOnly':
        return unit.unit_type.includes('SEQUENCE') ||
               unit.unit_type.includes('SPS') ||
               unit.unit_type.includes('PPS') ||
               unit.unit_type.includes('VPS') ||
               unit.unit_type.includes('APS');
    }
  }, [filterEnabled, search, frameFilter]);

  // Filter units (flatten hierarchy for filtering)
  const flattenUnits = useCallback((units: UnitNode[]): UnitNode[] => {
    const result: UnitNode[] = [];
    const traverse = (unit: UnitNode) => {
      if (passesFilter(unit)) {
        result.push(unit);
      }
      unit.children.forEach(traverse);
    };
    units.forEach(traverse);
    return result;
  }, [passesFilter]);

  // Convert frames to unit nodes for display
  const frameUnits = useMemo(() => {
    return frames.map(frame => ({
      key: `frame-${frame.frame_index}`,
      unit_type: frame.frame_type,
      offset: 0,
      size: frame.size,
      frame_index: frame.frame_index,
      pts: frame.pts,
      children: [] as UnitNode[],
      qp_avg: undefined,
    } as UnitNode));
  }, [frames]);

  const displayUnits = units.length > 0 ? units : frameUnits;

  const filteredUnits = useMemo(() => {
    return filterEnabled ? flattenUnits(displayUnits) : displayUnits;
  }, [displayUnits, filterEnabled, flattenUnits, frameUnits]);

  // Get color for unit type
  const getUnitColor = (unitType: string): string => {
    if (unitType === 'SEQUENCE_HEADER') return '#64c864';
    if (unitType === 'FRAME' || unitType === 'FRAME_HEADER') return '#6496ff';
    if (unitType === 'TILE_GROUP') return '#c89664';
    if (unitType === 'TEMPORAL_DELIMITER') return '#969696';
    return '#ffffff';
  };

  // Get icon for unit type
  const getUnitIcon = (unit: UnitNode): string => {
    if (unit.frame_index !== undefined) return 'F';
    switch (unit.unit_type) {
      case 'SEQUENCE_HEADER': return 'S';
      case 'TEMPORAL_DELIMITER': return 'T';
      case 'METADATA': return 'M';
      case 'PADDING': return 'P';
      default: return '•';
    }
  };

  // Render unit node recursively
  const renderUnitNode = (unit: UnitNode, depth: number = 0): JSX.Element | null => {
    if (filterEnabled && !passesFilter(unit)) return null;

    const color = getUnitColor(unit.unit_type);
    const icon = getUnitIcon(unit);
    const isSelected = selectedUnitKey === unit.key;
    const isExpanded = expandedNodes.has(unit.key);
    const hasChildren = unit.children.length > 0;

    const label = unit.frame_index !== undefined
      ? `[${icon}] Frame #${unit.frame_index} - ${unit.unit_type} @ 0x${unit.offset.toString(16).padStart(8, '0')} (${unit.size} bytes)`
      : `[${icon}] ${unit.unit_type} @ 0x${unit.offset.toString(16).padStart(8, '0')} (${unit.size} bytes)`;

    return (
      <div key={unit.key} className="stream-tree-node">
        <div
          className={`stream-tree-item ${isSelected ? 'selected' : ''}`}
          style={{ paddingLeft: `${depth * 16 + 8}px` }}
        >
          {hasChildren ? (
            <span
              className={`codicon codicon-${isExpanded ? 'chevron-down' : 'chevron-right'} expand-toggle`}
              onClick={(e) => handleToggleNode(unit.key, e)}
            />
          ) : (
            <span className="expand-placeholder" />
          )}
          <span
            className="stream-tree-label"
            style={{ color }}
            onClick={() => handleUnitSelect(unit)}
            title={label}
          >
            {label}
          </span>
        </div>
        {hasChildren && isExpanded && (
          <div className="stream-tree-children">
            {unit.children.map(child => renderUnitNode(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="stream-tree-panel">
      <div className="panel-header">
        <span className="panel-title">Stream Tree</span>
      </div>

      {/* Filter toolbar */}
      <div className="stream-tree-toolbar">
        <label className="filter-checkbox">
          <input
            type="checkbox"
            checked={filterEnabled}
            onChange={handleFilterEnabledChange}
          />
          <span>Filter</span>
        </label>

        {filterEnabled && (
          <>
            <span className="toolbar-separator"></span>

            {/* Frame type filter */}
            <select
              value={frameFilter}
              onChange={handleFrameFilterChange}
              className="frame-filter-select"
            >
              {FRAME_FILTERS.map(f => (
                <option key={f.value} value={f.value}>{f.label}</option>
              ))}
            </select>

            <span className="toolbar-separator"></span>

            {/* Search */}
            <span className="search-label">Search:</span>
            <input
              type="text"
              value={search}
              onChange={handleSearchChange}
              className="search-input"
              placeholder="Type or offset..."
            />
            {search && (
              <button
                className="clear-search-btn"
                onClick={handleClearSearch}
                title="Clear search"
              >
                <span className="codicon codicon-close"></span>
              </button>
            )}
          </>
        )}
      </div>

      <div className="panel-divider"></div>

      {/* Count info */}
      {filterEnabled && filteredUnits.length !== displayUnits.length && (
        <div className="stream-tree-count">
          Showing {filteredUnits.length} of {displayUnits.length} units
        </div>
      )}

      {/* Tree */}
      <div className="stream-tree-content">
        {displayUnits.length === 0 ? (
          <div className="stream-tree-empty">
            <span className="codicon codicon-symbol-tree"></span>
            <span>No frames loaded</span>
            <span className="stream-tree-empty-hint">Open a file to see stream units</span>
          </div>
        ) : filteredUnits.length > 0 ? (
          filteredUnits.map(unit => (
            <Fragment key={unit.key}>{renderUnitNode(unit)}</Fragment>
          ))
        ) : (
          <div className="stream-tree-empty">
            <span className="codicon codicon-search"></span>
            <span>No matching units</span>
          </div>
        )}
      </div>
    </div>
  );
});
