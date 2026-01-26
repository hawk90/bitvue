import { render, screen } from '@testing-library/react';
import QualityComparisonPanel from '../QualityComparisonPanel';

describe('QualityComparisonPanel', () => {
  it('renders file selection inputs', () => {
    render(<QualityComparisonPanel />);

    expect(screen.getByText(/Reference File/i)).toBeInTheDocument();
    expect(screen.getByText(/Distorted File/i)).toBeInTheDocument();
  });

  it('disables calculate button when no files selected', () => {
    render(<QualityComparisonPanel />);

    const button = screen.getByRole('button', { name: /calculate/i });
    expect(button).toBeDisabled();
  });

  it('enables calculate button when both files selected', async () => {
    render(<QualityComparisonPanel />);

    const fileInput1 = screen.getAllByLabelTextText(/Reference File/i)[0] as HTMLInputElement;
    const fileInput2 = screen.getAllByLabelTextText(/Distorted File/i)[0] as HTMLInputElement;

    const file1 = new File([''], 'ref.mp4', { type: 'video/mp4' });
    const file2 = new File([''], 'dist.mp4', { type: 'video/mp4' });

    await userEvent.upload(fileInput1, file1);
    await userEvent.upload(fileInput2, file2);

    const button = screen.getByRole('button', { name: /calculate/i });
    expect(button).not.toBeDisabled();
  });

  it('shows quality metrics after calculation', async () => {
    render(<QualityComparisonPanel />);

    const fileInput1 = screen.getAllByLabelTextText(/Reference File/i)[0] as HTMLInputElement;
    const fileInput2 = screen.getAllByLabelTextText(/Distorted File/i)[0] as HTMLInputElement;

    const file1 = new File([''], 'ref.mp4', { type: 'video/mp4' });
    const file2 = new File([''], 'dist.mp4', { type: 'video/mp4' });

    await userEvent.upload(fileInput1, file1);
    await userEvent.upload(fileInput2, file2);

    const button = screen.getByRole('button', { name: /calculate/i });
    await userEvent.click(button);

    // Wait for mock calculation
    await waitFor(() => {
      expect(screen.getByText(/PSNR/i)).toBeInTheDocument();
      expect(screen.getByText(/SSIM/i)).toBeInTheDocument();
    }, { timeout: 3000 });
  });
});
