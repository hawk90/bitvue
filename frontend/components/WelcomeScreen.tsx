/**
 * Welcome Screen
 *
 * Pure presentation: render + user-event callbacks only. Recent-file persistence/validation and
 * the "is an open in flight" state all live upstream (see WelcomeContainer, useRecentFiles,
 * useValidatedRecentFiles, useWelcomeActions) -- this component doesn't call any IPC bridge
 * itself.
 */

import { memo } from "react";
import { isMacOS } from "../utils/platform";
import { WelcomeHeader } from "./welcome/WelcomeHeader";
import { WelcomeActionRow } from "./welcome/WelcomeActionRow";
import { WelcomeRecentList } from "./welcome/WelcomeRecentList";
import { WelcomeSampleList } from "./welcome/WelcomeSampleList";
import { WelcomeFooter } from "./welcome/WelcomeFooter";
import "./WelcomeScreen.css";

interface WelcomeScreenProps {
  onOpenFile: () => void;
  loading: boolean;
  error: string | null;
  onShowShortcuts: () => void;
  recentFiles: string[];
  onOpenRecent: (path: string) => void;
  onRemoveRecent?: (path: string) => void;
  /** Resolved absolute paths for the "Samples" list's available entries, keyed by filename. */
  sampleResolvedPaths: Record<string, string | null>;
  onOpenSample: (path: string) => void;
}

const OpenIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <path
      d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);

const SpinnerIcon = (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
    <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" />
  </svg>
);

export const WelcomeScreen = memo(function WelcomeScreen({
  onOpenFile,
  loading,
  error,
  onShowShortcuts,
  recentFiles,
  onOpenRecent,
  onRemoveRecent,
  sampleResolvedPaths,
  onOpenSample,
}: WelcomeScreenProps) {
  const modKey = isMacOS() ? "⌘" : "Ctrl";

  return (
    <div className="welcome-screen" data-testid="welcome-screen">
      <div className="welcome-content">
        <WelcomeHeader />

        <div className="welcome-section">
          <div className="welcome-section-title">Start</div>
          <div className="welcome-action-list">
            <WelcomeActionRow
              icon={loading ? SpinnerIcon : OpenIcon}
              iconSpinning={loading}
              label={loading ? "Opening..." : "Open Bitstream File"}
              shortcut={loading ? undefined : [modKey, "O"]}
              onClick={onOpenFile}
              disabled={loading}
            />
            <WelcomeActionRow
              icon={<span className="codicon codicon-keyboard" />}
              label="Keyboard Shortcuts"
              shortcut={["?"]}
              onClick={onShowShortcuts}
              disabled={loading}
            />
          </div>

          {error && (
            <div className="welcome-error" role="alert">
              <span className="codicon codicon-error" aria-hidden="true" />
              {error}
            </div>
          )}
        </div>

        <WelcomeRecentList
          files={recentFiles}
          onOpen={onOpenRecent}
          onRemove={onRemoveRecent}
          disabled={loading}
        />

        <WelcomeSampleList
          resolvedPaths={sampleResolvedPaths}
          onOpen={onOpenSample}
          disabled={loading}
        />

        <WelcomeFooter />
      </div>
    </div>
  );
});
