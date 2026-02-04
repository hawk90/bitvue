/**
 * Tests for ExportDialog component
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ExportDialog } from '@/components/ExportDialog';
import type { FrameInfo } from '@/types/video';

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

const mockSave = vi.fn();
vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: mockSave,
}));

// Mock exportUtils - must be defined inline due to hoisting
vi.mock('@/utils/exportUtils', () => ({
  exportUtils: {
    exportFramesToCsv: vi.fn(() => Promise.resolve('/mock/path/export.csv')),
    exportFramesToJson: vi.fn(() => Promise.resolve('/mock/path/export.json')),
    exportAnalysisReport: vi.fn(() => Promise.resolve('/mock/path/report.txt')),
    generateAnalysisReport: vi.fn(() => ({
      codec: 'hevc',
      width: 1920,
      height: 1080,
      total_frames: 3,
      frame_type_distribution: {
        i_frames: 1,
        p_frames: 1,
        b_frames: 1,
      },
      size_statistics: {
        total: 90000,
        average: 30000,
        max: 50000,
        min: 15000,
      },
      gop_structure: {
        count: 1,
        average_size: 3,
      },
    })),
    exportToPdf: vi.fn(() => Promise.resolve()),
  },
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

  let mockOnClose: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockOnClose = vi.fn();
    vi.clearAllMocks();
    mockSave.mockResolvedValue('/mock/path/export.csv');
  });

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

    expect(container.firstChild).toBeNull();
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

    // CSV and JSON are always available
    expect(screen.getByText('CSV')).toBeInTheDocument();
    expect(screen.getByText('JSON')).toBeInTheDocument();

    // Text and PDF are only shown for analysis/report types
    expect(screen.queryByText('Text')).not.toBeInTheDocument();
    expect(screen.queryByText('PDF')).not.toBeInTheDocument();
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

  it('displays Text and PDF format options for analysis type', async () => {
    const user = userEvent.setup();
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

    // Click on Analysis Report radio button
    const analysisRadio = screen.getByLabelText('Analysis Report');
    await user.click(analysisRadio);

    // Now Text and PDF should be visible
    expect(screen.getByText('Text')).toBeInTheDocument();
    expect(screen.getByText('PDF')).toBeInTheDocument();
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
    const user = userEvent.setup();
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
    await user.click(cancelButton);

    expect(mockOnClose).toHaveBeenCalled();
  });

  it('calls export when export button is clicked', async () => {
    const user = userEvent.setup();
    const { exportUtils } = await import('@/utils/exportUtils');

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
    await user.click(exportButton);

    await waitFor(() => {
      expect(exportUtils.exportFramesToCsv).toHaveBeenCalled();
    }, { timeout: 3000 });
  });

  it('shows export status messages', async () => {
    const user = userEvent.setup();

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
    await user.click(exportButton);

    // The export completes very fast in tests, so we just check for success
    expect(await screen.findByText(/Export success/i)).toBeInTheDocument();
  });
});
