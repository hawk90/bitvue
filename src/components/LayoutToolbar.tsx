/**
 * Layout Toolbar
 *
 * Toolbar buttons for layout management
 * Reset layout, toggle panels, etc.
 */

import { useLayout } from '../contexts/LayoutContext';
import { memo, useCallback } from 'react';
import './LayoutToolbar.css';

export const LayoutToolbar = memo(function LayoutToolbar() {
  const { resetLayout } = useLayout();

  const handleReset = useCallback(() => {
    resetLayout();
    // Force page reload to apply reset
    window.location.reload();
  }, [resetLayout]);

  return (
    <div className="layout-toolbar">
      <button
        className="layout-toolbar-btn"
        onClick={handleReset}
        title="Reset Layout to Default"
      >
        <i className="codicon codicon-screen-normal" />
        <span>Reset Layout</span>
      </button>
    </div>
  );
});
