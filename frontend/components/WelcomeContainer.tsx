/**
 * WelcomeContainer
 *
 * Owns the Welcome-specific derived state (validated/existing recent files, open-in-flight
 * guarding against double-clicks) and wires it into the pure-presentation `WelcomeScreen`.
 *
 * `recentFiles`/`removeRecentFile` (`useRecentFiles`) and `onOpenFile`/`openFileAtPath`
 * (`useAppFileOperations`) are passed in as props rather than called here directly -- both of
 * those hooks are also consumed elsewhere in App.tsx (`addRecentFile` feeds
 * `useAppFileOperations`'s `onFileOpened`; `fileInfo`/frame state drives the rest of the app), so
 * a second independent instance of either hook in this component would silently diverge from
 * App.tsx's copy -- two separate localStorage-backed React states, or two separate in-flight
 * stream states -- instead of sharing one source of truth.
 */

import { WelcomeScreen } from "./WelcomeScreen";
import { useValidatedRecentFiles } from "../hooks/useValidatedRecentFiles";
import { useWelcomeActions } from "../hooks/useWelcomeActions";
import { useSamplePaths } from "../hooks/useSamplePaths";
import { AVAILABLE_SAMPLE_FILENAMES } from "./welcome/sampleCatalog";

interface WelcomeContainerProps {
  onOpenFile: () => Promise<void>;
  openFileAtPath: (path: string) => Promise<void>;
  error: string | null;
  onShowShortcuts: () => void;
  recentFiles: string[];
  removeRecentFile: (path: string) => void;
}

export function WelcomeContainer({
  onOpenFile,
  openFileAtPath,
  error,
  onShowShortcuts,
  recentFiles,
  removeRecentFile,
}: WelcomeContainerProps) {
  const validatedRecentFiles = useValidatedRecentFiles(
    recentFiles,
    removeRecentFile,
  );
  const { isOpening, handleOpenFile, handleOpenRecent } = useWelcomeActions(
    onOpenFile,
    openFileAtPath,
  );
  // Samples reuse the exact same "open a known path" operation/guard as Recent -- there's
  // nothing sample-specific about opening one once its path is resolved.
  const sampleResolvedPaths = useSamplePaths(AVAILABLE_SAMPLE_FILENAMES);

  return (
    <WelcomeScreen
      onOpenFile={handleOpenFile}
      loading={isOpening}
      error={error}
      onShowShortcuts={onShowShortcuts}
      recentFiles={validatedRecentFiles}
      onOpenRecent={handleOpenRecent}
      onRemoveRecent={removeRecentFile}
      sampleResolvedPaths={sampleResolvedPaths}
      onOpenSample={handleOpenRecent}
    />
  );
}
