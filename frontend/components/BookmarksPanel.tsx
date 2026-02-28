/**
 * Bookmarks Panel
 *
 * bookmarks for quick frame navigation
 */

import { useState, useCallback, useEffect, memo, useRef } from "react";
import { useSelection } from "../contexts/SelectionContext";
import { useFrameData } from "../contexts/FrameDataContext";
import { createLogger } from "../utils/logger";
import { TIMING } from "../constants/ui";
import "./BookmarksPanel.css";

const logger = createLogger("BookmarksPanel");

export interface Bookmark {
  id: string;
  frameIndex: number;
  frameType: string;
  poc: number;
  description: string;
  timestamp: number;
}

interface BookmarksPanelProps {
  className?: string;
}

const STORAGE_KEY = "bitvue-bookmarks";

/**
 * Type guard to validate Bookmark structure
 * Prevents prototype pollution and invalid data from being used
 */
function isValidBookmark(data: unknown): data is Bookmark {
  // Check if data is a plain object
  if (typeof data !== "object" || data === null || Array.isArray(data)) {
    return false;
  }

  // Prevent prototype pollution: ensure data is a plain object
  if (Object.getPrototypeOf(data) !== Object.prototype) {
    return false;
  }

  const bookmark = data as Record<string, unknown>;

  // Validate required fields
  if (typeof bookmark.id !== "string" || bookmark.id.trim() === "") {
    return false;
  }

  if (
    typeof bookmark.frameIndex !== "number" ||
    !Number.isFinite(bookmark.frameIndex) ||
    bookmark.frameIndex < 0
  ) {
    return false;
  }

  if (
    typeof bookmark.frameType !== "string" ||
    bookmark.frameType.trim() === ""
  ) {
    return false;
  }

  if (typeof bookmark.poc !== "number" || !Number.isFinite(bookmark.poc)) {
    return false;
  }

  if (typeof bookmark.description !== "string") {
    return false;
  }

  if (
    typeof bookmark.timestamp !== "number" ||
    !Number.isFinite(bookmark.timestamp) ||
    bookmark.timestamp < 0
  ) {
    return false;
  }

  // Check for unexpected properties
  const allowedKeys = [
    "id",
    "frameIndex",
    "frameType",
    "poc",
    "description",
    "timestamp",
  ];
  const actualKeys = Object.keys(bookmark);
  for (const key of actualKeys) {
    if (!allowedKeys.includes(key)) {
      return false;
    }
  }

  return true;
}

/**
 * Validate and sanitize an array of bookmarks
 */
function isValidBookmarkArray(data: unknown): data is Bookmark[] {
  if (!Array.isArray(data)) {
    return false;
  }

  // Check each bookmark
  return data.every(isValidBookmark);
}

