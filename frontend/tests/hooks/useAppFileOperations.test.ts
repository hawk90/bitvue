/**
 * useAppFileOperations Hook Tests
 *
 * Covers the 2026-08-08 rewiring to `electronBridgeService` (bitvue-sidecar) — proves
 * `handleOpenFile`/`handleCloseFile` call the new bridge with the right arguments and manage
 * local state correctly, without needing a real Electron process (that's covered separately by
 * `bitvue-desktop/electron/main.ts`'s selftest, which drives the real renderer/preload/sidecar
 * chain). `handleOpenDependentFile` (compare workspaces) is untouched by this migration and not
 * covered here.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useAppFileOperations } from "@/hooks/useAppFileOperations";

const mockSetFilePath = vi.fn();
const mockRefreshFrames = vi.fn().mockResolvedValue([]);
const mockClearData = vi.fn();
const mockSetCurrentFrameIndex = vi.fn();
const mockCreateWorkspace = vi.fn();

vi.mock("@/contexts/StreamDataContext", () => ({
  useFileState: () => ({
    setFilePath: mockSetFilePath,
    refreshFrames: mockRefreshFrames,
    clearData: mockClearData,
  }),
  useCurrentFrame: () => ({
    currentFrameIndex: 0,
    setCurrentFrameIndex: mockSetCurrentFrameIndex,
  }),
}));

vi.mock("@/contexts/CompareContext", () => ({
  useCompare: () => ({ createWorkspace: mockCreateWorkspace }),
}));

const { openStream, closeStream, selectFrame, showOpenDialog } = vi.hoisted(
  () => ({
    openStream: vi.fn(),
    closeStream: vi.fn(),
    selectFrame: vi.fn(),
    showOpenDialog: vi.fn(),
  }),
);

vi.mock("@/services/electronBridgeService", () => ({
  openStream,
  closeStream,
  selectFrame,
  showOpenDialog,
}));

describe("useAppFileOperations", () => {
  const onError = vi.fn();
  const onCodecChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("handleOpenFile", () => {
    it("does nothing if the user cancels the dialog", async () => {
      showOpenDialog.mockResolvedValue(null);
      const { result } = renderHook(() =>
        useAppFileOperations({ onError, onCodecChange }),
      );

      await act(async () => {
        await result.current.handleOpenFile();
      });

      expect(openStream).not.toHaveBeenCalled();
      expect(result.current.fileInfo).toBeNull();
    });

    it("on success: calls openStream, then selectFrame(0), sets fileInfo/filePath, resets frame index", async () => {
      showOpenDialog.mockResolvedValue("/tmp/clip.ivf");
      openStream.mockResolvedValue({
        success: true,
        path: "/tmp/clip.ivf",
        events: [],
        error: undefined,
      });
      selectFrame.mockResolvedValue([
        { type: "SelectionUpdated", stream: "A" },
      ]);

      const { result } = renderHook(() =>
        useAppFileOperations({ onError, onCodecChange }),
      );

      await act(async () => {
        await result.current.handleOpenFile();
      });

      expect(openStream).toHaveBeenCalledWith("A", "/tmp/clip.ivf");
      expect(selectFrame).toHaveBeenCalledWith("A", 0);
      expect(mockSetFilePath).toHaveBeenCalledWith("/tmp/clip.ivf");
      expect(mockSetCurrentFrameIndex).toHaveBeenCalledWith(0);
      await waitFor(() => expect(result.current.fileInfo?.success).toBe(true));
      expect(result.current.fileInfo?.path).toBe("/tmp/clip.ivf");
    });

    it("on success but selectFrame throwing: still succeeds (selection failure is non-blocking)", async () => {
      showOpenDialog.mockResolvedValue("/tmp/clip.ivf");
      openStream.mockResolvedValue({
        success: true,
        path: "/tmp/clip.ivf",
        events: [],
        error: undefined,
      });
      selectFrame.mockRejectedValue(new Error("sidecar not responding"));

      const { result } = renderHook(() =>
        useAppFileOperations({ onError, onCodecChange }),
      );

      await act(async () => {
        await result.current.handleOpenFile();
      });

      await waitFor(() => expect(result.current.fileInfo?.success).toBe(true));
      expect(onError).not.toHaveBeenCalledWith(
        expect.stringContaining("Open"),
        expect.anything(),
        expect.anything(),
      );
    });

    it("on failure (DiagnosticAdded-mapped error): sets filePath to null and calls onError", async () => {
      showOpenDialog.mockResolvedValue("/tmp/missing.ivf");
      openStream.mockResolvedValue({
        success: false,
        path: "/tmp/missing.ivf",
        events: [],
        error: "Failed to open file: No such file or directory",
      });

      const { result } = renderHook(() =>
        useAppFileOperations({ onError, onCodecChange }),
      );

      await act(async () => {
        await result.current.handleOpenFile();
      });

      expect(selectFrame).not.toHaveBeenCalled();
      expect(mockSetFilePath).toHaveBeenCalledWith(null);
      expect(onError).toHaveBeenCalledWith(
        "Failed to Open File",
        "Failed to open file: No such file or directory",
        "/tmp/missing.ivf",
      );
    });
  });

  describe("handleCloseFile", () => {
    it("calls closeStream and resets local state", async () => {
      closeStream.mockResolvedValue([{ type: "ModelUpdated", stream: "A" }]);
      const { result } = renderHook(() =>
        useAppFileOperations({ onError, onCodecChange }),
      );

      await act(async () => {
        await result.current.handleCloseFile();
      });

      expect(closeStream).toHaveBeenCalledWith("A");
      expect(mockSetFilePath).toHaveBeenCalledWith(null);
      expect(mockSetCurrentFrameIndex).toHaveBeenCalledWith(0);
      expect(mockClearData).toHaveBeenCalled();
      expect(onCodecChange).toHaveBeenCalledWith(null);
    });

    it("reports an error via onError if closeStream rejects", async () => {
      closeStream.mockRejectedValue(new Error("sidecar exited"));
      const { result } = renderHook(() =>
        useAppFileOperations({ onError, onCodecChange }),
      );

      await act(async () => {
        await result.current.handleCloseFile();
      });

      expect(onError).toHaveBeenCalledWith(
        "Failed to Close File",
        "sidecar exited",
      );
    });
  });
});
