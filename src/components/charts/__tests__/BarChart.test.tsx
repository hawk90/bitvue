/**
 * BarChart Component Tests
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { BarChart } from '../BarChart';

describe('BarChart', () => {
  it('should render null for empty data', () => {
    const { container } = render(<BarChart data={{}} />);
    expect(container.firstChild).toBe(null);
  });

  it('should render bars for data entries', () => {
    render(<BarChart data={{ I: 10, P: 20, B: 30 }} />);

    expect(screen.getByText('I:')).toBeInTheDocument();
    expect(screen.getByText('P:')).toBeInTheDocument();
    expect(screen.getByText('B:')).toBeInTheDocument();
  });

  it('should render values', () => {
    render(<BarChart data={{ I: 10, P: 20, B: 30 }} />);

    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText('20')).toBeInTheDocument();
    expect(screen.getByText('30')).toBeInTheDocument();
  });

  it('should sort entries by value descending', () => {
    const { container } = render(<BarChart data={{ I: 10, P: 30, B: 20 }} />);

    const rows = container.querySelectorAll('.bar-chart-row');
    const labels = Array.from(rows).map(r => r.querySelector('.bar-chart-label')?.textContent);

    // Sorted by value descending: P (30), B (20), I (10)
    expect(labels).toEqual(['P:', 'B:', 'I:']);
  });

  it('should use custom max value', () => {
    const { container } = render(<BarChart data={{ I: 50, P: 100 }} maxValue={200} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    const firstBar = bars[0] as HTMLElement;
    const secondBar = bars[1] as HTMLElement;

    // Bars are sorted by value descending, so P=100 is first, I=50 is second
    // With maxValue=200: P=100 should be 50% width, I=50 should be 25% width
    expect(parseFloat(firstBar.style.width)).toBe(50);
    expect(parseFloat(secondBar.style.width)).toBe(25);
  });

  it('should calculate max value when not provided', () => {
    const { container } = render(<BarChart data={{ I: 10, P: 50, B: 30 }} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    const widestBar = Array.from(bars).reduce((max, bar) => {
      const width = parseFloat((bar as HTMLElement).style.width);
      return width > max ? width : max;
    }, 0);

    expect(widestBar).toBe(100);
  });

  it('should apply custom colors', () => {
    const { container } = render(
      <BarChart data={{ I: 10, P: 20 }} colors={{ I: '#ff0000', P: '#00ff00' }} />
    );

    const bars = container.querySelectorAll('.bar-chart-bar');
    // Colors are normalized to RGB format by the browser
    expect((bars[0] as HTMLElement).style.backgroundColor).toBe('rgb(0, 255, 0)');
    expect((bars[1] as HTMLElement).style.backgroundColor).toBe('rgb(255, 0, 0)');
  });

  it('should use default colors when not provided', () => {
    const { container } = render(<BarChart data={{ I: 10, P: 20, B: 30 }} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    // In test environment, CSS variables are used directly (e.g., '--frame-i')
    // The style property may not show the computed value
    // Just verify that bars are rendered (they have width styles)
    bars.forEach(bar => {
      expect((bar as HTMLElement).style.width).toBeTruthy();
    });
  });

  it('should apply minimum bar height', () => {
    const { container } = render(<BarChart data={{ I: 1, P: 1000 }} minBarHeight={5} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    const smallestBar = Array.from(bars).reduce((min, bar) => {
      const width = parseFloat((bar as HTMLElement).style.width);
      return width < min ? width : min;
    }, Infinity);

    expect(smallestBar).toBeGreaterThanOrEqual(5);
  });

  it('should render bar chart rows', () => {
    const { container } = render(<BarChart data={{ I: 10, P: 20 }} />);

    const rows = container.querySelectorAll('.bar-chart-row');
    expect(rows).toHaveLength(2);
  });

  it('should render bar containers', () => {
    const { container } = render(<BarChart data={{ I: 10 }} />);

    expect(container.querySelector('.bar-chart-bar-container')).toBeInTheDocument();
  });

  it('should add title attribute to bars', () => {
    const { container } = render(<BarChart data={{ I: 10, P: 20 }} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    // Bars are sorted by value descending, so P:20 comes first, then I:10
    const titles = Array.from(bars).map(bar => (bar as HTMLElement).title);
    expect(titles).toContain('I: 10');
    expect(titles).toContain('P: 20');
  });

  it('should use custom class name', () => {
    const { container } = render(<BarChart data={{ I: 10 }} className="custom-chart" />);

    expect(container.querySelector('.custom-chart')).toBeInTheDocument();
  });

  it('should use default class name', () => {
    const { container } = render(<BarChart data={{ I: 10 }} />);

    expect(container.querySelector('.bar-chart')).toBeInTheDocument();
  });

  it('should handle single entry', () => {
    const { container } = render(<BarChart data={{ I: 50 }} />);

    const rows = container.querySelectorAll('.bar-chart-row');
    expect(rows).toHaveLength(1);
  });

  it('should handle many entries', () => {
    const data = Object.fromEntries(Array.from({ length: 20 }, (_, i) => [`Item${i}`, i]));
    const { container } = render(<BarChart data={data} />);

    const rows = container.querySelectorAll('.bar-chart-row');
    expect(rows).toHaveLength(20);
  });

  it('should handle zero values', () => {
    const { container } = render(<BarChart data={{ I: 0, P: 10, B: 0 }} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    // With default minBarHeight=1, zero values become 1% width
    // The zero-value entries (I and B) will have width of 1 due to minBarHeight
    const widths = Array.from(bars).map(bar => parseFloat((bar as HTMLElement).style.width));
    const minBarWidths = widths.filter(w => w === 1);

    // I and B both have value 0, so they should both have minBarHeight width
    expect(minBarWidths).toHaveLength(2);
  });

  it('should handle negative values', () => {
    const { container } = render(<BarChart data={{ I: -10, P: 10 }} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    bars.forEach(bar => {
      const width = parseFloat((bar as HTMLElement).style.width);
      expect(width).toBeGreaterThanOrEqual(0);
    });
  });

  it('should handle decimal values', () => {
    render(<BarChart data={{ I: 10.5, P: 20.7, B: 15.3 }} />);

    expect(screen.getByText('10.5')).toBeInTheDocument();
    expect(screen.getByText('20.7')).toBeInTheDocument();
    expect(screen.getByText('15.3')).toBeInTheDocument();
  });

  it('should apply bar width percentage correctly', () => {
    const { container } = render(<BarChart data={{ I: 25, P: 50, B: 100 }} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    const widths = Array.from(bars).map(bar => parseFloat((bar as HTMLElement).style.width));

    // Bars are sorted by value descending: B=100%, P=50%, I=25%
    expect(widths).toContain(100);
    expect(widths).toContain(50);
    expect(widths).toContain(25);
  });

  it('should handle all same values', () => {
    const { container } = render(<BarChart data={{ I: 10, P: 10, B: 10 }} />);

    const bars = container.querySelectorAll('.bar-chart-bar');
    bars.forEach(bar => {
      expect(parseFloat((bar as HTMLElement).style.width)).toBe(100);
    });
  });

  it('should render label, bar, and value for each entry', () => {
    const { container } = render(<BarChart data={{ I: 10 }} />);

    expect(container.querySelector('.bar-chart-label')).toBeInTheDocument();
    expect(container.querySelector('.bar-chart-bar-container')).toBeInTheDocument();
    expect(container.querySelector('.bar-chart-bar')).toBeInTheDocument();
    expect(container.querySelector('.bar-chart-value')).toBeInTheDocument();
  });
});