export const BookmarksPanel = memo(function BookmarksPanel({
  className = "",
}: BookmarksPanelProps) {
  const { selection, setFrameSelection } = useSelection();
  const { frames } = useFrameData();
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDescription, setEditDescription] = useState("");
  // Ref to store timeout for debounced saves
  const saveTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Load bookmarks from localStorage
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        // Use type guard for comprehensive validation
        if (isValidBookmarkArray(parsed)) {
          setBookmarks(parsed);
          logger.info(
            `Successfully loaded ${parsed.length} bookmarks from storage`,
          );
        } else {
          logger.warn(
            "Invalid bookmarks structure in storage, using empty array",
          );
          setBookmarks([]);
        }
      }
    } catch (error) {
      logger.error("Failed to load bookmarks:", error);
      setBookmarks([]);
    }
  }, []);

  // Debounced save to localStorage
  // Saves bookmarks after a delay, reducing write frequency
  useEffect(() => {
    // Skip saving in test environment or if no bookmarks
    if (process.env.NODE_ENV === "test" || bookmarks.length === 0) {
      return;
    }

    // Clear any pending save
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }

    // Debounce the save operation
    saveTimeoutRef.current = setTimeout(() => {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(bookmarks));
        logger.debug(`Saved ${bookmarks.length} bookmarks to storage`);
      } catch (error) {
        logger.error("Failed to save bookmarks:", error);
      }
      saveTimeoutRef.current = null;
    }, TIMING.STORAGE_DEBOUNCE_DELAY);

    // Cleanup: cancel pending save on unmount
    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, [bookmarks]);

  // Update bookmarks state (save is debounced via useEffect)
  const saveBookmarks = useCallback((updatedBookmarks: Bookmark[]) => {
    setBookmarks(updatedBookmarks);
  }, []);

  // Add current frame as bookmark
  const currentFrameIndex = selection?.frame?.frameIndex;

  const addBookmark = useCallback(() => {
    if (currentFrameIndex === undefined) return;

    // Get frame data for type and poc
    const frame = frames.find((f) => f.frame_index === currentFrameIndex);
    const frameType = frame?.frame_type ?? "UNKNOWN";
    const poc = frame?.poc ?? 0;

    const newBookmark: Bookmark = {
      id: `bookmark-${Date.now()}-${currentFrameIndex}`,
      frameIndex: currentFrameIndex,
      frameType,
      poc,
      description: `Frame ${currentFrameIndex}`,
      timestamp: Date.now(),
    };

    const updated = [...bookmarks, newBookmark].sort(
      (a, b) => a.frameIndex - b.frameIndex,
    );
    saveBookmarks(updated);
    setEditingId(null);
  }, [bookmarks, currentFrameIndex, frames, saveBookmarks]);

  // Remove bookmark
  const removeBookmark = useCallback(
    (id: string) => {
      const updated = bookmarks.filter((b) => b.id !== id);
      saveBookmarks(updated);
      if (editingId === id) {
        setEditingId(null);
        setEditDescription("");
      }
    },
    [bookmarks, saveBookmarks, editingId],
  );

  // Navigate to bookmark
  const goToBookmark = useCallback(
    (bookmark: Bookmark) => {
      setFrameSelection(
        { stream: "A", frameIndex: bookmark.frameIndex },
        "bookmarks",
      );
    },
    [setFrameSelection],
  );

  // Start editing bookmark description
  const startEdit = useCallback((bookmark: Bookmark) => {
    setEditingId(bookmark.id);
    setEditDescription(bookmark.description);
  }, []);

  // Save edited description
  const saveEdit = useCallback(() => {
    if (!editingId) return;

    const updated = bookmarks.map((b) =>
      b.id === editingId ? { ...b, description: editDescription } : b,
    );
    saveBookmarks(updated);
    setEditingId(null);
    setEditDescription("");
  }, [bookmarks, editingId, editDescription, saveBookmarks]);

  // Cancel editing
  const cancelEdit = useCallback(() => {
    setEditingId(null);
    setEditDescription("");
  }, []);

  const canAddBookmark =
    currentFrameIndex !== undefined &&
    !bookmarks.some((b) => b.frameIndex === currentFrameIndex);

  return (
    <div className={`bookmarks-panel ${className}`}>
      <div className="panel-header">
        <h3>Bookmarks</h3>
        <button
          className="add-bookmark-btn"
          onClick={addBookmark}
          disabled={!canAddBookmark}
          title={
            canAddBookmark ? "Bookmark current frame" : "Already bookmarked"
          }
        >
          <span
            className={`codicon codicon-${canAddBookmark ? "bookmark" : "bookmark-slash"}`}
          ></span>
        </button>
      </div>

      <div className="panel-content">
        {bookmarks.length === 0 ? (
          <div className="empty-state">
            <span className="codicon codicon-bookmark"></span>
            <p>No bookmarks yet</p>
            <p className="hint">
              Navigate to a frame and click the bookmark icon to add one
            </p>
          </div>
        ) : (
          <div className="bookmarks-list">
            {bookmarks.map((bookmark) => (
              <div
                key={bookmark.id}
                className={`bookmark-item ${editingId === bookmark.id ? "editing" : ""}`}
              >
                {editingId === bookmark.id ? (
                  <input
                    type="text"
                    className="bookmark-edit-input"
                    value={editDescription}
                    onChange={(e) => setEditDescription(e.target.value)}
                    autoFocus
                    onBlur={saveEdit}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        saveEdit();
                      } else if (e.key === "Escape") {
                        cancelEdit();
                      }
                    }}
                  />
                ) : (
                  <>
                    <div
                      className="bookmark-info"
                      onClick={() => goToBookmark(bookmark)}
                      title={bookmark.description}
                    >
                      <div className="bookmark-frame">
                        #{bookmark.frameIndex}
                      </div>
                      <div className="bookmark-desc">
                        {bookmark.description}
                      </div>
                    </div>
                    <button
                      className="bookmark-edit-btn"
                      onClick={() => startEdit(bookmark)}
                      title="Edit description"
                    >
                      <span className="codicon codicon-edit"></span>
                    </button>
                    <button
                      className="bookmark-remove-btn"
                      onClick={() => removeBookmark(bookmark.id)}
                      title="Remove bookmark"
                    >
                      <span className="codicon codicon-x"></span>
                    </button>
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer with stats */}
      {bookmarks.length > 0 && (
        <div className="panel-footer">
          <span className="bookmarks-count">
            {bookmarks.length} bookmark{bookmarks.length > 1 ? "s" : ""}
          </span>
        </div>
      )}
    </div>
  );
});
