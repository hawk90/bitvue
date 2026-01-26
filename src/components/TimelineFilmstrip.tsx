/**
 * Combined Timeline + Filmstrip Component
 * Timeline on top (80px), Filmstrip below (140px)
 * Synchronized frame selection
 */

import { memo } from 'react';
import Timeline from './Timeline';
import Filmstrip from './Filmstrip';
import type { FrameInfo } from '../types/video';
import './TimelineFilmstrip.css';

interface TimelineFilmstripProps {
  frames: FrameInfo[];
  className?: string;
  viewMode?: 'overview' | 'coding' | 'prediction' | 'transform' | 'qp' | 'mv' | 'reference';
  filmstripCollapsed?: boolean;
}

export const TimelineFilmstrip = memo(function TimelineFilmstrip({ frames, className = '', viewMode = 'overview', filmstripCollapsed = false }: TimelineFilmstripProps) {
  return (
    <div className={`timeline-filmstrip ${filmstripCollapsed ? 'collapsed' : ''} ${className}`}>
      {/* Timeline on top - always visible */}
      <Timeline frames={frames} />

      {/* Filmstrip below - hidden when collapsed */}
      {!filmstripCollapsed && <Filmstrip frames={frames} viewMode={viewMode} />}
    </div>
  );
});

export default TimelineFilmstrip;
