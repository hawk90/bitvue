import { WELCOME_SAMPLE_GROUPS } from "./sampleCatalog";
import { WelcomeSampleGroup } from "./WelcomeSampleGroup";

interface WelcomeSampleListProps {
  /** Resolved absolute paths for the `available` subset of the catalog, keyed by filename (see
   *  useSamplePaths). Unavailable entries never need an entry here. */
  resolvedPaths: Record<string, string | null>;
  onOpen: (path: string) => void;
  disabled?: boolean;
}

export function WelcomeSampleList({
  resolvedPaths,
  onOpen,
  disabled,
}: WelcomeSampleListProps) {
  return (
    <div className="welcome-section">
      <div className="welcome-section-title">Samples</div>
      {WELCOME_SAMPLE_GROUPS.map((group) => (
        <WelcomeSampleGroup
          key={group.codec}
          {...group}
          resolvedPaths={resolvedPaths}
          onOpen={onOpen}
          disabled={disabled}
        />
      ))}
    </div>
  );
}
