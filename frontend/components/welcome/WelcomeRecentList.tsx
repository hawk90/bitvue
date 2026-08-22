import { WelcomeRecentItem } from "./WelcomeRecentItem";
import "./welcomeShared.css";
import "./WelcomeRecentList.css";

interface WelcomeRecentListProps {
  files: string[];
  onOpen: (path: string) => void;
  onRemove?: (path: string) => void;
  disabled?: boolean;
}

/** Renders nothing when there are no (validated) recent files -- callers don't need to guard
 *  the "Recent" section header themselves. */
export function WelcomeRecentList({
  files,
  onOpen,
  onRemove,
  disabled,
}: WelcomeRecentListProps) {
  if (files.length === 0) return null;

  return (
    <div className="welcome-section">
      <div className="welcome-section-title">Recent</div>
      <ul
        className="welcome-recent-list"
        aria-label="Recently opened bitstream files"
      >
        {files.map((path) => (
          <WelcomeRecentItem
            key={path}
            path={path}
            onOpen={onOpen}
            onRemove={onRemove}
            disabled={disabled}
          />
        ))}
      </ul>
    </div>
  );
}
