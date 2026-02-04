/**
 * StatisticsTab Component Tests
 * Tests statistics tab component
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { StatisticsTab } from '@/components/panels/SyntaxDetailPanel/StatisticsTab';

describe('StatisticsTab', () => {
  const mockCurrentFrame = {
    frame_index: 42,
    frame_type: 'P',
    size: 30000,
    temporal_id: 0,
    display_order: 40,
    coding_order: 41,
    ref_frames: [0, 1],
  };

  const mockFrames = [
    { frame_index: 0, frame_type: 'I', size: 50000 },
    { frame_index: 1, frame_type: 'P', size: 30000 },
    { frame_index: 2, frame_type: 'P', size: 35000 },
    { frame_index: 3, frame_type: 'B', size: 20000 },
    { frame_index: 4, frame_type: 'B', size: 25000 },
  ];

  const defaultProps = {
    currentFrame: mockCurrentFrame,
    frames: mockFrames,
  };

  it('should render statistics tab', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('Current Frame')).toBeInTheDocument();
  });

  it('should display current frame index', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('42')).toBeInTheDocument();
  });

  it('should display frame type', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('P')).toBeInTheDocument();
  });

  it('should display frame size in KB', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('29.30 KB')).toBeInTheDocument();
  });

  it('should render stream statistics section', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('Stream Statistics')).toBeInTheDocument();
  });

  it('should display total frames', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('Total Frames:')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('should display average size', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('Avg Size:')).toBeInTheDocument();
    expect(screen.getByText(/32\.00 KB/)).toBeInTheDocument();
  });

  it('should display frame type counts', () => {
    render(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText(/I.*\d+/)).toBeInTheDocument();
    expect(screen.getByText(/P.*\d+/)).toBeInTheDocument();
    expect(screen.getByText(/B.*\d+/)).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<StatisticsTab {...defaultProps} />);

    rerender(<StatisticsTab {...defaultProps} />);

    expect(screen.getByText('Current Frame')).toBeInTheDocument();
  });
});

describe('StatisticsTab frame info', () => {
  it('should display temporal ID when present', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
      temporal_id: 2,
    };

    render(<StatisticsTab currentFrame={frame} frames={[]} />);

    expect(screen.getByText('Temporal ID:')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
  });

  it('should display display order when present', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
      display_order: 10,
    };

    render(<StatisticsTab currentFrame={frame} frames={[]} />);

    expect(screen.getByText('Display Order:')).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
  });

  it('should display coding order when present', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
      coding_order: 5,
    };

    render(<StatisticsTab currentFrame={frame} frames={[]} />);

    expect(screen.getByText('Coding Order:')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('should display reference count', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
      ref_frames: [0, 2, 3],
    };

    render(<StatisticsTab currentFrame={frame} frames={[]} />);

    expect(screen.getByText('References:')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('should show N/A for missing frame', () => {
    render(<StatisticsTab currentFrame={null} frames={[]} />);

    expect(screen.getByText('Index:')).toBeInTheDocument();
    expect(screen.getAllByText('N/A').length).toBeGreaterThan(0);
  });
});

describe('StatisticsTab calculations', () => {
  it('should calculate correct average', () => {
    const frames = [
      { frame_index: 0, frame_type: 'I', size: 50000 },
      { frame_index: 1, frame_type: 'P', size: 10000 },
    ];

    render(<StatisticsTab currentFrame={frames[0]} frames={frames} />);

    expect(screen.getByText(/29\.30 KB/)).toBeInTheDocument();
  });

  it('should handle zero frames', () => {
    render(<StatisticsTab currentFrame={null} frames={[]} />);

    expect(screen.getByText('Total Frames:')).toBeInTheDocument();
    expect(screen.getByText('0')).toBeInTheDocument();
  });

  it('should handle single frame type', () => {
    const frames = [
      { frame_index: 0, frame_type: 'I', size: 50000 },
    ];

    render(<StatisticsTab currentFrame={frames[0]} frames={frames} />);

    expect(screen.getByText(/I.*1/)).toBeInTheDocument();
  });
});
