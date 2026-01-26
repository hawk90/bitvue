/**
 * Bookmarks Panel
 *
 * VQAnalyzer-style bookmarks for quick frame navigation
 */

import { useState, useCallback, useEffect, memo } from 'react';
import { useSelection } from '../contexts/SelectionContext';
import { useStreamData } from '../contexts/StreamDataContext';
import { createLogger } from '../utils/logger';
import './BookmarksPanel.css';

const logger = createLogger('BookmarksPanel');

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

const STORAGE_KEY = 'bitvue-bookmarks';

export const BookmarksPanel = memo(function BookmarksPanel({ className = '' }: BookmarksPanelProps) {
  const { selection, setFrameSelection } = useSelection();
  const { frames } = useStreamData();
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDescription, setEditDescription] = useState('');

  // Load bookmarks from localStorage
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        setBookmarks(parsed);
      }
    } catch (error) {
      logger.error('Failed to load bookmarks:', error);
    }
  }, []);

  // Save bookmarks to localStorage
  const saveBookmarks = useCallback((updatedBookmarks: Bookmark[]) => {
    setBookmarks(updatedBookmarks);
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(updatedBookmarks));
    } catch (error) {
      logger.error('Failed to save bookmarks:', error);
    }
  }, []);

  // Add current frame as bookmark
  const currentFrameIndex = selection?.frame?.frameIndex;

  const addBookmark = useCallback(() => {
    if (currentFrameIndex === undefined) return;

    // Get frame data for type and poc
    const frame = frames.find(f => f.frame_index === currentFrameIndex);
    const frameType = frame?.frame_type ?? 'UNKNOWN';
    const poc = frame?.poc ?? 0;

    const newBookmark: Bookmark = {
      id: `bookmark-${Date.now()}-${currentFrameIndex}`,
      frameIndex: currentFrameIndex,
      frameType,
      poc,
      description: `Frame ${currentFrameIndex}`,
      timestamp: Date.now(),
    };

    const updated = [...bookmarks, newBookmark].sort((a, b) => a.frameIndex - b.frameIndex);
    saveBookmarks(updated);
    setEditingId(null);
  }, [bookmarks, currentFrameIndex, frames, saveBookmarks]);

  // Remove bookmark
  const removeBookmark = useCallback((id: string) => {
    const updated = bookmarks.filter(b => b.id !== id);
    saveBookmarks(updated);
    if (editingId === id) {
      setEditingId(null);
      setEditDescription('');
    }
  }, [bookmarks, saveBookmarks, editingId]);

  // Navigate to bookmark
  const goToBookmark = useCallback((bookmark: Bookmark) => {
    setFrameSelection(
      { stream: 'A', frameIndex: bookmark.frameIndex },
      'bookmarks'
    );
  }, [setFrameSelection]);

  // Start editing bookmark description
  const startEdit = useCallback((bookmark: Bookmark) => {
    setEditingId(bookmark.id);
    setEditDescription(bookmark.description);
  }, []);

  // Save edited description
  const saveEdit = useCallback(() => {
    if (!editingId) return;

    const updated = bookmarks.map(b =>
      b.id === editingId
        ? { ...b, description: editDescription }
        : b
    );
    saveBookmarks(updated);
    setEditingId(null);
    setEditDescription('');
  }, [bookmarks, editingId, editDescription, saveBookmarks]);

  // Cancel editing
  const cancelEdit = useCallback(() => {
    setEditingId(null);
    setEditDescription('');
  }, []);

  const canAddBookmark = currentFrameIndex !== undefined && !bookmarks.some(b => b.frameIndex === currentFrameIndex);

  return (
    <div className={`bookmarks-panel ${className}`}>
      <div className="panel-header">
        <h3>Bookmarks</h3>
        <button
          className="add-bookmark-btn"
          onClick={addBookmark}
          disabled={!canAddBookmark}
          title={canAddBookmark ? 'Bookmark current frame' : 'Already bookmarked'}
        >
          <span className={`codicon codicon-${canAddBookmark ? 'bookmark' : 'bookmark-slash'}`}></span>
        </button>
      </div>

      <div className="panel-content">
        {bookmarks.length === 0 ? (
          <div className="empty-state">
            <span className="codicon codicon-bookmark"></span>
            <p>No bookmarks yet</p>
            <p className="hint">Navigate to a frame and click the bookmark icon to add one</p>
          </div>
        ) : (
          <div className="bookmarks-list">
            {bookmarks.map((bookmark) => (
              <div
                key={bookmark.id}
                className={`bookmark-item ${editingId === bookmark.id ? 'editing' : ''}`}
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
                      if (e.key === 'Enter') {
                        saveEdit();
                      } else if (e.key === 'Escape') {
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
                      <div className="bookmark-frame">#{bookmark.frameIndex}</div>
                      <div className="bookmark-desc">{bookmark.description}</div>
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
          <span className="bookmarks-count">{bookmarks.length} bookmark{bookmarks.length > 1 ? 's' : ''}</span>
        </div>
      )}
    </div>
  );
});
