/**
 * Frame Navigation Toolbar
 *
 * Quick navigation controls for frames
 * Features:
 * - Go to first/last frame
 * - Go to next/previous keyframe
 * - Go to next/previous frame by type
 * - Frame search
 */

import { useReducer, useCallback, memo, useEffect } from "react";
import { useFrameData } from "../contexts/FrameDataContext";
import { useCurrentFrame } from "../contexts/CurrentFrameContext";
import {
  findNextKeyframe,
  findPrevKeyframe,
  findNextFrameByType,
  findPrevFrameByType,
  findFrameByNumber,
} from "../utils/frameNavigation";
import "./FrameNavigationToolbar.css";

interface FrameNavigationToolbarProps {
  onNavigate?: (frameIndex: number) => void;
}

// Search state management with useReducer
interface SearchState {
  open: boolean;
  query: string;
  results: number[];
  selectedIndex: number;
}

type SearchAction =
  | { type: "TOGGLE" }
  | { type: "OPEN" }
  | { type: "CLOSE" }
  | { type: "SET_QUERY"; payload: string }
  | { type: "SET_RESULTS"; payload: number[] }
  | { type: "CLEAR" }
  | { type: "SELECT_PREV" }
  | { type: "SELECT_NEXT" };

const initialSearchState: SearchState = {
  open: false,
  query: "",
  results: [],
  selectedIndex: 0,
};

function searchReducer(state: SearchState, action: SearchAction): SearchState {
  switch (action.type) {
    case "TOGGLE":
      return { ...state, open: !state.open };
    case "OPEN":
      return { ...state, open: true };
    case "CLOSE":
      return { ...initialSearchState };
    case "SET_QUERY":
      return { ...state, query: action.payload, selectedIndex: 0 };
    case "SET_RESULTS":
      return {
        ...state,
        results: action.payload,
        selectedIndex: Math.min(state.selectedIndex, action.payload.length - 1),
      };
    case "CLEAR":
      return { ...state, query: "", results: [], selectedIndex: 0 };
    case "SELECT_PREV":
      return { ...state, selectedIndex: Math.max(0, state.selectedIndex - 1) };
    case "SELECT_NEXT":
      return {
        ...state,
        selectedIndex: Math.min(
          state.results.length - 1,
          state.selectedIndex + 1,
        ),
      };
    default:
      return state;
  }
}

