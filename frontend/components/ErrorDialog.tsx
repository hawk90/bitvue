/**
 * Error Dialog Component
 *
 * Modal dialog for displaying critical errors with details
 */

import { useEffect, memo } from "react";
import "./ErrorDialog.css";

interface ErrorDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  details?: string;
  errorCode?: string;
  onClose: () => void;
  onDismiss?: () => void;
  showViewDetails?: boolean;
}

export const ErrorDialog = memo(function ErrorDialog({
  isOpen,
  title,
  message,
  details,
  errorCode,
  onClose,
  onDismiss,
  showViewDetails = true,
}: ErrorDialogProps) {
  // Handle ESC key to close
  useEffect(() => {
    if (!isOpen) return;

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="error-dialog-overlay" onClick={onClose}>
      <div className="error-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="error-dialog-header">
          <div className="error-dialog-icon">
            <i className="codicon codicon-error" />
          </div>
          <h2 className="error-dialog-title">{title}</h2>
        </div>

        <div className="error-dialog-content">
          <p className="error-dialog-message">{message}</p>

          {errorCode && (
            <div className="error-dialog-code">
              <span className="error-dialog-code-label">Error Code:</span>
              <span className="error-dialog-code-value">{errorCode}</span>
            </div>
          )}

          {showViewDetails && details && (
            <details className="error-dialog-details">
              <summary>View Details</summary>
              <pre className="error-dialog-details-text">{details}</pre>
            </details>
          )}
        </div>

        <div className="error-dialog-footer">
          {onDismiss && (
            <button
              className="error-dialog-button error-dialog-button-secondary"
              onClick={onDismiss}
            >
              Dismiss
            </button>
          )}
          <button
            className="error-dialog-button error-dialog-button-primary"
            onClick={onClose}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
});

/**
 * Non-critical warning banner (inline)
 */
interface WarningBannerProps {
  message: string;
  onDismiss?: () => void;
  severity?: "warning" | "info";
}

export const WarningBanner = memo(function WarningBanner({
  message,
  onDismiss,
  severity = "warning",
}: WarningBannerProps) {
  return (
    <div className={`warning-banner warning-banner-${severity}`}>
      <span className="warning-banner-icon">
        <i
          className={`codicon codicon-${severity === "warning" ? "warning" : "info"}`}
        />
      </span>
      <span className="warning-banner-message">{message}</span>
      {onDismiss && (
        <button
          className="warning-banner-dismiss"
          onClick={onDismiss}
          aria-label="Dismiss"
        >
          <i className="codicon codicon-close" />
        </button>
      )}
    </div>
  );
});

/**
 * Error toast notification (temporary, auto-dismiss)
 */
interface ErrorToastProps {
  message: string;
  duration?: number;
  onClose: () => void;
}

export const ErrorToast = memo(function ErrorToast({
  message,
  duration = 5000,
  onClose,
}: ErrorToastProps) {
  useEffect(() => {
    const timer = setTimeout(onClose, duration);
    return () => clearTimeout(timer);
  }, [duration, onClose]);

  return (
    <div className="error-toast">
      <span className="error-toast-icon">
        <i className="codicon codicon-error" />
      </span>
      <span className="error-toast-message">{message}</span>
      <button
        className="error-toast-dismiss"
        onClick={onClose}
        aria-label="Dismiss"
      >
        <i className="codicon codicon-close" />
      </button>
    </div>
  );
});
