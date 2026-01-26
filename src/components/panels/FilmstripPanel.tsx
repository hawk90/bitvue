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

export const FilmstripPanel = memo(function FilmstripPanel({ frames }: FilmstripPanelProps) {
  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column' }}>
      <TimelineFilmstrip frames={frames} />
    </div>
  );
});
