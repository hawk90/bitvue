/**
 * Placeholder Panel Component
 *
 * Temporary placeholder for panels under development
 */

import { memo } from 'react';
import './PlaceholderPanel.css';

export interface PlaceholderPanelProps {
  title: string;
  description?: string;
  icon?: string;
}

function PlaceholderPanel({
  title,
  description = 'Coming soon...',
  icon = 'clock',
}: PlaceholderPanelProps) {
  return (
    <div className="placeholder-panel">
      <div className="placeholder-panel-header">
        <h3>{title}</h3>
      </div>
      <div className="placeholder-panel-content">
        <div className="placeholder-panel-icon">
          <span className={`codicon codicon-${icon}`}></span>
        </div>
        <p className="placeholder-panel-text">{description}</p>
      </div>
    </div>
  );
}

// Memoize PlaceholderPanel to prevent unnecessary re-renders
export default memo(PlaceholderPanel);
