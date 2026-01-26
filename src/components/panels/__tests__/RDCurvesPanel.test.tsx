/**
 * RDCurvesPanel Component Tests
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@/test/test-utils';
import userEvent from '@testing-library/user-event';
import { RDCurvesPanel } from '../RDCurvesPanel';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('RDCurvesPanel', () => {
  it('renders rd curves panel', () => {
    render(<RDCurvesPanel />);

    expect(screen.getByText('Rate-Distortion Curves')).toBeInTheDocument();
  });

  it('shows no data message initially', () => {
    render(<RDCurvesPanel />);

    expect(screen.getByText(/no data loaded/i)).toBeInTheDocument();
  });

  it('loads demo data when button clicked', async () => {
    const user = userEvent.setup();
    render(<RDCurvesPanel />);

    const button = screen.getByRole('button', { name: /load demo data/i });
    await user.click(button);

    await waitFor(() => {
      expect(screen.queryByText(/no data loaded/i)).not.toBeInTheDocument();
    });
  });

  it('disables bd-rate calculate button when no data', () => {
    render(<RDCurvesPanel />);

    const button = screen.getByRole('button', { name: /calculate bd-rate/i });
    expect(button).toBeDisabled();
  });

  it('enables bd-rate calculate button after loading data', async () => {
    const user = userEvent.setup();
    render(<RDCurvesPanel />);

    await user.click(screen.getByRole('button', { name: /load demo data/i }));

    await waitFor(() => {
      const button = screen.getByRole('button', { name: /calculate bd-rate/i });
      expect(button).not.toBeDisabled();
    });
  });

  it('shows metric selection buttons', () => {
    render(<RDCurvesPanel />);

    expect(screen.getByText('PSNR')).toBeInTheDocument();
    expect(screen.getByText('SSIM')).toBeInTheDocument();
    expect(screen.getByText('VMAF')).toBeInTheDocument();
  });

  it('switches quality metric', async () => {
    const user = userEvent.setup();
    render(<RDCurvesPanel />);

    await user.click(screen.getByRole('button', { name: 'SSIM' }));
    expect(screen.getByRole('button', { name: 'SSIM' })).toHaveClass('bg-blue-500');
  });

  it('calculates bd-rate after loading data', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue({
      anchor_name: 'H.264/AVC',
      test_name: 'H.265/HEVC',
      bd_rate: -45.8,
      bd_psnr: 1.2,
      interpretation: 'HEVC achieves similar quality at 45.8% lower bitrate than AVC',
    });

    const user = userEvent.setup();
    render(<RDCurvesPanel />);

    await user.click(screen.getByRole('button', { name: /load demo data/i }));

    const calculateButton = screen.getByRole('button', { name: /calculate bd-rate/i });
    await user.click(calculateButton);

    // Wait for calculation to complete and results to appear
    await waitFor(() => {
      expect(screen.getByText(/BD-Rate Calculation Results/i)).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  it('displays bd-rate results', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue({
      anchor_name: 'H.264/AVC',
      test_name: 'H.265/HEVC',
      bd_rate: -45.8,
      bd_psnr: 1.2,
      interpretation: 'HEVC achieves similar quality at 45.8% lower bitrate than AVC',
    });

    const user = userEvent.setup();
    render(<RDCurvesPanel />);

    await user.click(screen.getByRole('button', { name: /load demo data/i }));

    const calculateButton = screen.getByRole('button', { name: /calculate bd-rate/i });
    await user.click(calculateButton);

    // Wait for results section to appear
    await waitFor(() => {
      // Look for the specific results header
      expect(screen.getByText(/BD-Rate Calculation Results/i)).toBeInTheDocument();
    }, { timeout: 3000 });

    // Check for specific values - there should be multiple "BD-Rate" elements (button + result)
    expect(screen.getAllByText(/BD-Rate/i).length).toBeGreaterThan(1);
    expect(screen.getByText(/BD-PSNR/i)).toBeInTheDocument();
    expect(screen.getByText(/Interpretation/i)).toBeInTheDocument();
  });

  it('clears data when clear button clicked', async () => {
    const user = userEvent.setup();
    render(<RDCurvesPanel />);

    await user.click(screen.getByRole('button', { name: /load demo data/i }));
    await waitFor(() => {
      expect(screen.queryByText(/no data loaded/i)).not.toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /clear/i }));
    expect(screen.getByText(/no data loaded/i)).toBeInTheDocument();
  });

  it('displays info about bd-rate', () => {
    render(<RDCurvesPanel />);

    expect(screen.getByText(/bjøntegaard delta rate/i)).toBeInTheDocument();
  });
});
