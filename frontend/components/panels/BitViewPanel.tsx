/**
 * Bit View Panel
 *
 * Binary/bit-level view of syntax elements
 * Shows individual bits with highlighting for selected fields
 *
 * Reference: crates/ui/src/panels/bit_view.rs
 */

import PlaceholderPanel from './PlaceholderPanel';
import { memo } from 'react';

export const BitViewPanel = memo(function BitViewPanel() {
  return (
    <PlaceholderPanel
      title="Bit View"
      description="Binary/bit-level syntax element display"
      icon="symbol-boolean"
    />
  );
});
