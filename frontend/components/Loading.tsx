/**
 * Loading & Skeleton States
 *
 * Reusable loading indicators and skeleton screens
 */

import { memo } from 'react';
import './Loading.css';

export const Skeleton = memo(function Skeleton({ width, height, variant = 'default' }: {
  width?: string | number;
  height?: string | number;
  variant?: 'default' | 'text' | 'circular' | 'rectangular';
}) {
  const style: React.CSSProperties = {};
  if (width) style.width = typeof width === 'number' ? `${width}px` : width;
  if (height) style.height = typeof height === 'number' ? `${height}px` : height;

  return (
    <div className={`skeleton skeleton-${variant}`} style={style} />
  );
});

export const SkeletonBlock = memo(function SkeletonBlock({
  rows = 3,
  width = '100%',
  height = 12,
}: {
  rows?: number;
  width?: string | number;
  height?: number;
}) {
  return (
    <div className="skeleton-block">
      {Array.from({ length: rows }).map((_, i) => (
        <Skeleton
          key={i}
          width={width}
          height={height}
          variant="default"
        />
      ))}
    </div>
  );
});

export const Spinner = memo(function Spinner({ size = 'md', active = true }: {
  size?: 'sm' | 'md' | 'lg';
  active?: boolean;
}) {
  return (
    <div className={`spinner spinner-${size} ${active ? 'active' : ''}`}>
      <div className="spinner-dot" />
      <div className="spinner-dot" />
      <div className="spinner-dot" />
    </div>
  );
});

export const LoadingScreen = memo(function LoadingScreen({
  title = 'Loading...',
  message,
  progress
}: {
  title?: string;
  message?: string;
  progress?: number;
}) {
  return (
    <div className="loading-screen">
      <div className="loading-content">
        <Spinner size="lg" />
        <h2 className="loading-title">{title}</h2>
        {message && <p className="loading-message">{message}</p>}
        {progress !== undefined && (
          <div className="loading-progress">
            <div className="loading-progress-bar" style={{ width: `${progress}%` }} />
            <span className="loading-progress-text">{Math.round(progress)}%</span>
          </div>
        )}
      </div>
    </div>
  );
});

export const InlineLoading = memo(function InlineLoading({ text = 'Loading...' }: { text?: string }) {
  return (
    <div className="inline-loading">
      <Spinner size="sm" />
      <span>{text}</span>
    </div>
  );
});
