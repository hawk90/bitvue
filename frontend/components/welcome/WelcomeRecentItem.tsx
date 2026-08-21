function splitPath(path: string): { name: string; dir: string } {
  const lastSep = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (lastSep === -1) return { name: path, dir: "" };
  return { name: path.slice(lastSep + 1), dir: path.slice(0, lastSep) };
}

interface WelcomeRecentItemProps {
  path: string;
  onOpen: (path: string) => void;
  onRemove?: (path: string) => void;
  disabled?: boolean;
}

export function WelcomeRecentItem({
  path,
  onOpen,
  onRemove,
  disabled,
}: WelcomeRecentItemProps) {
  const { name, dir } = splitPath(path);

  return (
    <li className="welcome-recent-item">
      <button
        className="welcome-row welcome-recent-link"
        onClick={() => onOpen(path)}
        disabled={disabled}
        title={path}
      >
        <span className="welcome-row-name">{name}</span>
        {dir && <span className="welcome-row-secondary">{dir}</span>}
      </button>
      {onRemove && (
        <button
          className="welcome-recent-remove"
          aria-label={`Remove ${name} from recent files`}
          disabled={disabled}
          onClick={(e) => {
            // These are DOM siblings, not nested, so this can't actually bubble into the open
            // button's own onClick -- stopPropagation is defensive against a future restructure,
            // not a fix for an observed bug.
            e.stopPropagation();
            onRemove(path);
          }}
        >
          <span className="codicon codicon-close" />
        </button>
      )}
    </li>
  );
}
