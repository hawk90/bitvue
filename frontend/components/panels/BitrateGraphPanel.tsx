/**
 * Bitrate Graph Panel
 *
 * Bitrate visualization over time
 * Shows frame sizes and bitrate changes
 *
 * Reference: crates/ui/src/panels/bitrate_graph.rs
 */

import PlaceholderPanel from './PlaceholderPanel';
import { memo } from 'react';

export const BitrateGraphPanel = memo(function BitrateGraphPanel() {
  return (
    <PlaceholderPanel
      title="Bitrate Graph"
      description="Frame size and bitrate over time"
      icon="graph"
    />
  );
});
