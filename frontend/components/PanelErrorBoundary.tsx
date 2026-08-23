/**
 * Panel-scoped error boundary (EDGE-01 fix).
 *
 * Before this, the only `ErrorBoundary` in the app was the single one wrapping the entire tree
 * in App.tsx -- a render-time throw inside any ONE panel (a bad grid overlay, a malformed frame,
 * etc.) took down the whole app to one generic full-screen fallback, not just that panel. Async/
 * data-fetch errors were already well-isolated per-panel (each panel catches its own promise
 * rejections into a real error state, e.g. YuvViewerPanel's `loadError`) -- this specifically
 * covers synchronous render exceptions, which had no per-panel isolation at all.
 *
 * Reuses `ErrorBoundary`'s real catch/reset logic, just with a fallback sized to fill the
 * panel's own layout slot (not `position: fixed` over the whole viewport like the app-wide one).
 */

import { useMemo, type ReactNode } from "react";
import { ErrorBoundary, type ErrorFallbackProps } from "./ErrorBoundary";
import "./PanelErrorBoundary.css";

function PanelErrorFallback({
  panelName,
  error,
  resetError,
}: ErrorFallbackProps & { panelName: string }) {
  return (
    <div className="panel-error-fallback">
      <span className="codicon codicon-error" aria-hidden="true" />
      <div className="panel-error-message">
        <strong>{panelName}</strong> panel failed to render
        {error && <div className="panel-error-detail">{error.message}</div>}
      </div>
      <button className="panel-error-retry" onClick={resetError}>
        <span className="codicon codicon-refresh" aria-hidden="true" />
        Retry
      </button>
    </div>
  );
}

export function PanelErrorBoundary({
  panelName,
  children,
}: {
  panelName: string;
  children: ReactNode;
}) {
  // Stable identity per panelName (all call sites pass a static string) -- avoids remounting the
  // fallback element on every parent re-render while an error is being shown.
  const fallback = useMemo(
    () =>
      function BoundPanelErrorFallback(props: ErrorFallbackProps) {
        return <PanelErrorFallback panelName={panelName} {...props} />;
      },
    [panelName],
  );

  return <ErrorBoundary fallback={fallback}>{children}</ErrorBoundary>;
}
