/**
 * Tests for ExportDialog component
 */

import { describe, it, expect, vi, fireEvent } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { ExportDialog } from '@/components/ExportDialog';
import type { FrameInfo } from '@/types/video';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(() => Promise.resolve('/mock/path/export.csv')),
}));

describe('ExportDialog', () => {
  const mockFrames: FrameInfo[] = [
    {
      frameNumber: 0,
      frameType: 'I',
      poc: 0,
      pts: 0,
      size: 50000,
      qp: 26,
    },
    {
      frameNumber: 1,
      frameType: 'P',
      poc: 1,
      pts: 1,
      size: 25000,
      qp: 28,
      refFrames: [0],
    },
    {
      frameNumber: 2,
      frameType: 'B',
      poc: 2,
      pts: 2,
      size: 15000,
      qp: 30,
      refFrames: [0, 1],
    },
  ];

  const mockOnClose = vi.fn();

  it('renders when isOpen is true', () => {
    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    expect(screen.getByText('Export Data')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Export' })).toBeInTheDocument();
  });

  it('does not render when isOpen is false', () => {
    const { container } = render(
      <ExportDialog
        isOpen={false}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    expect(container).toBeEmpty();
  });

  it('displays export type options', () => {
    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    expect(screen.getByText('Frame Data')).toBeInTheDocument();
    expect(screen.getByText('Analysis Report')).toBeInTheDocument();
    expect(screen.getByText('Full Report')).toBeInTheDocument();
  });

  it('displays format options', () => {
    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    expect(screen.getByText('CSV')).toBeInTheDocument();
    expect(screen.getByText('JSON')).toBeInTheDocument();
    expect(screen.getByText('Text')).toBeInTheDocument();
    expect(screen.getByText('PDF')).toBeInTheDocument();
  });

  it('shows frame count info', () => {
    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    expect(screen.getByText(/Frames to export: 3/)).toBeInTheDocument();
  });

  it('disables export button when no frames', () => {
    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={[]}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    const exportButton = screen.getByRole('button', { name: 'Export' });
    expect(exportButton).toBeDisabled();
  });

  it('calls onClose when cancel is clicked', async () => {
    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    const cancelButton = screen.getByRole('button', { name: 'Cancel' });
    fireEvent.click(cancelButton);

    await waitFor(() => {
      expect(mockOnClose).toHaveBeenCalled();
    });
  });

  it('calls export when export button is clicked', async () => {
    const { invoke } = require('@tauri-apps/api/core');
    invoke.mockResolvedValue('Export successful: /mock/path/export.csv');

    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    const exportButton = screen.getByRole('button', { name: 'Export' });
    fireEvent.click(exportButton);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalled();
    });
  });

  it('shows export status messages', async () => {
    const { invoke } = require('@tauri-apps/api/core');
    invoke.mockResolvedValue('Export successful: /mock/path/export.csv');

    render(
      <ExportDialog
        isOpen={true}
        onClose={mockOnClose}
        frames={mockFrames}
        codec="hevc"
        width={1920}
        height={1080}
      />
    );

    const exportButton = screen.getByRole('button', { name: 'Export' });
    fireEvent.click(exportButton);

    await waitFor(() => {
      expect(screen.getByText(/Exporting\.\.\./)).toBeInTheDocument();
      expect(screen.getByText(/Export success/i)).toBeInTheDocument();
    });
  });
});
