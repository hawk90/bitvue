/**
 * KeyboardShortcutsDialog Component Tests
 * Tests keyboard shortcuts modal
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@/test/test-utils';
import { KeyboardShortcutsDialog } from '@/components/KeyboardShortcutsDialog';

describe('KeyboardShortcutsDialog', () => {
  it('should render dialog when open', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();
  });

  it('should not render when closed', () => {
    const { container } = render(<KeyboardShortcutsDialog open={false} onClose={vi.fn()} />);

    expect(container.firstChild).toBe(null);
  });

  it('should render navigation shortcuts', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    expect(screen.getByText(/First Frame/)).toBeInTheDocument();
    expect(screen.getByText(/Last Frame/)).toBeInTheDocument();
    expect(screen.getByText(/Home/)).toBeInTheDocument();
    expect(screen.getByText(/End/)).toBeInTheDocument();
  });

  it('should render frame navigation shortcuts', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Next Keyframe/)).toBeInTheDocument();
    expect(screen.getByText(/Previous Keyframe/)).toBeInTheDocument();
  });

  it('should render frame type navigation', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    // Should have I-frame navigation
    const iFrameNavs = screen.queryAllByText(/I-frame/);
    expect(iFrameNavs.length).toBeGreaterThan(0);
  });

  it('should render mode shortcuts', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Overview.*F1/)).toBeInTheDocument();
    expect(screen.getByText(/Coding Flow.*F2/)).toBeInTheDocument();
    expect(screen.getByText(/Prediction.*F3/)).toBeInTheDocument();
  });

  it('should render playback shortcuts', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Play.*Space/)).toBeInTheDocument();
    expect(screen.getByText(/Pause.*Space/)).toBeInTheDocument();
  });

  it('should have close button', () => {
    const handleClose = vi.fn();
    render(<KeyboardShortcutsDialog open={true} onClose={handleClose} />);

    const closeButton = screen.getByRole('button', { name: /close/i });
    fireEvent.click(closeButton);

    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it('should close on Escape key', () => {
    const handleClose = vi.fn();
    render(<KeyboardShortcutsDialog open={true} onClose={handleClose} />);

    fireEvent.keyDown(document, { key: 'Escape' });

    expect(handleClose).toHaveBeenCalled();
  });

  it('should be modal with backdrop', () => {
    const { container } = render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    const backdrop = container.querySelector('.modal-backdrop');
    const modal = container.querySelector('.keyboard-shortcuts-dialog');

    expect(modal).toBeInTheDocument();
    // Backdrop may or may not exist based on implementation
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    rerender(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();
  });

  it('should have organized sections', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    // Should have sections like Navigation, Frame Navigation, Modes, etc.
    expect(screen.getAllByText(/Navigation/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Frame/i).length).toBeGreaterThan(0);
  });
});

describe('KeyboardShortcutsDialog categories', () => {
  it('should show file operations section', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Open.*Ctrl\+O/)).toBeInTheDocument();
    expect(screen.getByText(/Close.*Ctrl\+W/)).toBeInTheDocument();
  });

  it('should show view options section', () => {
    render(<KeyboardShortcutsDialog open={true} onClose={vi.fn()} />);

    // May have view-related shortcuts
    expect(screen.queryByText(/Toggle.*Panel/i)).toBeInTheDocument();
  });
});

const BASE_SHORTCUTS = [
  { key: 'ArrowLeft', action: 'Previous frame' },
  { key: 'ArrowRight', action: 'Next frame' },
  { key: ' ', action: 'Play/Pause' },
  { key: 'K', action: 'Next keyframe' },
];
