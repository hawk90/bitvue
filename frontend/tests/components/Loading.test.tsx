/**
 * Loading Component Tests
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { Skeleton, SkeletonBlock, Spinner, LoadingScreen, InlineLoading } from '@/components/Loading';

describe('Skeleton', () => {
  it('should render with default styles', () => {
    const { container } = render(<Skeleton />);
    const skeleton = container.querySelector('.skeleton');
    expect(skeleton).toBeInTheDocument();
    expect(skeleton).toHaveClass('skeleton-default');
  });

  it('should render with custom dimensions', () => {
    const { container } = render(<Skeleton width="100px" height={50} />);
    const skeleton = container.querySelector('.skeleton') as HTMLElement;
    expect(skeleton.style.width).toBe('100px');
    expect(skeleton.style.height).toBe('50px');
  });

  it('should render with numeric dimensions', () => {
    const { container } = render(<Skeleton width={100} height={50} />);
    const skeleton = container.querySelector('.skeleton') as HTMLElement;
    expect(skeleton.style.width).toBe('100px');
    expect(skeleton.style.height).toBe('50px');
  });

  it('should render with different variants', () => {
    const { container: textContainer } = render(<Skeleton variant="text" />);
    const { container: circularContainer } = render(<Skeleton variant="circular" />);
    const { container: rectContainer } = render(<Skeleton variant="rectangular" />);

    expect(textContainer.querySelector('.skeleton-text')).toBeInTheDocument();
    expect(circularContainer.querySelector('.skeleton-circular')).toBeInTheDocument();
    expect(rectContainer.querySelector('.skeleton-rectangular')).toBeInTheDocument();
  });
});

describe('SkeletonBlock', () => {
  it('should render with default rows', () => {
    const { container } = render(<SkeletonBlock />);
    const skeletons = container.querySelectorAll('.skeleton');
    expect(skeletons).toHaveLength(3);
  });

  it('should render with custom rows', () => {
    const { container } = render(<SkeletonBlock rows={5} />);
    const skeletons = container.querySelectorAll('.skeleton');
    expect(skeletons).toHaveLength(5);
  });

  it('should render with custom dimensions', () => {
    const { container } = render(<SkeletonBlock width="80%" height={16} />);
    const skeleton = container.querySelector('.skeleton') as HTMLElement;
    expect(skeleton.style.width).toBe('80%');
    expect(skeleton.style.height).toBe('16px');
  });
});

describe('Spinner', () => {
  it('should render with default size and active state', () => {
    const { container } = render(<Spinner />);
    const spinner = container.querySelector('.spinner');
    expect(spinner).toBeInTheDocument();
    expect(spinner).toHaveClass('spinner-md');
    expect(spinner).toHaveClass('active');
  });

  it('should render with different sizes', () => {
    const { container: smContainer } = render(<Spinner size="sm" />);
    const { container: lgContainer } = render(<Spinner size="lg" />);

    expect(smContainer.querySelector('.spinner-sm')).toBeInTheDocument();
    expect(lgContainer.querySelector('.spinner-lg')).toBeInTheDocument();
  });

  it('should not be active when active prop is false', () => {
    const { container } = render(<Spinner active={false} />);
    const spinner = container.querySelector('.spinner');
    expect(spinner).not.toHaveClass('active');
  });

  it('should render three dots', () => {
    const { container } = render(<Spinner />);
    const dots = container.querySelectorAll('.spinner-dot');
    expect(dots).toHaveLength(3);
  });
});

describe('LoadingScreen', () => {
  it('should render with default title', () => {
    render(<LoadingScreen />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should render with custom title', () => {
    render(<LoadingScreen title="Custom Title" />);
    expect(screen.getByText('Custom Title')).toBeInTheDocument();
  });

  it('should render with message', () => {
    render(<LoadingScreen message="Loading your data..." />);
    expect(screen.getByText('Loading your data...')).toBeInTheDocument();
  });

  it('should render with progress bar', () => {
    render(<LoadingScreen progress={50} />);
    expect(screen.getByText('50%')).toBeInTheDocument();

    const progressBar = document.querySelector('.loading-progress-bar') as HTMLElement;
    expect(progressBar.style.width).toBe('50%');
  });

  it('should round progress percentage', () => {
    render(<LoadingScreen progress={66.666} />);
    expect(screen.getByText('67%')).toBeInTheDocument();
  });
});

describe('InlineLoading', () => {
  it('should render with default text', () => {
    render(<InlineLoading />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should render with custom text', () => {
    render(<InlineLoading text="Processing..." />);
    expect(screen.getByText('Processing...')).toBeInTheDocument();
  });

  it('should render spinner and text', () => {
    const { container } = render(<InlineLoading text="Loading..." />);
    expect(container.querySelector('.spinner')).toBeInTheDocument();
    expect(container.querySelector('.spinner-sm')).toBeInTheDocument();
  });
});
