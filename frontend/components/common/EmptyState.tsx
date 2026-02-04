/**
 * EmptyState Component
 *
 * Reusable component for displaying empty states
 * Used when there's no data to display (no file loaded, no search results, etc.)
 */

import { memo } from 'react';

export interface EmptyStateProps {
  /** Icon name using VS Code codicon class */
  icon?: string;
  /** Title/headline text */
  title: string;
  /** Optional description text */
  description?: string;
  /** Optional action button */
  action?: {
    label: string;
    onClick: () => void;
  };
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
  /** CSS class name for custom styling */
  className?: string;
}

const SIZE_STYLES = {
  sm: { icon: '24px', title: 'text-sm', spacing: 'gap-2' },
  md: { icon: '32px', title: 'text-base', spacing: 'gap-3' },
  lg: { icon: '48px', title: 'text-lg', spacing: 'gap-4' },
} as const;

export const EmptyState = memo(function EmptyState({
  icon = 'codicon-circle-outline',
  title,
  description,
  action,
  size = 'md',
  className = 'empty-state',
}: EmptyStateProps) {
  const sizeStyle = SIZE_STYLES[size];

  return (
    <div className={`${className} ${sizeStyle.spacing}`}>
      {icon && (
        <span
          className={`codicon ${icon}`}
          style={{ fontSize: sizeStyle.icon, opacity: 0.5 }}
        />
      )}
      <div className="empty-state-content">
        <p className={`empty-state-title ${sizeStyle.title}`}>{title}</p>
        {description && (
          <p className="empty-state-description">{description}</p>
        )}
        {action && (
          <button
            className="btn-primary"
            onClick={action.onClick}
            type="button"
          >
            {action.label}
          </button>
        )}
      </div>
    </div>
  );
});

EmptyState.displayName = 'EmptyState';

/**
 * Pre-configured empty states for common scenarios
 */

export const NoFileLoaded = memo(function NoFileLoaded({ onOpenFile }: { onOpenFile?: () => void }) {
  return (
    <EmptyState
      icon="codicon-folder-opened"
      title="No file loaded"
      description="Open a video bitstream file to begin analysis"
      action={onOpenFile ? { label: 'Open File', onClick: onOpenFile } : undefined}
      size="lg"
    />
  );
});

NoFileLoaded.displayName = 'NoFileLoaded';

export const NoFramesFound = memo(function NoFramesFound() {
  return (
    <EmptyState
      icon="codicon-search"
      title="No frames found"
      description="The loaded file contains no frame information"
      size="md"
    />
  );
});

NoFramesFound.displayName = 'NoFramesFound';

export const NoResultsFound = memo(function NoResultsFound({ query }: { query?: string }) {
  return (
    <EmptyState
      icon="codicon-search"
      title="No results found"
      description={query ? `No matches for "${query}"` : 'Try adjusting your search or filter criteria'}
      size="md"
    />
  );
});

NoResultsFound.displayName = 'NoResultsFound';

export const NoSelection = memo(function NoSelection() {
  return (
    <EmptyState
      icon="codicon-debug-stackframe"
      title="No frame selected"
      description="Select a frame to view detailed information"
      size="sm"
    />
  );
});

NoSelection.displayName = 'NoSelection';

export const NoSearchResults = memo(function NoSearchResults() {
  return (
    <EmptyState
      icon="codicon-search"
      title="No results found"
      description="Try adjusting your search criteria"
      size="sm"
    />
  );
});

NoSearchResults.displayName = 'NoSearchResults';

export const NoReferenceFrames = memo(function NoReferenceFrames() {
  return (
    <EmptyState
      icon="codicon-references"
      title="No reference frames"
      description="This frame does not reference any other frames"
      size="sm"
    />
  );
});

NoReferenceFrames.displayName = 'NoReferenceFrames';

export const PanelEmptyState = memo(function PanelEmptyState({ panelName }: { panelName: string }) {
  return (
    <EmptyState
      icon="codicon-panel"
      title={`${panelName} unavailable`}
      description="Load a file to view this panel"
      size="md"
    />
  );
});

PanelEmptyState.displayName = 'PanelEmptyState';

// Alias for backward compatibility
export const NoFramesSelected = NoSelection;
