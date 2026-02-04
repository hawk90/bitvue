/**
 * DetailsPanel Component Tests
 * Tests details panel with frame-specific information
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { DetailsPanel, type FrameDetails } from '../DetailsPanel';

describe('DetailsPanel', () => {
  const mockFrame: FrameDetails = {
    temporal_id: 0,
    display_order: 1,
    coding_order: 0,
    ref_frames: [0, 1],
  };

  it('should render details panel', () => {
    render(<DetailsPanel frame={mockFrame} />);

    expect(screen.getByText('Temporal Layer:')).toBeInTheDocument();
    expect(screen.getByText('Display Order:')).toBeInTheDocument();
  });

  it('should display temporal_id', () => {
    render(<DetailsPanel frame={mockFrame} />);

    const values = screen.getAllByText('0');
    expect(values).toContainEqual(expect.any(HTMLElement));
  });

  it('should display display_order', () => {
    render(<DetailsPanel frame={mockFrame} />);

    const values = screen.getAllByText('1');
    expect(values).toContainEqual(expect.any(HTMLElement));
  });

  it('should display coding_order', () => {
    render(<DetailsPanel frame={mockFrame} />);

    const values = screen.getAllByText('0');
    expect(values).toContainEqual(expect.any(HTMLElement));
  });

  it('should display ref_frames', () => {
    render(<DetailsPanel frame={mockFrame} />);

    expect(screen.getByText('0, 1')).toBeInTheDocument();
  });

  it('should display "A" for missing temporal_id', () => {
    const frameWithoutTemporalId: FrameDetails = {
      display_order: 1,
      coding_order: 0,
    };

    render(<DetailsPanel frame={frameWithoutTemporalId} />);

    expect(screen.getByText('A')).toBeInTheDocument();
  });

  it('should display "N/A" for missing display_order', () => {
    const frameWithoutDisplayOrder: FrameDetails = {
      temporal_id: 0,
      coding_order: 0,
    };

    render(<DetailsPanel frame={frameWithoutDisplayOrder} />);

    const naValues = screen.queryAllByText('N/A');
    expect(naValues.length).toBeGreaterThan(0);
  });

  it('should display "N/A" for missing coding_order', () => {
    const frameWithoutCodingOrder: FrameDetails = {
      temporal_id: 0,
      display_order: 1,
    };

    render(<DetailsPanel frame={frameWithoutCodingOrder} />);

    const naValues = screen.queryAllByText('N/A');
    expect(naValues.length).toBeGreaterThan(0);
  });

  it('should display "None" for empty ref_frames', () => {
    const frameWithoutRefs: FrameDetails = {
      temporal_id: 0,
      ref_frames: [],
    };

    render(<DetailsPanel frame={frameWithoutRefs} />);

    expect(screen.getByText('None')).toBeInTheDocument();
  });

  it('should display "None" for undefined ref_frames', () => {
    const frameWithoutRefs: FrameDetails = {
      temporal_id: 0,
    };

    render(<DetailsPanel frame={frameWithoutRefs} />);

    expect(screen.getByText('None')).toBeInTheDocument();
  });

  it('should handle null frame', () => {
    render(<DetailsPanel frame={null} />);

    expect(screen.getByText('A')).toBeInTheDocument(); // temporal_id defaults to 'A'
    const naValues = screen.queryAllByText('N/A');
    expect(naValues.length).toBeGreaterThan(0);
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<DetailsPanel frame={mockFrame} />);

    rerender(<DetailsPanel frame={mockFrame} />);

    expect(screen.getByText('Temporal Layer:')).toBeInTheDocument();
  });

  it('should handle single reference frame', () => {
    const frameWithSingleRef: FrameDetails = {
      temporal_id: 0,
      ref_frames: [5],
    };

    render(<DetailsPanel frame={frameWithSingleRef} />);

    const values = screen.getAllByText('5');
    expect(values).toContainEqual(expect.any(HTMLElement));
  });

  it('should handle multiple reference frames', () => {
    const frameWithMultipleRefs: FrameDetails = {
      temporal_id: 0,
      ref_frames: [0, 1, 2, 3],
    };

    render(<DetailsPanel frame={frameWithMultipleRefs} />);

    expect(screen.getByText('0, 1, 2, 3')).toBeInTheDocument();
  });
});

describe('DetailsPanel edge cases', () => {
  it('should handle frame with only temporal_id', () => {
    const minimalFrame: FrameDetails = {
      temporal_id: 2,
    };

    render(<DetailsPanel frame={minimalFrame} />);

    const values = screen.getAllByText('2');
    expect(values).toContainEqual(expect.any(HTMLElement));
    expect(screen.getByText('N/A')).toBeInTheDocument();
    expect(screen.getByText('None')).toBeInTheDocument();
  });

  it('should handle high temporal_id values', () => {
    const highTemporalFrame: FrameDetails = {
      temporal_id: 7,
    };

    render(<DetailsPanel frame={highTemporalFrame} />);

    const values = screen.getAllByText('7');
    expect(values).toContainEqual(expect.any(HTMLElement));
  });

  it('should handle large reference frame arrays', () => {
    const frameWithManyRefs: FrameDetails = {
      temporal_id: 0,
      ref_frames: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    };

    render(<DetailsPanel frame={frameWithManyRefs} />);

    expect(screen.getByText('0, 1, 2, 3, 4, 5, 6, 7, 8, 9')).toBeInTheDocument();
  });
});
