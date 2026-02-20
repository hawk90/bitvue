/**
 * Search Tab Component
 *
 * Search functionality for frames
 * Search by frame type, number, or PTS value
 * Displays clickable search results
 */

import { memo, useCallback } from "react";

interface SearchResult {
  frame_index: number;
  frame_type: string;
  pts?: number;
  size: number;
}

interface SearchTabProps {
  frames: SearchResult[];
  currentFrameIndex: number;
  searchQuery: string;
  searchResults: number[];
  onSearchChange: (query: string) => void;
  onClearSearch: () => void;
}

export const SearchTab = memo(function SearchTab({
  frames,
  currentFrameIndex,
  searchQuery,
  searchResults,
  onSearchChange,
  onClearSearch,
}: SearchTabProps) {
  const handleNavigateToFrame = useCallback((frameIndex: number) => {
    window.dispatchEvent(
      new CustomEvent("navigate-to-frame", { detail: frameIndex }),
    );
  }, []);

  return (
    <div className="syntax-tab-content">
      <div className="search-container">
        <div className="search-input-wrapper">
          <span className="codicon codicon-search search-icon"></span>
          <input
            type="text"
            className="search-input"
            placeholder="Search by frame type, number, or PTS..."
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            autoFocus
          />
          {searchQuery && (
            <button className="search-clear" onClick={onClearSearch}>
              <span className="codicon codicon-close"></span>
            </button>
          )}
        </div>
        {searchQuery && (
          <div className="search-results-info">
            Found {searchResults.length} result
            {searchResults.length !== 1 ? "s" : ""}
          </div>
        )}
      </div>

      <div className="panel-divider"></div>

      {!searchQuery ? (
        <div className="search-hints">
          <div className="search-hint-title">Search Tips:</div>
          <div className="search-hint-item">
            • Type frame type: "I", "P", "B"
          </div>
          <div className="search-hint-item">• Type frame number: "42"</div>
          <div className="search-hint-item">• Type PTS value</div>
        </div>
      ) : searchResults.length === 0 ? (
        <div className="syntax-empty">
          <span className="codicon codicon-search"></span>
          <span>No results found for "{searchQuery}"</span>
        </div>
      ) : (
        <div className="search-results">
          {searchResults.map((idx) => {
            const frame = frames[idx];
            return (
              <div
                key={idx}
                className={`search-result-item ${idx === currentFrameIndex ? "current" : ""}`}
                onClick={() => handleNavigateToFrame(idx)}
              >
                <span
                  className={`search-result-type frame-type-${frame.frame_type.toLowerCase()}`}
                >
                  {frame.frame_type}
                </span>
                <span className="search-result-index">
                  #{frame.frame_index}
                </span>
                <span className="search-result-pts">
                  PTS: {frame.pts ?? "N/A"}
                </span>
                <span className="search-result-size">
                  {(frame.size / 1024).toFixed(2)} KB
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
});
