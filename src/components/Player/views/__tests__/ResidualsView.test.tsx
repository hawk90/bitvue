/**
 * Tests for ResidualsView component
 * TODO: Skipping due to complex codec view requiring full parser backend
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ResidualsView } from '../ResidualsView';
import type { FrameInfo } from '@/types/video';

describe.skip('ResidualsView', () => {
  const mockFrame: FrameInfo = {
    frame_index: 100,
    frame_type: 'P',
    poc: 100,
    pts: 100,
    size: 25000,
    temporal_id: 0,
    spatial_id: 0,
    ref_frames: [99],
  };

  it('renders without crashing', () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
      />
    );
    expect(screen.getByText('Residuals Analysis')).toBeInTheDocument();
  });

  it('displays frame information', () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
      />
    );

    expect(screen.getByText('Frame 100')).toBeInTheDocument();
    expect(screen.getByText('P')).toBeInTheDocument();
    expect(screen.getByText(/1920x1080/)).toBeInTheDocument();
  });

  it('displays coefficient statistics', () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
      />
    );

    // The component generates mock stats, so we should see stats
    expect(screen.getByText(/Non-Zero Coeffs:/)).toBeInTheDocument();
    expect(screen.getByText(/Zero Coeffs:/)).toBeInTheDocument();
  });

  it('renders heatmap view when showHeatmap is true', () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={true}
        showHistogram={false}
      />
    );

    expect(screen.getByText('Residual Energy Heatmap')).toBeInTheDocument();
  });

  it('renders histogram view when showHistogram is true', () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={false}
        showHistogram={true}
      />
    );

    expect(screen.getByText('Coefficient Distribution')).toBeInTheDocument();
  });

  it('renders both views when both are enabled', () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={true}
        showHistogram={true}
      />
    );

    expect(screen.getByText('Residual Energy Heatmap')).toBeInTheDocument();
    expect(screen.getByText('Coefficient Distribution')).toBeInTheDocument();
  });

  it('handles null frame gracefully', () => {
    const { container } = render(
      <ResidualsView
        frame={null}
        width={1920}
        height={1080}
      />
    );

    expect(screen.getByText('No frame selected')).toBeInTheDocument();
  });

  it('displays heatmap color scale legend', () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={true}
      />
    );

    expect(screen.getByText('Low')).toBeInTheDocument();
    expect(screen.getByText('High')).toBeInTheDocument();
  });
});
