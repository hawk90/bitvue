/**
 * Resolves bundled sample filenames (e.g. "foreman_av1.ivf") to their real absolute paths, for
 * the welcome screen's "Samples" quick-open list. Only ever called with the *available* subset
 * of the catalog (see sampleCatalog.ts) -- there's nothing to resolve for a row that isn't
 * clickable yet.
 */

import { useEffect, useState } from "react";
import { getSamplePath } from "../services/electronBridgeService";
import { createLogger } from "../utils/logger";

const logger = createLogger("useSamplePaths");

export function useSamplePaths(
  filenames: string[],
): Record<string, string | null> {
  const [paths, setPaths] = useState<Record<string, string | null>>({});

  useEffect(() => {
    let cancelled = false;
    Promise.all(
      filenames.map(async (filename) => ({
        filename,
        path: await getSamplePath(filename).catch((err) => {
          logger.warn(`Failed to resolve sample path for ${filename}:`, err);
          return null;
        }),
      })),
    ).then((results) => {
      if (cancelled) return;
      const map: Record<string, string | null> = {};
      for (const result of results) map[result.filename] = result.path;
      setPaths(map);
    });
    return () => {
      cancelled = true;
    };
    // `filenames` is expected to be a stable module-level reference (e.g.
    // sampleCatalog.ts's AVAILABLE_SAMPLE_FILENAMES), not an inline array literal -- an inline
    // literal would be a new reference every render and re-trigger this effect in a loop (see
    // useValidatedRecentFiles.test.ts's doc comment for how that class of bug actually happened
    // once already this session).
  }, [filenames]);

  return paths;
}
