import { useState } from "react";
import type { WelcomeSampleGroup as WelcomeSampleGroupData } from "./sampleCatalog";
import { WelcomeSampleItem } from "./WelcomeSampleItem";

interface WelcomeSampleGroupProps extends WelcomeSampleGroupData {
  resolvedPaths: Record<string, string | null>;
  onOpen: (path: string) => void;
  disabled?: boolean;
}

export function WelcomeSampleGroup({
  codec,
  defaultExpanded,
  entries,
  resolvedPaths,
  onOpen,
  disabled,
}: WelcomeSampleGroupProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const availableCount = entries.filter((e) => e.available).length;

  return (
    <div className="welcome-sample-group">
      <button
        className="welcome-sample-group-header"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
      >
        <span
          className={`codicon codicon-chevron-right welcome-sample-group-chevron${expanded ? " expanded" : ""}`}
          aria-hidden="true"
        />
        <span className="welcome-sample-group-codec">{codec}</span>
        <span className="welcome-sample-group-count">
          {entries.length} format{entries.length === 1 ? "" : "s"}
          {availableCount > 0 && ` · ${availableCount} ready`}
        </span>
      </button>
      {expanded && (
        <ul
          className="welcome-sample-list"
          aria-label={`${codec} sample bitstreams by container`}
        >
          {entries.map((entry) => (
            <WelcomeSampleItem
              key={entry.container}
              {...entry}
              path={resolvedPaths[entry.filename] ?? null}
              onOpen={onOpen}
              disabled={disabled}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
