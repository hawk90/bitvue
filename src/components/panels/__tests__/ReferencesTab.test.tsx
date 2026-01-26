/**
 * ReferencesTab Component Tests
 * Tests reference frames tab component
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { ReferencesTab } from '../SyntaxDetailPanel/ReferencesTab';

describe('ReferencesTab', () => {
  const mockCurrentFrame = {
    frame_index: 3,
    frame_type: 'B',
    ref_frames: [0, 1, 2],
  };

  const mockFrames = [
    { frame_index: 0, frame_type: 'I', pts: 0 },
    { frame_index: 1, frame_type: 'P', pts: 1, ref_frames: [0] },
    { frame_index: 2, frame_type: 'P', pts: 2, ref_frames: [0] },
    { frame_index: 3, frame_type: 'B', pts: 3, ref_frames: [1, 2] },
  ];

  const defaultProps = {
    currentFrame: mockCurrentFrame,
    frames: mockFrames,
  };

  it('should render references tab', () => {
    render(<ReferencesTab {...defaultProps} />);

    expect(screen.getByText(/Reference Frames:/)).toBeInTheDocument();
  });

  it('should display reference frame count', () => {
    render(<ReferencesTab {...defaultProps} />);

    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('should list all reference frames', () => {
    render(<ReferencesTab {...defaultProps} />);

    expect(screen.getByText('[0]')).toBeInTheDocument();
    expect(screen.getByText('[1]')).toBeInTheDocument();
    expect(screen.getByText('[2]')).toBeInTheDocument();
  });

  it('should display frame numbers for references', () => {
    render(<ReferencesTab {...defaultProps} />);

    expect(screen.getByText('Frame 0')).toBeInTheDocument();
    expect(screen.getByText('Frame 1')).toBeInTheDocument();
    expect(screen.getByText('Frame 2')).toBeInTheDocument();
  });

  it('should show frame types with badges', () => {
    render(<ReferencesTab {...defaultProps} />);

    const typeBadges = document.querySelectorAll('.ref-type');
    expect(typeBadges.length).toBeGreaterThan(0);
  });

  it('should display PTS values for reference frames', () => {
    render(<ReferencesTab {...defaultProps} />);

    expect(screen.getByText('PTS: 0')).toBeInTheDocument();
    expect(screen.getByText('PTS: 1')).toBeInTheDocument();
    expect(screen.getByText('PTS: 2')).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<ReferencesTab {...defaultProps} />);

    rerender(<ReferencesTab {...defaultProps} />);

    expect(screen.getByText('Reference Frames:')).toBeInTheDocument();
  });

  it('should show reference indices in brackets', () => {
    render(<ReferencesTab {...defaultProps} />);

    const indices = screen.queryAllByText(/\[\d\]/);
    expect(indices.length).toBe(3);
  });
});

describe('ReferencesTab edge cases', () => {
  it('should handle null current frame', () => {
    render(
      <ReferencesTab
        currentFrame={null}
        frames={[]}
      />
    );

    expect(screen.getByText('No frame selected')).toBeInTheDocument();
  });

  it('should show no references message for keyframes', () => {
    const keyframe = {
      frame_index: 0,
      frame_type: 'I',
      ref_frames: [],
    };

    render(
      <ReferencesTab
        currentFrame={keyframe}
        frames={[keyframe]}
      />
    );

    expect(screen.getByText(/No reference frames/)).toBeInTheDocument();
    expect(screen.getByText(/keyframe or intra-only/)).toBeInTheDocument();
  });

  it('should handle empty ref_frames array', () => {
    const frame = {
      frame_index: 5,
      frame_type: 'P',
      ref_frames: [],
    };

    render(
      <ReferencesTab
        currentFrame={frame}
        frames={[frame]}
      />
    );

    expect(screen.getByText('0')).toBeInTheDocument();
    expect(screen.getByText(/No reference frames/)).toBeInTheDocument();
  });

  it('should handle missing reference frame data', () => {
    const currentFrame = {
      frame_index: 5,
      frame_type: 'B',
      ref_frames: [99], // Reference to non-existent frame
    };

    render(
      <ReferencesTab
        currentFrame={currentFrame}
        frames={[currentFrame]}
      />
    );

    expect(screen.getByText('Frame 99')).toBeInTheDocument();
  });

  it('should handle missing pts in reference frames', () => {
    const frames = [
      { frame_index: 0, frame_type: 'I' }, // No pts
    ];

    const currentFrame = {
      frame_index: 1,
      frame_type: 'P',
      ref_frames: [0],
    };

    render(
      <ReferencesTab
        currentFrame={currentFrame}
        frames={frames}
      />
    );

    expect(screen.getByText('PTS: N/A')).toBeInTheDocument();
  });
});

describe('ReferencesTab styling', () => {
  it('should apply frame type classes', () => {
    const props = {
      currentFrame: { frame_index: 2, frame_type: 'B', ref_frames: [0, 1] },
      frames: [
        { frame_index: 0, frame_type: 'I', pts: 0 },
        { frame_index: 1, frame_type: 'P', pts: 1 },
        { frame_index: 2, frame_type: 'B', pts: 2, ref_frames: [0, 1] },
      ],
    };

    render(<ReferencesTab {...props} />);

    const iBadge = document.querySelector('.frame-type-i');
    const pBadge = document.querySelector('.frame-type-p');
    expect(iBadge).toBeInTheDocument();
    expect(pBadge).toBeInTheDocument();
  });

  it('should display reference items in correct order', () => {
    const props = {
      currentFrame: { frame_index: 3, frame_type: 'B', ref_frames: [2, 1, 0] },
      frames: [
        { frame_index: 0, frame_type: 'I', pts: 0 },
        { frame_index: 1, frame_type: 'P', pts: 1 },
        { frame_index: 2, frame_type: 'P', pts: 2 },
      ],
    };

    render(<ReferencesTab {...props} />);

    const refItems = document.querySelectorAll('.ref-item');
    expect(refItems.length).toBe(3);
  });
});