export const FrameNavigationToolbar = memo(function FrameNavigationToolbar({
  onNavigate,
}: FrameNavigationToolbarProps) {
  const { frames } = useFrameData();
  const { currentFrameIndex } = useCurrentFrame();
  const [searchState, dispatch] = useReducer(searchReducer, initialSearchState);

  const navigateToFrame = useCallback(
    (index: number) => {
      if (index >= 0 && index < frames.length) {
        onNavigate?.(index);
      }
    },
    [frames.length, onNavigate],
  );

  const goToFirst = useCallback(() => {
    navigateToFrame(0);
  }, [navigateToFrame]);

  const goToLast = useCallback(() => {
    navigateToFrame(frames.length - 1);
  }, [frames.length, navigateToFrame]);

  const goToNextKeyframe = useCallback(() => {
    const nextKeyframe = findNextKeyframe(frames, currentFrameIndex);
    if (nextKeyframe !== null) {
      navigateToFrame(nextKeyframe);
    }
  }, [frames, currentFrameIndex, navigateToFrame]);

  const goToPrevKeyframe = useCallback(() => {
    const prevKeyframe = findPrevKeyframe(frames, currentFrameIndex);
    if (prevKeyframe !== null) {
      navigateToFrame(prevKeyframe);
    }
  }, [frames, currentFrameIndex, navigateToFrame]);

  const goToNextIFrame = useCallback(() => {
    const nextIFrame = findNextFrameByType(frames, currentFrameIndex, "I");
    if (nextIFrame !== null) {
      navigateToFrame(nextIFrame);
    }
  }, [frames, currentFrameIndex, navigateToFrame]);

  const goToPrevIFrame = useCallback(() => {
    const prevIFrame = findPrevFrameByType(frames, currentFrameIndex, "I");
    if (prevIFrame !== null) {
      navigateToFrame(prevIFrame);
    }
  }, [frames, currentFrameIndex, navigateToFrame]);

  // Search handlers
  const handleToggleSearch = useCallback(() => {
    dispatch({ type: "TOGGLE" });
  }, []);

  const handleSearchChange = useCallback(
    (query: string) => {
      dispatch({ type: "SET_QUERY", payload: query });

      if (!query.trim()) {
        dispatch({ type: "SET_RESULTS", payload: [] });
        return;
      }

      // Try to parse as frame number first
      const frameNum = parseInt(query, 10);
      if (!isNaN(frameNum)) {
        const foundIndex = findFrameByNumber(frames, frameNum);
        if (foundIndex !== null) {
          dispatch({ type: "SET_RESULTS", payload: [foundIndex] });
          return;
        }
      }

      // Otherwise search by type or partial match
      const results: number[] = [];
      const lowerQuery = query.toLowerCase();

      frames.forEach((frame, idx) => {
        if (
          frame.frame_type.toLowerCase().includes(lowerQuery) ||
          String(frame.frame_index).includes(lowerQuery)
        ) {
          results.push(idx);
        }
      });

      // Limit results
      dispatch({ type: "SET_RESULTS", payload: results.slice(0, 100) });
    },
    [frames],
  );

  const handleClearSearch = useCallback(() => {
    dispatch({ type: "CLEAR" });
  }, []);

  const handleSearchKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        dispatch({ type: "CLOSE" });
      } else if (e.key === "Enter" && searchState.results.length > 0) {
        navigateToFrame(searchState.results[searchState.selectedIndex]);
        dispatch({ type: "CLOSE" });
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        dispatch({ type: "SELECT_NEXT" });
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        dispatch({ type: "SELECT_PREV" });
      }
    },
    [searchState.results, searchState.selectedIndex, navigateToFrame],
  );

  const goToSearchResult = useCallback(
    (index: number) => {
      navigateToFrame(index);
      dispatch({ type: "CLOSE" });
    },
    [navigateToFrame],
  );

  // Clear search when navigating away
  useEffect(() => {
    if (!searchState.open) {
      dispatch({ type: "CLEAR" });
    }
  }, [searchState.open]);

  if (frames.length === 0) {
    return null;
  }

  return (
    <div className="frame-nav-toolbar">
      {/* Primary navigation */}
      <div className="frame-nav-group">
        <button
          className="frame-nav-btn"
          onClick={goToFirst}
          disabled={currentFrameIndex === 0}
          title="First Frame (Home)"
        >
          <span className="codicon codicon-chevron-double-left"></span>
        </button>
        <button
          className="frame-nav-btn"
          onClick={goToPrevKeyframe}
          disabled={findPrevKeyframe(frames, currentFrameIndex) === null}
          title="Previous Keyframe (K)"
        >
          <span className="codicon codicon-arrow-left"></span>
          <span className="frame-nav-btn-label">KEY</span>
        </button>
      </div>

      <div className="frame-nav-info">
        <span className="frame-nav-current">{currentFrameIndex}</span>
        <span className="frame-nav-separator">/</span>
        <span className="frame-nav-total">{frames.length - 1}</span>
      </div>

      <div className="frame-nav-group">
        <button
          className="frame-nav-btn"
          onClick={goToNextKeyframe}
          disabled={findNextKeyframe(frames, currentFrameIndex) === null}
          title="Next Keyframe (K)"
        >
          <span className="frame-nav-btn-label">KEY</span>
          <span className="codicon codicon-arrow-right"></span>
        </button>
        <button
          className="frame-nav-btn"
          onClick={goToLast}
          disabled={currentFrameIndex >= frames.length - 1}
          title="Last Frame (End)"
        >
          <span className="codicon codicon-chevron-double-right"></span>
        </button>
      </div>

      {/* Frame type navigation */}
      <div className="frame-nav-divider"></div>

      <div className="frame-nav-group">
        <button
          className="frame-nav-btn frame-nav-btn-type"
          onClick={goToPrevIFrame}
          disabled={
            findPrevFrameByType(frames, currentFrameIndex, "I") === null
          }
          title="Previous I-Frame"
        >
          <span className="frame-nav-type-badge frame-type-i">I</span>
          <span className="codicon codicon-arrow-left"></span>
        </button>
        <button
          className="frame-nav-btn frame-nav-btn-type"
          onClick={goToNextIFrame}
          disabled={
            findNextFrameByType(frames, currentFrameIndex, "I") === null
          }
          title="Next I-Frame"
        >
          <span className="codicon codicon-arrow-right"></span>
          <span className="frame-nav-type-badge frame-type-i">I</span>
        </button>
      </div>

      {/* Search button */}
      <div className="frame-nav-divider"></div>

      <div className="frame-nav-group">
        <button
          className={`frame-nav-btn frame-nav-search ${searchState.open ? "active" : ""}`}
          onClick={handleToggleSearch}
          title="Search Frames (Ctrl+F)"
        >
          <span className="codicon codicon-search"></span>
        </button>
      </div>

      {/* Search dropdown */}
      {searchState.open && (
        <div className="frame-nav-search-dropdown">
          <div className="frame-nav-search-input">
            <span className="codicon codicon-search"></span>
            <input
              type="text"
              placeholder="Frame # or type..."
              value={searchState.query}
              onChange={(e) => handleSearchChange(e.target.value)}
              autoFocus
              onKeyDown={handleSearchKeyDown}
            />
            {searchState.query && (
              <button
                className="frame-nav-search-clear"
                onClick={handleClearSearch}
              >
                <span className="codicon codicon-close"></span>
              </button>
            )}
          </div>

          {searchState.query && searchState.results.length > 0 && (
            <div className="frame-nav-search-results">
              <div className="frame-nav-search-count">
                {searchState.results.length} result
                {searchState.results.length !== 1 ? "s" : ""}
              </div>
              {searchState.results.slice(0, 10).map((idx, i) => {
                const frame = frames[idx];
                const isSelected = i === searchState.selectedIndex;
                return (
                  <div
                    key={idx}
                    className={`frame-nav-search-result ${isSelected ? "selected" : ""}`}
                    onClick={() => goToSearchResult(idx)}
                  >
                    <span
                      className={`frame-nav-result-badge frame-type-${frame.frame_type.toLowerCase()}`}
                    >
                      {frame.frame_type}
                    </span>
                    <span className="frame-nav-result-index">
                      #{frame.frame_index}
                    </span>
                  </div>
                );
              })}
              {searchState.results.length > 10 && (
                <div className="frame-nav-search-more">
                  ...and {searchState.results.length - 10} more
                </div>
              )}
            </div>
          )}

          {searchState.query && searchState.results.length === 0 && (
            <div className="frame-nav-search-empty">No results found</div>
          )}

          <div className="frame-nav-search-tips">
            <div className="frame-nav-search-tip">
              Type frame number or type. Use ↑↓ to navigate, Enter to select.
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
