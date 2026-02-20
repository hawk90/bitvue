/**
 * useFileOperations Hook
 *
 * Custom hook for file operations (open/close bitstream files)
 * Handles file dialogs, Tauri commands, and error states
 *
 * Usage:
 * ```tsx
 * const { openBitstream, closeBitstream, isLoading, error } = useFileOperations();
 *
 * // Open a file with optional codec type
 * await openBitstream('av1');
 *
 * // Close the current file
 * await closeBitstream();
 * ```
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { logger } from "../utils/logger";
import type { FileInfo, FileOpenedEvent } from "../types/video";

/**
 * Supported codec types for opening files
 */
export type CodecType =
  | "av1"
  | "hevc"
  | "avc"
  | "vp9"
  | "vvc"
  | "mpeg2"
  | "auto";

/**
 * Valid codec types for validation
 */
const VALID_CODECS: readonly CodecType[] = [
  "av1",
  "hevc",
  "avc",
  "vp9",
  "vvc",
  "mpeg2",
  "auto",
];

/**
 * Validate codec type
 */
function isValidCodec(codec: string): codec is CodecType {
  return VALID_CODECS.includes(codec as CodecType);
}

/**
 * File operation result
 */
export interface FileOperationResult {
  success: boolean;
  path?: string;
  error?: string;
}

/**
 * File operations hook state and methods
 */
export interface UseFileOperationsResult {
  /** Whether a file operation is in progress */
  isLoading: boolean;
  /** Error message from the last operation */
  error: string | null;
  /** Information about the currently loaded file */
  fileInfo: FileInfo | null;
  /**
   * Open a bitstream file
   * @param codec - Optional codec type to force (defaults to 'auto')
   */
  openBitstream: (codec?: CodecType) => Promise<FileOperationResult>;
  /** Close the currently loaded bitstream file */
  closeBitstream: () => Promise<void>;
  /** Clear the current error state */
  clearError: () => void;
}

/**
 * File filter for the open file dialog
 */
const FILE_FILTERS = [
  {
    name: "Video Files",
    extensions: [
      "ivf",
      "av1",
      "hevc",
      "h265",
      "vvc",
      "h266",
      "mp4",
      "mkv",
      "webm",
      "ts",
    ],
  },
  {
    name: "All Files",
    extensions: ["*"],
  },
];

/**
 * Hook for managing file operations (open/close bitstreams)
 */
export function useFileOperations(): UseFileOperationsResult {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fileInfo, setFileInfo] = useState<FileInfo | null>(null);

  /**
   * Open a bitstream file
   */
  const openBitstream = useCallback(
    async (codec?: CodecType): Promise<FileOperationResult> => {
      setIsLoading(true);
      setError(null);

      // Validate codec parameter if provided
      if (codec !== undefined && !isValidCodec(codec)) {
        const errorMsg = `Invalid codec type: "${codec}". Valid types are: ${VALID_CODECS.join(", ")}`;
        setError(errorMsg);
        logger.error("Invalid codec type:", codec);
        setIsLoading(false);
        return { success: false, error: errorMsg };
      }

      try {
        const selected = await open({
          multiple: false,
          filters: FILE_FILTERS,
        });

        if (selected && typeof selected === "string") {
          logger.info("Opening file:", selected);

          // Call the Tauri command to open the file
          const result = await invoke<FileInfo>("open_file", {
            path: selected,
            codec: codec || "auto",
          });

          setFileInfo(result);

          if (result.success) {
            logger.info("File opened successfully");
            return { success: true, path: selected };
          } else {
            const errorMsg = result.error || "Unknown error";
            setError(errorMsg);
            logger.error("Failed to open file:", errorMsg);
            return { success: false, error: errorMsg, path: selected };
          }
        }

        return { success: false };
      } catch (err) {
        const errorMsg = err as string;
        setError(errorMsg);
        logger.error("Failed to open file:", err);
        return { success: false, error: errorMsg };
      } finally {
        setIsLoading(false);
      }
    },
    [],
  );

  /**
   * Close the currently loaded bitstream file
   */
  const closeBitstream = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);

    try {
      await invoke("close_file");
      setFileInfo(null);
      logger.info("File closed successfully");
    } catch (err) {
      const errorMsg = err as string;
      setError(errorMsg);
      logger.error("Failed to close file:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Clear the current error state
   */
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Use refs to store stable references to the callback functions
  // This prevents the event listeners from being re-registered on every render
  const openBitstreamRef = useRef(openBitstream);
  const closeBitstreamRef = useRef(closeBitstream);

  // Update the refs whenever the callbacks change
  useEffect(() => {
    openBitstreamRef.current = openBitstream;
    closeBitstreamRef.current = closeBitstream;
  });

  // Set up menu event listeners for file operations
  // Empty dependency array ensures this only runs once
  useEffect(() => {
    const handleOpenBitstream = () => {
      openBitstreamRef.current();
    };

    const handleCloseFile = () => {
      closeBitstreamRef.current();
    };

    window.addEventListener("menu-open-bitstream", handleOpenBitstream);
    window.addEventListener("menu-close-file", handleCloseFile);

    return () => {
      window.removeEventListener("menu-open-bitstream", handleOpenBitstream);
      window.removeEventListener("menu-close-file", handleCloseFile);
    };
  }, []); // Empty deps - listeners are set up once and use refs for callbacks

  // Listen for file-opened events from Tauri
  useEffect(() => {
    const unlisten = listen<FileOpenedEvent>("file-opened", async (event) => {
      setFileInfo(event.payload);
      if (event.payload.success) {
        logger.info(`Opened: ${event.payload.path}`);
      } else {
        setError(event.payload.error || "Unknown error");
        logger.error("Failed to open file:", event.payload.error);
      }
    });

    return () => {
      // Proper cleanup: handle potential errors during unlisten
      unlisten
        .then((fn) => fn())
        .catch((err) => {
          logger.warn("Failed to unlisten from file-opened event:", err);
        });
    };
  }, []);

  return {
    isLoading,
    error,
    fileInfo,
    openBitstream,
    closeBitstream,
    clearError,
  };
}

export default useFileOperations;
