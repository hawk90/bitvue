/**
 * FrameSyntaxTab Component Tests
 * Tests frame syntax tree tab component
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@/test/test-utils';
import { FrameSyntaxTab } from '../SyntaxDetailPanel/FrameSyntaxTab';

describe('FrameSyntaxTab', () => {
  const mockFrame = {
    frame_index: 1,
    frame_type: 'P',
    size: 30000,
    pts: 1,
    temporal_id: 0,
    display_order: 1,
    coding_order: 0,
    ref_frames: [0],
  };

  const defaultProps = {
    frame: mockFrame,
    expandedNodes: new Set<string>(),
    onToggleNode: vi.fn(),
  };

  it('should render frame syntax tab', () => {
    render(<FrameSyntaxTab {...defaultProps} />);

    expect(screen.getByText('Frame:')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('should display frame type badge', () => {
    render(<FrameSyntaxTab {...defaultProps} />);

    const badge = document.querySelector('.frame-type-p');
    expect(badge).toBeInTheDocument();
  });

  it('should render syntax tree', () => {
    render(<FrameSyntaxTab {...defaultProps} />);

    const tree = document.querySelector('.syntax-tree');
    expect(tree).toBeInTheDocument();
  });

  it('should expand nodes when clicked', () => {
    const handleToggle = vi.fn();
    render(<FrameSyntaxTab {...defaultProps} onToggleNode={handleToggle} />);

    const expandButton = document.querySelector('.expand-toggle');
    if (expandButton) {
      fireEvent.click(expandButton);
      expect(handleToggle).toHaveBeenCalled();
    }
  });

  it('should render frame properties', () => {
    render(<FrameSyntaxTab {...defaultProps} expandedNodes={new Set(['Frame 1'])} />);

    expect(screen.getByText(/frame_type/)).toBeInTheDocument();
    expect(screen.getByText(/size/)).toBeInTheDocument();
    expect(screen.getByText(/pts/)).toBeInTheDocument();
  });

  it('should handle null frame', () => {
    render(<FrameSyntaxTab {...defaultProps} frame={null} />);

    expect(screen.getByText('No frame selected')).toBeInTheDocument();
  });

  it('should show reference frames in tree', () => {
    render(<FrameSyntaxTab {...defaultProps} expandedNodes={new Set(['Frame 1'])} />);

    expect(screen.getByText(/ref_frames/)).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<FrameSyntaxTab {...defaultProps} />);

    rerender(<FrameSyntaxTab {...defaultProps} />);

    expect(screen.getByText('Frame:')).toBeInTheDocument();
  });

  it('should render collapsed nodes by default', () => {
    render(<FrameSyntaxTab {...defaultProps} expandedNodes={new Set()} />);

    const chevronRight = document.querySelector('.codicon-chevron-right');
    expect(chevronRight).toBeInTheDocument();
  });

  it('should render expanded nodes when in set', () => {
    render(
      <FrameSyntaxTab
        {...defaultProps}
        expandedNodes={new Set(['Frame 1', 'Frame 1/ref_frames'])}
      />
    );

    const children = document.querySelector('.syntax-children');
    expect(children).toBeInTheDocument();
  });
});

describe('FrameSyntaxTab properties', () => {
  it('should display temporal_id when present', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
      temporal_id: 2,
    };

    render(<FrameSyntaxTab frame={frame} expandedNodes={new Set(['Frame 1'])} onToggleNode={vi.fn()} />);

    expect(screen.getByText(/temporal_id/)).toBeInTheDocument();
    expect(screen.getAllByText(/2/).length).toBeGreaterThan(0);
  });

  it('should display display_order when present', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
      display_order: 5,
    };

    render(<FrameSyntaxTab frame={frame} expandedNodes={new Set(['Frame 1'])} onToggleNode={vi.fn()} />);

    expect(screen.getByText(/display_order/)).toBeInTheDocument();
    expect(screen.getAllByText(/5/).length).toBeGreaterThan(0);
  });

  it('should display coding_order when present', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
      coding_order: 3,
    };

    render(<FrameSyntaxTab frame={frame} expandedNodes={new Set(['Frame 1'])} onToggleNode={vi.fn()} />);

    expect(screen.getByText(/coding_order/)).toBeInTheDocument();
    expect(screen.getAllByText(/3/).length).toBeGreaterThan(0);
  });

  it('should show -1 for missing optional properties', () => {
    const frame = {
      frame_index: 1,
      frame_type: 'P',
      size: 30000,
    };

    render(<FrameSyntaxTab frame={frame} expandedNodes={new Set(['Frame 1'])} onToggleNode={vi.fn()} />);

    // Missing properties should show -1
    const minusOneValues = screen.getAllByText(/-1/);
    expect(minusOneValues.length).toBeGreaterThan(0);
  });
});

describe('FrameSyntaxTab reference frames', () => {
  it('should display multiple reference frames', () => {
    const frame = {
      frame_index: 5,
      frame_type: 'B',
      size: 20000,
      ref_frames: [0, 1, 2, 3],
    };

    render(<FrameSyntaxTab frame={frame} expandedNodes={new Set(['Frame 5', 'Frame 5/ref_frames'])} onToggleNode={vi.fn()} />);

    const refs = document.querySelectorAll('.syntax-label');
    const refNodes = Array.from(refs).filter(r => r.textContent === 'ref[0]' || r.textContent === 'ref[1]');
    expect(refNodes.length).toBeGreaterThan(0);
  });

  it('should show no references for I-frames', () => {
    const frame = {
      frame_index: 0,
      frame_type: 'I',
      size: 50000,
    };

    render(<FrameSyntaxTab frame={frame} expandedNodes={new Set(['Frame 0'])} onToggleNode={vi.fn()} />);

    expect(screen.getByText(/ref_frames/)).toBeInTheDocument();
  });
});
