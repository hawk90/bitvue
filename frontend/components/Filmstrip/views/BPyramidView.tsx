/**
 * B-Pyramid View Component
 *
 * GOP hierarchy visualization - timeline style with temporal layer rows
 * Frames displayed as small circles with reference arrows
 */

import { useMemo, memo } from 'react';
import { BPyramidViewProps } from './BPyramidTypes';
import { BPyramidTimeline, analyzeTemporalLevels } from './BPyramidTimeline';

function BPyramidView({
  frames,
  currentFrameIndex,
  onFrameClick,
  getFrameTypeColorClass,
}: BPyramidViewProps) {

  const { levels, gopBoundaries, frameMap } = useMemo(
    () => analyzeTemporalLevels(frames),
    [frames]
  );

  if (frames.length === 0) {
    return (
      <div className="bpyramid-view" role="region" aria-label="B-Pyramid">
        <div className="bpyramid-empty" role="status" aria-label="No frames loaded">
          <span className="codicon codicon-graph" aria-hidden="true"></span>
          <p>No frames loaded</p>
        </div>
      </div>
    );
  }

  return (
    <div className="bpyramid-view" role="region" aria-label="B-Pyramid">
      <BPyramidTimeline
        frames={frames}
        currentFrameIndex={currentFrameIndex}
        onFrameClick={onFrameClick}
        getFrameTypeColorClass={getFrameTypeColorClass}
        levels={levels}
        frameMap={frameMap}
        gopBoundaries={gopBoundaries}
      />
    </div>
  );
}

// Memoize BPyramidView to prevent unnecessary re-renders
export default memo(BPyramidView, (prevProps, nextProps) => {
  return (
    prevProps.frames === nextProps.frames &&
    prevProps.currentFrameIndex === nextProps.currentFrameIndex
  );
});
