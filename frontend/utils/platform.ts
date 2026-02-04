/**
 * Platform detection utilities for platform-specific UI behavior
 */

import { createLogger } from './logger';

const logger = createLogger('platform');

export type Platform = 'macos' | 'windows' | 'linux' | 'unknown';

/**
 * Detect the current platform
 */
export function detectPlatform(): Platform {
  const userAgent = navigator.userAgent;
  const platform = navigator.platform;

  logger.debug('userAgent:', userAgent, 'platform:', platform);

  if (platform.startsWith('Mac') || /Macintosh|MacIntel|MacPPC|Mac68K/i.test(userAgent)) {
    logger.debug('Detected as macOS');
    return 'macos';
  }
  if (platform.startsWith('Win') || /Windows|Win32|Win64/i.test(userAgent)) {
    logger.debug('Detected as Windows');
    return 'windows';
  }
  if (platform.startsWith('Linux') || /Linux/i.test(userAgent)) {
    logger.debug('Detected as Linux');
    return 'linux';
  }
  logger.debug('Platform unknown');
  return 'unknown';
}

/**
 * Check if the current platform is macOS
 */
export function isMacOS(): boolean {
  return detectPlatform() === 'macos';
}

/**
 * Check if the current platform is Windows
 */
export function isWindows(): boolean {
  return detectPlatform() === 'windows';
}

/**
 * Check if the current platform is Linux
 */
export function isLinux(): boolean {
  return detectPlatform() === 'linux';
}

/**
 * Check if we should use the native system menu (macOS)
 */
export function shouldUseNativeMenu(): boolean {
  return isMacOS();
}

/**
 * Check if we should show the custom title bar (Windows/Linux)
 */
export function shouldShowTitleBar(): boolean {
  return isWindows() || isLinux();
}
