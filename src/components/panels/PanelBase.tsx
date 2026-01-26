/**
 * Panel Base Component
 *
 * Provides a consistent structure for all panel components.
 * Includes header with title and close button, content area, and optional footer.
 *
 * Features:
 * - Size variants (sm, md, lg, xl)
 * - Optional icon in header
 * - Optional footer content
 * - Custom CSS class support
 * - Consistent styling via PanelBase.css
 */

import { memo, ReactNode } from 'react';
import './PanelBase.css';

export interface PanelBaseProps {
  /** Panel visibility */
  visible?: boolean;
  /** Close callback */
  onClose?: () => void;
  /** Panel title */
  title: string;
  /** Icon class name (e.g., 'codicon-search') */
  icon?: string;
  /** Panel content */
  children: ReactNode;
  /** Optional footer content */
  footer?: ReactNode;
  /** Additional CSS classes */
  className?: string;
  /** Panel size variant */
  size?: 'sm' | 'md' | 'lg' | 'xl';
  /** Whether to show the close button */
  showCloseButton?: boolean;
  /** Additional header content */
  headerExtra?: ReactNode;
}

/**
 * PanelBase component for consistent panel structure
 */
export const PanelBase = memo(function PanelBase({
  visible = true,
  onClose,
  title,
  icon,
  children,
  footer,
  className = '',
  size,
  showCloseButton = true,
  headerExtra,
}: PanelBaseProps) {
  if (!visible) {
    return null;
  }

  const sizeClass = size ? `panel--${size}` : '';
  const classes = `panel-container ${sizeClass} ${className}`.trim();

  return (
    <div className={classes}>
      {/* Header */}
      <div className="panel-header">
        {icon && <span className={`codicon ${icon}`} style={{ marginRight: '8px' }} />}
        <span className="panel-title">{title}</span>
        {headerExtra}
        {showCloseButton && onClose && (
          <button
            className="panel-close-button"
            onClick={onClose}
            aria-label="Close"
          >
            <span className="codicon codicon-close" />
          </button>
        )}
      </div>

      {/* Content */}
      <div className="panel-content">
        {children}
      </div>

      {/* Footer */}
      {footer && (
        <div className="panel-footer">
          {footer}
        </div>
      )}
    </div>
  );
});

/**
 * PanelSection component for grouping related content
 */
export interface PanelSectionProps {
  children: ReactNode;
  title?: string;
  className?: string;
}

export const PanelSection = memo(function PanelSection({
  children,
  title,
  className = '',
}: PanelSectionProps) {
  return (
    <div className={`panel-section ${className}`.trim()}>
      {title && <div className="panel-section-title">{title}</div>}
      {children}
    </div>
  );
});

/**
 * PanelInfoRow component for displaying label-value pairs
 */
export interface PanelInfoRowProps {
  label: string;
  value: ReactNode;
  className?: string;
}

export const PanelInfoRow = memo(function PanelInfoRow({
  label,
  value,
  className = '',
}: PanelInfoRowProps) {
  return (
    <div className={`panel-info-row ${className}`.trim()}>
      <span className="panel-info-label">{label}</span>
      <span className="panel-info-value">{value}</span>
    </div>
  );
});

/**
 * PanelEmpty component for empty state display
 */
export interface PanelEmptyProps {
  icon?: string;
  message: string;
  className?: string;
}

export const PanelEmpty = memo(function PanelEmpty({
  icon = 'codicon-circle-slash',
  message,
  className = '',
}: PanelEmptyProps) {
  return (
    <div className={`panel-empty ${className}`.trim()}>
      {icon && <span className={`codicon ${icon}`} />}
      <span>{message}</span>
    </div>
  );
});

/**
 * PanelLoading component for loading state display
 */
export interface PanelLoadingProps {
  message?: string;
  className?: string;
}

export const PanelLoading = memo(function PanelLoading({
  message = 'Loading...',
  className = '',
}: PanelLoadingProps) {
  return (
    <div className={`panel-loading ${className}`.trim()}>
      <span className="codicon codicon-loading" />
      <span>{message}</span>
    </div>
  );
});
