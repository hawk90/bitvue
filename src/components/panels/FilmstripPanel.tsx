/**
 * Filmstrip Panel
 *
 * Timeline/filmstrip visualization panel
 */

import { memo } from 'react';
import { TimelineFilmstrip } from '../TimelineFilmstrip';
import type { FrameInfo } from '../../types/video';

export interface FilmstripPanelProps {
  frames: FrameInfo[];
}

// Memoize container style to avoid creating new object on every render
const FILMSTRIP_PANEL_CONTAINER_STYLE = {
  width: '100%',
  height: '100%',
  display: 'flex',
  flexDirection: 'column' as const,
} as const;

export const FilmstripPanel = memo(function FilmstripPanel({ frames }: FilmstripPanelProps) {
  return (
    <div style={FILMSTRIP_PANEL_CONTAINER_STYLE}>
      <TimelineFilmstrip frames={frames} />
    </div>
  );
});
