import { WELCOME_PRACTICE_ITEMS } from "./practiceCatalog";
import "./welcomeShared.css";
import "./WelcomePracticeList.css";

interface WelcomePracticeListProps {
  /** Resolved path to the AV1 sample every row opens -- null while still resolving or if
   *  resolution failed (see PRACTICE_SAMPLE_FILENAME in sampleCatalog.ts). */
  samplePath: string | null;
  onOpen: (path: string) => void;
  disabled?: boolean;
}

export function WelcomePracticeList({
  samplePath,
  onOpen,
  disabled,
}: WelcomePracticeListProps) {
  const clickable = samplePath !== null;

  return (
    <div className="welcome-section">
      <div className="welcome-section-title">Practice</div>
      <div className="welcome-practice-list">
        {WELCOME_PRACTICE_ITEMS.map((item) => (
          <button
            key={item.title}
            className="welcome-row welcome-practice-link"
            onClick={() => samplePath && onOpen(samplePath)}
            disabled={!clickable || disabled}
            title={
              clickable
                ? "Opens the AV1 sample stream"
                : "Sample not available yet"
            }
          >
            <span className="welcome-row-icon">
              <span
                className={`codicon codicon-${item.icon}`}
                aria-hidden="true"
              />
            </span>
            <span className="welcome-practice-text">
              <span className="welcome-practice-title">{item.title}</span>
              <span className="welcome-practice-desc">{item.description}</span>
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
