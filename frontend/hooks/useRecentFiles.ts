/**
 * Recent Files Hook
 *
 * Persists up to MAX_RECENT file paths across sessions using localStorage.
 * Exposes add / remove / clear helpers and the sorted list.
 */

import { useState, useCallback, useEffect } from "react";
import { createLogger } from "../utils/logger";

const logger = createLogger("useRecentFiles");

const STORAGE_KEY = "bitvue-recent-files";
const MAX_RECENT = 10;

function loadFromStorage(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    // Validate entries are strings and not suspiciously long
    return (parsed as unknown[])
      .filter((v): v is string => typeof v === "string" && v.length < 4096)
      .slice(0, MAX_RECENT);
  } catch {
    return [];
  }
}

function saveToStorage(files: string[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(files));
  } catch (e) {
    logger.warn("Failed to save recent files:", e);
  }
}

export interface UseRecentFilesReturn {
  recentFiles: string[];
  addRecentFile: (path: string) => void;
  removeRecentFile: (path: string) => void;
  clearRecentFiles: () => void;
}

export function useRecentFiles(): UseRecentFilesReturn {
  const [recentFiles, setRecentFiles] = useState<string[]>(loadFromStorage);

  // Sync to localStorage whenever state changes
  useEffect(() => {
    saveToStorage(recentFiles);
  }, [recentFiles]);

  const addRecentFile = useCallback((path: string) => {
    setRecentFiles((prev) => {
      // Move to front if already present, otherwise prepend
      const filtered = prev.filter((p) => p !== path);
      return [path, ...filtered].slice(0, MAX_RECENT);
    });
  }, []);

  const removeRecentFile = useCallback((path: string) => {
    setRecentFiles((prev) => prev.filter((p) => p !== path));
  }, []);

  const clearRecentFiles = useCallback(() => {
    setRecentFiles([]);
  }, []);

  return { recentFiles, addRecentFile, removeRecentFile, clearRecentFiles };
}
