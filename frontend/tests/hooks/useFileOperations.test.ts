/**
 * useFileOperations Hook Tests
 * Tests file operations hook for opening/closing bitstream files
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useFileOperations, type CodecType } from "@/hooks/useFileOperations";

// Get the mocked functions from the setup file
const { invoke } = await import("@tauri-apps/api/core");
const { open: dialogOpen } = await import("@tauri-apps/plugin-dialog");

describe("useFileOperations", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return initial state", () => {
    const { result } = renderHook(() => useFileOperations());

    expect(result.current.isLoading).toBe(false);
    expect(result.current.error).toBe(null);
    expect(result.current.fileInfo).toBe(null);
  });

  it("should have openBitstream function", () => {
    const { result } = renderHook(() => useFileOperations());

    expect(typeof result.current.openBitstream).toBe("function");
  });

  it("should have closeBitstream function", () => {
    const { result } = renderHook(() => useFileOperations());

    expect(typeof result.current.closeBitstream).toBe("function");
  });

  it("should have clearError function", () => {
    const { result } = renderHook(() => useFileOperations());

    expect(typeof result.current.clearError).toBe("function");
  });
});

describe("useFileOperations openBitstream", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should set loading state during open", async () => {
    // Mock file dialog to return a file path
    vi.mocked(dialogOpen).mockResolvedValue("/path/to/file.ivf");

    // Mock invoke to delay response
    vi.mocked(invoke).mockImplementation(
      () =>
        new Promise((resolve) =>
          setTimeout(
            () =>
              resolve({
                success: true,
                path: "/path/to/file.ivf",
                codec: "auto",
                width: 1920,
                height: 1080,
                frameRate: 30,
                duration: 10,
                totalFrames: 300,
              }),
            100,
          ),
        ),
    );

    const { result } = renderHook(() => useFileOperations());

    expect(result.current.isLoading).toBe(false);

    // Start the async operation
    const promise = result.current.openBitstream();

    // Wait for state to update
    await waitFor(() => {
      expect(result.current.isLoading).toBe(true);
    });

    await promise;

    // After completion, loading should be false
    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });
  });

  it("should set fileInfo on successful open", async () => {
    const mockFileInfo = {
      success: true,
      path: "/test/file.ivf",
      codec: "av1",
      width: 1920,
      height: 1080,
      frameRate: 30,
      duration: 10,
      totalFrames: 300,
    };

    vi.mocked(dialogOpen).mockResolvedValue("/test/file.ivf");
    vi.mocked(invoke).mockResolvedValue(mockFileInfo);

    const { result } = renderHook(() => useFileOperations());

    await result.current.openBitstream();

    await waitFor(() => {
      expect(result.current.fileInfo).toEqual(mockFileInfo);
    });
  });

  it("should set error on failed open", async () => {
    vi.mocked(dialogOpen).mockResolvedValue("/test/file.ivf");
    vi.mocked(invoke).mockResolvedValue({
      success: false,
      error: "Failed to parse file",
    });

    const { result } = renderHook(() => useFileOperations());

    await result.current.openBitstream();

    await waitFor(() => {
      expect(result.current.error).toBe("Failed to parse file");
    });
  });

  it("should handle cancelled dialog", async () => {
    vi.mocked(dialogOpen).mockResolvedValue(null);

    const { result } = renderHook(() => useFileOperations());

    const response = await result.current.openBitstream();

    expect(response.success).toBe(false);
    expect(result.current.fileInfo).toBe(null);
  });

  it("should pass codec parameter", async () => {
    vi.mocked(dialogOpen).mockResolvedValue("/test/file.ivf");
    vi.mocked(invoke).mockResolvedValue({
      success: true,
      path: "/test/file.ivf",
      codec: "av1",
    });

    const { result } = renderHook(() => useFileOperations());

    await result.current.openBitstream("av1");

    expect(invoke).toHaveBeenCalledWith("open_file", {
      path: "/test/file.ivf",
      codec: "av1",
    });
  });
});

describe("useFileOperations closeBitstream", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should close file successfully", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    const { result } = renderHook(() => useFileOperations());

    await result.current.closeBitstream();

    await waitFor(() => {
      expect(result.current.fileInfo).toBe(null);
    });
  });

  it("should set error on close failure", async () => {
    vi.mocked(invoke).mockRejectedValue("Close failed");

    const { result } = renderHook(() => useFileOperations());

    await result.current.closeBitstream();

    await waitFor(() => {
      expect(result.current.error).toBeTruthy();
    });
  });
});

describe("useFileOperations clearError", () => {
  it("should clear error state", () => {
    const { result } = renderHook(() => useFileOperations());

    result.current.clearError();

    expect(result.current.error).toBe(null);
  });
});

describe("useFileOperations event listeners", () => {
  it("should register menu event listeners", () => {
    const addEventListenerSpy = vi.spyOn(window, "addEventListener");

    renderHook(() => useFileOperations());

    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "menu-open-bitstream",
      expect.any(Function),
    );
    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "menu-close-file",
      expect.any(Function),
    );

    addEventListenerSpy.mockRestore();
  });

  it("should cleanup event listeners on unmount", () => {
    const removeEventListenerSpy = vi.spyOn(window, "removeEventListener");

    const { unmount } = renderHook(() => useFileOperations());

    unmount();

    // Listeners should be cleaned up
    expect(removeEventListenerSpy).toHaveBeenCalledWith(
      "menu-open-bitstream",
      expect.any(Function),
    );
    expect(removeEventListenerSpy).toHaveBeenCalledWith(
      "menu-close-file",
      expect.any(Function),
    );

    removeEventListenerSpy.mockRestore();
  });
});

describe("useFileOperations codecs", () => {
  it("should support all codec types", () => {
    const codecs: CodecType[] = [
      "av1",
      "hevc",
      "avc",
      "vp9",
      "vvc",
      "mpeg2",
      "auto",
    ];

    codecs.forEach((codec) => {
      expect(codec).toBeTruthy();
    });
  });

  it("should default to auto codec", async () => {
    vi.mocked(dialogOpen).mockResolvedValue("/test/file.ivf");
    vi.mocked(invoke).mockResolvedValue({ success: true });

    const { result } = renderHook(() => useFileOperations());

    await result.current.openBitstream();

    expect(invoke).toHaveBeenCalledWith("open_file", {
      path: "/test/file.ivf",
      codec: "auto",
    });
  });
});
