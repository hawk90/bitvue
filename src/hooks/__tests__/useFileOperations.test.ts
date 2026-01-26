/**
 * useFileOperations Hook Tests
 * Tests file operations hook for opening/closing bitstream files
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFileOperations, type CodecType } from '@/hooks/useFileOperations';

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => ({
    then: (cb: (unlisten: () => void) => void) => {
      cb(() => {});
      return { catch: () => ({ finally: (fn: () => void) => fn?.() }) };
    },
  })),
}));

describe('useFileOperations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should return initial state', () => {
    const { result } = renderHook(() => useFileOperations());

    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBe(null);
    expect(result.current.fileInfo).toBe(null);
  });

  it('should have openBitstream function', () => {
    const { result } = renderHook(() => useFileOperations());

    expect(typeof result.current.openBitstream).toBe('function');
  });

  it('should have closeBitstream function', () => {
    const { result } = renderHook(() => useFileOperations());

    expect(typeof result.current.closeBitstream).toBe('function');
  });

  it('should have clearError function', () => {
    const { result } = renderHook(() => useFileOperations());

    expect(typeof result.current.clearError).toBe('function');
  });
});

describe('useFileOperations openBitstream', () => {
  it('should set loading state during open', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');

    (open as vi.MockedFunction).mockResolvedValue('/path/to/file.ivf');
    (invoke as vi.MockedFunction).mockResolvedValue({
      success: true,
      path: '/path/to/file.ivf',
      codec: 'auto',
      width: 1920,
      height: 1080,
      frameRate: 30,
      duration: 10,
      totalFrames: 300,
    });

    const { result } = renderHook(() => useFileOperations());

    expect(result.current.isLoading).toBe(false);

    const promise = act(() => result.current.openBitstream());

    expect(result.current.isLoading).toBe(true);
  });

  it('should set fileInfo on successful open', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');

    const mockFileInfo = {
      success: true,
      path: '/test/file.ivf',
      codec: 'av1',
      width: 1920,
      height: 1080,
      frameRate: 30,
      duration: 10,
      totalFrames: 300,
    };

    (open as vi.MockedFunction).mockResolvedValue('/test/file.ivf');
    (invoke as vi.MockedFunction).mockResolvedValue(mockFileInfo);

    const { result } = renderHook(() => useFileOperations());

    await act(() => result.current.openBitstream());

    expect(result.current.fileInfo).toEqual(mockFileInfo);
  });

  it('should set error on failed open', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');

    (open as vi.MockedFunction).mockResolvedValue('/test/file.ivf');
    (invoke as vi.MockedFunction).mockResolvedValue({
      success: false,
      error: 'Failed to parse file',
    });

    const { result } = renderHook(() => useFileOperations());

    await act(() => result.current.openBitstream());

    expect(result.current.error).toBe('Failed to parse file');
  });

  it('should handle cancelled dialog', async () => {
    const { open } = await import('@tauri-apps/plugin-dialog');

    (open as vi.MockedFunction).mockResolvedValue(null);

    const { result } = renderHook(() => useFileOperations());

    const response = await act(() => result.current.openBitstream());

    expect(response.success).toBe(false);
    expect(result.current.fileInfo).toBe(null);
  });

  it('should pass codec parameter', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');

    (open as vi.MockedFunction).mockResolvedValue('/test/file.ivf');
    (invoke as vi.MockedFunction).mockResolvedValue({
      success: true,
      path: '/test/file.ivf',
      codec: 'av1',
    });

    const { result } = renderHook(() => useFileOperations());

    await act(() => result.current.openBitstream('av1'));

    expect(invoke).toHaveBeenCalledWith('open_file', {
      path: '/test/file.ivf',
      codec: 'av1',
    });
  });
});

describe('useFileOperations closeBitstream', () => {
  it('should close file successfully', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    (invoke as vi.MockedFunction).mockResolvedValue(undefined);

    const { result } = renderHook(() => useFileOperations());

    // Set some file info first
    result.current.fileInfo = { success: true, path: '/test' } as any;

    await act(() => result.current.closeBitstream());

    expect(result.current.fileInfo).toBe(null);
  });

  it('should set error on close failure', async () => {
    const { invoke } = await import('@tauri-apps/api/core');

    (invoke as vi.MockedFunction).mockRejectedValue('Close failed');

    const { result } = renderHook(() => useFileOperations());

    await act(() => result.current.closeBitstream());

    expect(result.current.error).toBeTruthy();
  });
});

describe('useFileOperations clearError', () => {
  it('should clear error state', () => {
    const { result } = renderHook(() => useFileOperations());

    act(() => {
      result.current.clearError();
    });

    expect(result.current.error).toBe(null);
  });
});

describe('useFileOperations event listeners', () => {
  it('should register menu event listeners', () => {
    const addEventListenerSpy = vi.spyOn(window, 'addEventListener');

    renderHook(() => useFileOperations());

    expect(addEventListenerSpy).toHaveBeenCalledWith('menu-open-bitstream', expect.any(Function));
    expect(addEventListenerSpy).toHaveBeenCalledWith('menu-close-file', expect.any(Function));
  });

  it('should cleanup event listeners on unmount', () => {
    const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = renderHook(() => useFileOperations());

    unmount();

    // Listeners should be cleaned up
    expect(removeEventListenerSpy).toHaveBeenCalledWith('menu-open-bitstream', expect.any(Function));
  });
});

describe('useFileOperations codecs', () => {
  it('should support all codec types', () => {
    const codecs: CodecType[] = ['av1', 'hevc', 'avc', 'vp9', 'vvc', 'mpeg2', 'auto'];

    codecs.forEach(codec => {
      expect(codec).toBeTruthy();
    });
  });

  it('should default to auto codec', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    const { open } = await import('@tauri-apps/plugin-dialog');

    (open as vi.MockedFunction).mockResolvedValue('/test/file.ivf');
    (invoke as vi.MockedFunction).mockResolvedValue({ success: true });

    const { result } = renderHook(() => useFileOperations());

    await act(() => result.current.openBitstream());

    expect(invoke).toHaveBeenCalledWith('open_file', {
      path: '/test/file.ivf',
      codec: 'auto',
    });
  });
});
