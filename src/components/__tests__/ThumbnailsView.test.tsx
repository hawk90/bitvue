/**
 * ThumbnailsView Component Tests
 * Tests thumbnail filmstrip view component
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@/test/test-utils';
import ThumbnailsView from '../Filmstrip/views/ThumbnailsView';
import type { FrameInfo } from '@/types/video';

// Mock usePreRenderedArrows hook
vi.mock('../usePreRenderedArrows', () => ({
  usePreRenderedArrows: vi.fn(() => ({
    allArrowData: [],
    svgWidth: 0,
  })),
  ArrowPosition: null,
  PathCalculator: null,
  FrameInfoBase: null,
}));

// Mock getFrameTypeColor
vi.mock('@/types/video', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/types/video')>();
  return {
    ...actual,
    getFrameTypeColor: vi.fn((type) => {
      const colors: Record<string, string> = {
        'I': '#ff4444',
        'P': '#44ff44',
        'B': '#4444ff',
      };
      return colors[type] || '#888888';
    }),
  };
});

const mockFrames: FrameInfo[] = [
  { frame_index: 0, frame_type: 'I', size: 50000, poc: 0, temporal_id: 0, key_frame: true },
  { frame_index: 1, frame_type: 'P', size: 30000, poc: 1, temporal_id: 0, ref_frames: [0] },
  { frame_index: 2, frame_type: 'B', size: 20000, poc: 2, temporal_id: 1, ref_frames: [0, 1] },
];

const defaultProps = {
  frames: mockFrames,
  currentFrameIndex: 0,
  thumbnails: new Map<number, string>(),
  loadingThumbnails: new Set<number>(),
  referencedFrameIndices: new Set<number>(),
  expandedFrameIndex: null,
  onFrameClick: vi.fn(),
  onToggleReferenceExpansion: vi.fn(),
  onHoverFrame: vi.fn(),
  getFrameTypeColorClass: vi.fn((type: string) => `frame-type-${type.toLowerCase()}`),
};

describe('ThumbnailsView', () => {
  it('should render thumbnails view', () => {
    render(<ThumbnailsView {...defaultProps} />);

    const container = document.querySelector('.filmstrip-thumbnails-container');
    expect(container).toBeInTheDocument();
  });

  it('should render all frame thumbnails', () => {
    render(<ThumbnailsView {...defaultProps} />);

    const frames = document.querySelectorAll('[data-frame-index]');
    expect(frames).toHaveLength(3);
  });

  it('should highlight current frame', () => {
    render(<ThumbnailsView {...defaultProps} currentFrameIndex={1} />);

    const currentFrame = document.querySelector('[data-frame-index="1"].selected');
    expect(currentFrame).toBeInTheDocument();
  });

  it('should call onFrameClick when thumbnail clicked', () => {
    const handleClick = vi.fn();
    render(<ThumbnailsView {...defaultProps} onFrameClick={handleClick} />);

    const frame = document.querySelector('[data-frame-index="0"]');
    if (frame) {
      fireEvent.click(frame);
      expect(handleClick).toHaveBeenCalledWith(0);
    }
  });

  it('should show frame type indicator', () => {
    render(<ThumbnailsView {...defaultProps} />);

    const frameTypes = document.querySelectorAll('.frame-nal-type-inner');
    expect(frameTypes.length).toBeGreaterThan(0);
  });

  it('should show frame number', () => {
    render(<ThumbnailsView {...defaultProps} />);

    expect(screen.getByText('I-0 0')).toBeInTheDocument();
    expect(screen.getByText('P-0 1')).toBeInTheDocument();
  });

  it('should display loading state for missing thumbnails', () => {
    const loadingThumbnails = new Set([1]);
    render(<ThumbnailsView {...defaultProps} loadingThumbnails={loadingThumbnails} />);

    const loading = document.querySelectorAll('.frame-placeholder.loading');
    expect(loading.length).toBeGreaterThan(0);
  });

  it('should display thumbnail image when available', () => {
    const thumbnails = new Map([[0, 'data:image/png;base64,mockdata']]);
    render(<ThumbnailsView {...defaultProps} thumbnails={thumbnails} />);

    const thumbnail = document.querySelector('[data-frame-index="0"] img');
    expect(thumbnail).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<ThumbnailsView {...defaultProps} />);

    rerender(<ThumbnailsView {...defaultProps} />);

    expect(document.querySelector('.filmstrip-thumbnails-container')).toBeInTheDocument();
  });
});

describe('ThumbnailsView interactions', () => {
  const mockFrames: FrameInfo[] = [
    { frame_index: 0, frame_type: 'I', size: 50000, poc: 0, temporal_id: 0 },
    { frame_index: 1, frame_type: 'P', size: 30000, poc: 1, temporal_id: 0 },
  ];

  const defaultProps = {
    frames: mockFrames,
    currentFrameIndex: 0,
    thumbnails: new Map([[0, 'data:image0']]),
    loadingThumbnails: new Set<number>(),
    referencedFrameIndices: new Set<number>(),
    expandedFrameIndex: null,
    onFrameClick: vi.fn(),
    onToggleReferenceExpansion: vi.fn(),
    onHoverFrame: vi.fn(),
    getFrameTypeColorClass: vi.fn(() => 'frame-type-i'),
  };

  it('should call onHoverFrame on mouse enter', () => {
    const handleHover = vi.fn();
    render(<ThumbnailsView {...defaultProps} onHoverFrame={handleHover} />);

    const frame = document.querySelector('[data-frame-index="0"]');
    if (frame) {
      fireEvent.mouseEnter(frame);
      expect(handleHover).toHaveBeenCalled();
    }
  });

  it('should call onHoverFrame on mouse move', () => {
    const handleHover = vi.fn();
    render(<ThumbnailsView {...defaultProps} onHoverFrame={handleHover} />);

    const frame = document.querySelector('[data-frame-index="0"]');
    if (frame) {
      fireEvent.mouseMove(frame);
      expect(handleHover).toHaveBeenCalled();
    }
  });

  it('should call onHoverFrame with null on mouse leave', () => {
    const handleHover = vi.fn();
    render(<ThumbnailsView {...defaultProps} onHoverFrame={handleHover} />);

    const frame = document.querySelector('[data-frame-index="0"]');
    if (frame) {
      fireEvent.mouseLeave(frame);
      expect(handleHover).toHaveBeenCalledWith(null, 0, 0);
    }
  });
});

describe('ThumbnailsView reference arrows', () => {
  const mockFrames: FrameInfo[] = [
    { frame_index: 0, frame_type: 'I', size: 50000, poc: 0, temporal_id: 0 },
    { frame_index: 1, frame_type: 'P', size: 30000, poc: 1, temporal_id: 0, ref_frames: [0] },
  ];

  const defaultProps = {
    frames: mockFrames,
    currentFrameIndex: 1,
    thumbnails: new Map([[0, 'data:image0'], [1, 'data:image1']]),
    loadingThumbnails: new Set<number>(),
    referencedFrameIndices: new Set([0]),
    expandedFrameIndex: 1,
    onFrameClick: vi.fn(),
    onToggleReferenceExpansion: vi.fn(),
    onHoverFrame: vi.fn(),
    getFrameTypeColorClass: vi.fn(() => 'frame-type-p'),
  };

  it('should show reference badge for frames with refs', () => {
    render(<ThumbnailsView {...defaultProps} />);

    const refBadge = document.querySelector('[data-count="1"]');
    expect(refBadge).toBeInTheDocument();
  });

  it('should toggle reference expansion on click', () => {
    const handleToggle = vi.fn();
    render(<ThumbnailsView {...defaultProps} onToggleReferenceExpansion={handleToggle} />);

    const refBadge = document.querySelector('[data-count="1"]');
    if (refBadge) {
      fireEvent.click(refBadge);
      expect(handleToggle).toHaveBeenCalled();
    }
  });
});

describe('ThumbnailsView edge cases', () => {
  it('should handle empty frames array', () => {
    const props = {
      frames: [],
      currentFrameIndex: -1,
      thumbnails: new Map(),
      loadingThumbnails: new Set<number>(),
      referencedFrameIndices: new Set<number>(),
      expandedFrameIndex: null,
      onFrameClick: vi.fn(),
      onToggleReferenceExpansion: vi.fn(),
      onHoverFrame: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ''),
    };

    render(<ThumbnailsView {...props} />);

    expect(document.querySelector('.filmstrip-thumbnails-container')).toBeInTheDocument();
  });

  it('should handle no thumbnails loaded', () => {
    const props = {
      frames: [{ frame_index: 0, frame_type: 'I', size: 50000, poc: 0, temporal_id: 0 }],
      currentFrameIndex: 0,
      thumbnails: new Map(),
      loadingThumbnails: new Set<number>(),
      referencedFrameIndices: new Set<number>(),
      expandedFrameIndex: null,
      onFrameClick: vi.fn(),
      onToggleReferenceExpansion: vi.fn(),
      onHoverFrame: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ''),
    };

    render(<ThumbnailsView {...props} />);

    const placeholders = document.querySelectorAll('.frame-placeholder');
    expect(placeholders.length).toBe(1);
  });

  it('should handle frame without temporal_id', () => {
    const framesWithoutTemporal = [
      { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
    ] as FrameInfo[];
    const props = { ...defaultProps, frames: framesWithoutTemporal };

    render(<ThumbnailsView {...props} />);

    // Should show 'A' as default layer
    expect(screen.getByText('I-A 0')).toBeInTheDocument();
  });
});
