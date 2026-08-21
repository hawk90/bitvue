import type { WelcomeSampleEntry } from "./sampleCatalog";

interface WelcomeSampleItemProps extends WelcomeSampleEntry {
  /** Resolved absolute path, or null while still resolving / if resolution failed. Only ever
   *  populated for `available` entries -- see useSamplePaths. */
  path: string | null;
  onOpen: (path: string) => void;
  disabled?: boolean;
}

export function WelcomeSampleItem({
  container,
  filename,
  available,
  path,
  onOpen,
  disabled,
}: WelcomeSampleItemProps) {
  const clickable = available && path !== null;

  return (
    <li className="welcome-sample-item">
      <button
        className="welcome-row welcome-sample-link"
        onClick={() => path && onOpen(path)}
        disabled={!clickable || disabled}
        title={clickable ? path : "Not yet supported by the analysis backend"}
      >
        <span className="welcome-row-name">{container}</span>
        <span className="welcome-row-secondary">{filename}</span>
        {!available && (
          <span className="welcome-sample-badge">Coming soon</span>
        )}
      </button>
    </li>
  );
}
