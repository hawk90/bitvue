/**
 * Tests for CodingFlowView component
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CodingFlowView } from '../CodingFlowView';

describe('CodingFlowView', () => {
  const mockFrame = {
    frame_index: 100,
    frame_type: 'P' as const,
    size: 25000,
  };

  it('renders without crashing', () => {
    render(
      <CodingFlowView
        frame={mockFrame}
        codec="hevc"
      />
    );
    expect(screen.getByText('Coding Flow')).toBeInTheDocument();
  });

  it('displays frame information', () => {
    render(
      <CodingFlowView
        frame={mockFrame}
        codec="hevc"
      />
    );

    expect(screen.getByText('Frame 100')).toBeInTheDocument();
    expect(screen.getByText('P')).toBeInTheDocument();
  });

  it('displays codec-specific features for HEVC', () => {
    render(
      <CodingFlowView
        frame={mockFrame}
        codec="hevc"
      />
    );

    expect(screen.getByText('35 Intra Modes')).toBeInTheDocument();
    expect(screen.getByText('Advanced Motion Vector Pred')).toBeInTheDocument();
  });

  it('displays codec-specific features for AV1', () => {
    render(
      <CodingFlowView
        frame={mockFrame}
        codec="av1"
      />
    );

    expect(screen.getByText('Directional Intra Pred')).toBeInTheDocument();
    expect(screen.getByText('Compound Prediction')).toBeInTheDocument();
  });

  it('displays codec-specific features for VVC', () => {
    render(
      <CodingFlowView
        frame={mockFrame}
        codec="vvc"
      />
    );

    expect(screen.getByText('67 Intra Modes')).toBeInTheDocument();
    expect(screen.getByText('GPM/Combine Pred')).toBeInTheDocument();
  });

  it('renders all pipeline stages', () => {
    render(
      <CodingFlowView
        frame={mockFrame}
        codec="hevc"
      />
    );

    expect(screen.getByText('Input')).toBeInTheDocument();
    expect(screen.getByText('Prediction')).toBeInTheDocument();
    expect(screen.getByText('Transform')).toBeInTheDocument();
    expect(screen.getByText('Quantization')).toBeInTheDocument();
    expect(screen.getByText('Entropy Coding')).toBeInTheDocument();
  });

  it('renders legend', () => {
    render(
      <CodingFlowView
        frame={mockFrame}
        codec="hevc"
      />
    );

    expect(screen.getByText('Current Stage')).toBeInTheDocument();
    expect(screen.getByText('Completed Flow')).toBeInTheDocument();
    expect(screen.getByText('Encoder Only')).toBeInTheDocument();
    expect(screen.getByText('Decoder Path')).toBeInTheDocument();
  });

  it('handles null frame gracefully', () => {
    const { container } = render(
      <CodingFlowView
        frame={null}
        codec="hevc"
      />
    );

    // Should render without crashing
    expect(container).toBeInTheDocument();
  });
});
