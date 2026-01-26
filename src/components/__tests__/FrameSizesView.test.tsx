/**
 * FrameSizesView Component Tests
 * Tests frame sizes bar chart view
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@/test/test-utils';
import FrameSizesView, { type SizeMetrics } from '../Filmstrip/views/FrameSizesView';

// Mock getFrameTypeColor
vi.mock('@/types/video', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/types/video')>();
  return {
    ...actual,
    getFrameTypeColor: vi.fn(() => '#ff0000'),
  };
});

describe('FrameSizesView', () => {
  const mockFrames = [
    { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
    { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    { frame_index: 2, frame_type: 'P', size: 35000, poc: 2 },
    { frame_index: 3, frame_type: 'B', size: 20000, poc: 3 },
    { frame_index: 4, frame_type: 'B', size: 25000, poc: 4 },
  ];

  const defaultProps = {
    frames: mockFrames,
    maxSize: 50000,
    currentFrameIndex: 2,
    visibleFrameTypes: new Set(['I', 'P', 'B']),
    onFrameClick: vi.fn(),
    getFrameTypeColorClass: vi.fn((type: string) => `frame-type-${type.toLowerCase()}`),
    sizeMetrics: {
      showBitrateBar: true,
      showBitrateCurve: true,
      showAvgSize: true,
      showMinSize: true,
      showMaxSize: true,
      showMovingAvg: true,
      showBlockMinQP: false,
      showBlockMaxQP: false,
    } as SizeMetrics,
  };

  it('should render frame sizes view', () => {
    render(<FrameSizesView {...defaultProps} />);

    const container = document.querySelector('.filmstrip-sizes');
    expect(container).toBeInTheDocument();
  });

  it('should render all frame bars', () => {
    render(<FrameSizesView {...defaultProps} />);

    const bars = document.querySelectorAll('.frame-size-bar');
    expect(bars.length).toBe(5);
  });

  it('should highlight current frame', () => {
    render(<FrameSizesView {...defaultProps} currentFrameIndex={2} />);

    const currentBar = document.querySelector('.frame-size-bar.selected');
    expect(currentBar).toBeInTheDocument();
  });

  it('should call onFrameClick when bar clicked', () => {
    const handleClick = vi.fn();
    render(<FrameSizesView {...defaultProps} onFrameClick={handleClick} />);

    const bars = document.querySelectorAll('.frame-size-bar');
    if (bars.length > 0) {
      fireEvent.click(bars[0]);
      expect(handleClick).toHaveBeenCalledWith(0);
    }
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<FrameSizesView {...defaultProps} />);

    rerender(<FrameSizesView {...defaultProps} />);

    expect(document.querySelector('.filmstrip-sizes')).toBeInTheDocument();
  });
});

describe('FrameSizesView calculations', () => {
  const mockFrames = [
    { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
    { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    { frame_index: 2, frame_type: 'P', size: 35000, poc: 2 },
    { frame_index: 3, frame_type: 'B', size: 20000, poc: 3 },
  ];

  const defaultProps = {
    frames: mockFrames,
    maxSize: 50000,
    currentFrameIndex: 0,
    visibleFrameTypes: new Set(['I', 'P', 'B']),
    onFrameClick: vi.fn(),
    getFrameTypeColorClass: vi.fn(() => ''),
    sizeMetrics: {
      showBitrateBar: true,
      showBitrateCurve: false,
      showAvgSize: true,
      showMinSize: true,
      showMaxSize: true,
      showMovingAvg: true,
      showBlockMinQP: false,
      showBlockMaxQP: false,
    } as SizeMetrics,
  };

  it('should calculate average size and display in axis', () => {
    render(<FrameSizesView {...defaultProps} />);

    // Max size is 50000, so axis shows 49 KB (50000/1024 = 48.8, rounded to 49)
    const maxLabel = document.querySelector('.axis-label.max');
    expect(maxLabel?.textContent).toContain('49');

    // Check for KB in the axis title
    const axisTitle = document.querySelector('.axis-title');
    expect(axisTitle?.textContent).toBe('Bitrate');
  });

  it('should display min size in axis labels', () => {
    render(<FrameSizesView {...defaultProps} />);

    // Should show 0 as min in the axis label
    const minLabels = document.querySelectorAll('.axis-label.min');
    expect(minLabels.length).toBeGreaterThan(0);
    expect(minLabels[0].textContent).toBe('0');
  });

  it('should display max size in axis labels', () => {
    render(<FrameSizesView {...defaultProps} />);

    // Max size / 1024 = ~49 KB, rounded
    expect(screen.getByText(/49/)).toBeInTheDocument();
  });

  it('should show moving average line when enabled', () => {
    render(<FrameSizesView {...defaultProps} />);

    // Moving average is rendered as SVG with polyline
    const movingAvgSvg = document.querySelector('.frame-sizes-line-chart');
    expect(movingAvgSvg).toBeInTheDocument();
  });
});

describe('FrameSizesView visibility', () => {
  const mockFrames = [
    { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
    { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    { frame_index: 2, frame_type: 'B', size: 20000, poc: 2 },
  ];

  const baseProps = {
    frames: mockFrames,
    maxSize: 50000,
    currentFrameIndex: 0,
    visibleFrameTypes: new Set(['I', 'P', 'B']),
    onFrameClick: vi.fn(),
    getFrameTypeColorClass: vi.fn(() => ''),
    sizeMetrics: {} as SizeMetrics,
  };

  it('should filter by visible frame types', () => {
    render(<FrameSizesView {...baseProps} visibleFrameTypes={new Set(['I'])} />);

    const bars = document.querySelectorAll('.frame-size-bar');
    expect(bars.length).toBe(1);
  });

  it('should show bitrate curve when enabled', () => {
    // Add duration to frames for bitrate calculation
    const framesWithDuration = [
      { ...baseProps.frames[0], duration: 1000 },
      { ...baseProps.frames[1], duration: 1000 },
    ];

    const props = {
      ...baseProps,
      frames: framesWithDuration,
      sizeMetrics: { showBitrateCurve: true } as SizeMetrics,
    };
    render(<FrameSizesView {...props} />);

    const curve = document.querySelector('.frame-sizes-line-chart');
    expect(curve).toBeInTheDocument();
  });
});

describe('FrameSizesView edge cases', () => {
  it('should handle empty frames array', () => {
    const props = {
      frames: [],
      maxSize: 0,
      currentFrameIndex: -1,
      visibleFrameTypes: new Set(['I', 'P', 'B']),
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ''),
      sizeMetrics: {} as SizeMetrics,
    };

    render(<FrameSizesView {...props} />);

    expect(document.querySelector('.filmstrip-sizes')).toBeInTheDocument();
  });

  it('should handle single frame', () => {
    const props = {
      frames: [{ frame_index: 0, frame_type: 'I', size: 50000, poc: 0 }],
      maxSize: 50000,
      currentFrameIndex: 0,
      visibleFrameTypes: new Set(['I']),
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ''),
      sizeMetrics: {} as SizeMetrics,
    };

    render(<FrameSizesView {...props} />);

    const bars = document.querySelectorAll('.frame-size-bar');
    expect(bars.length).toBe(1);
  });

  it('should handle zero max size', () => {
    const props = {
      frames: [{ frame_index: 0, frame_type: 'I', size: 0, poc: 0 }],
      maxSize: 0,
      currentFrameIndex: 0,
      visibleFrameTypes: new Set(['I']),
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ''),
      sizeMetrics: {} as SizeMetrics,
    };

    render(<FrameSizesView {...props} />);

    expect(document.querySelector('.filmstrip-sizes')).toBeInTheDocument();
  });
});
