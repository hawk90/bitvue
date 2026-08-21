/**
 * Validated Recent Files Hook
 *
 * `useRecentFiles` only owns persistence (what was added, in what order) -- it has no way to
 * know whether a stored path still points at a real file (deleted, moved, or a cleaned-up temp/
 * worktree dir are all common in practice). This hook is the existence-check layer on top of it:
 * given the raw persisted list and a way to remove an entry, it checks each path via the
 * `pathExists` IPC bridge and returns only the ones that still exist, pruning the stale ones from
 * the persisted list too so they don't keep reappearing.
 */

import { useEffect, useRef, useState } from "react";
import { pathExists } from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("useValidatedRecentFiles");

export function useValidatedRecentFiles(
  recentFiles: string[],
  removeRecentFile: (path: string) => void,
): string[] {
  const [validatedRecentFiles, setValidatedRecentFiles] = useState(recentFiles);
  // Paths already confirmed to exist in a prior pass -- re-checking them on every unrelated
  // list change (e.g. a different entry getting added) would be a redundant IPC round-trip per
  // entry; only paths this hook hasn't seen confirmed yet need a fresh check.
  const confirmedExisting = useRef<Set<string>>(new Set());

  useEffect(() => {
    let cancelled = false;
    setValidatedRecentFiles(recentFiles);

    const toCheck = recentFiles.filter(
      (path) => !confirmedExisting.current.has(path),
    );
    if (toCheck.length === 0) return;

    Promise.all(
      toCheck.map(async (path) => ({
        path,
        // Fail open on a check error -- don't hide an entry just because the existence check
        // itself broke.
        exists: await pathExists(path).catch((err) => {
          logger.warn(`pathExists check failed for ${path}, keeping it:`, err);
          return true;
        }),
      })),
    ).then((results) => {
      if (cancelled) return;
      for (const result of results) {
        if (result.exists) confirmedExisting.current.add(result.path);
      }
      const stale = results.filter((r) => !r.exists).map((r) => r.path);
      if (stale.length === 0) return;
      setValidatedRecentFiles((prev) => prev.filter((p) => !stale.includes(p)));
      stale.forEach((path) => removeRecentFile(path));
    });

    return () => {
      cancelled = true;
    };
  }, [recentFiles, removeRecentFile]);

  return validatedRecentFiles;
}
