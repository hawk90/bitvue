/**
 * LineChart Component Tests
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LineChart } from '../LineChart';

describe('LineChart', () => {
  it('renders line chart with data', () => {
    const series = [{
      name: 'Test Series',
      data: [
        { x: 0, y: 10 },
        { x: 100, y: 20 },
        { x: 200, y: 15 },
      ],
    }];

    const { container } = render(
      <LineChart series={series} height={300} />
    );

    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
  });

  it('shows no data message when empty', () => {
    const { container } = render(
      <LineChart series={[]} height={300} />
    );

    expect(screen.getByText(/no data to display/i)).toBeInTheDocument();
  });

  it('displays axis labels', () => {
    const series = [{
      name: 'Test',
      data: [{ x: 0, y: 0 }],
    }];

    render(
      <LineChart
        series={series}
        xAxisLabel="Bitrate"
        yAxisLabel="Quality"
        height={300}
      />
    );

    expect(screen.getByText('Bitrate')).toBeInTheDocument();
    expect(screen.getByText('Quality')).toBeInTheDocument();
  });

  it('displays legend when enabled', () => {
    const series = [{
      name: 'HEVC',
      data: [{ x: 0, y: 0 }],
    }];

    render(
      <LineChart series={series} showLegend={true} height={300} />
    );

    expect(screen.getByText('HEVC')).toBeInTheDocument();
  });

  it('hides legend when disabled', () => {
    const series = [{
      name: 'HEVC',
      data: [{ x: 0, y: 0 }],
    }];

    const { container } = render(
      <LineChart series={series} showLegend={false} height={300} />
    );

    expect(screen.queryByText('HEVC')).not.toBeInTheDocument();
  });

  it('renders multiple series', () => {
    const series = [
      { name: 'AVC', data: [{ x: 0, y: 10 }] },
      { name: 'HEVC', data: [{ x: 0, y: 12 }] },
      { name: 'VVC', data: [{ x: 0, y: 14 }] },
    ];

    const { container } = render(
      <LineChart series={series} height={300} />
    );

    expect(screen.getByText('AVC')).toBeInTheDocument();
    expect(screen.getByText('HEVC')).toBeInTheDocument();
    expect(screen.getByText('VVC')).toBeInTheDocument();
  });

  it('displays title when provided', () => {
    const series = [{
      name: 'Test',
      data: [{ x: 0, y: 0 }],
    }];

    render(
      <LineChart series={series} title="RD Curves" height={300} />
    );

    expect(screen.getByText('RD Curves')).toBeInTheDocument();
  });
});
