/**
 * Error Handling - v0.8.x Stabilization
 *
 * Graceful degradation and user-friendly error messages
 */

export interface ErrorContext {
  operation: string;
  component?: string;
  filePath?: string;
  frameIndex?: number;
  additionalInfo?: Record<string, unknown>;
}

export interface UserFriendlyError {
  title: string;
  message: string;
  suggestion: string;
  details?: string;
  errorCode?: string;
  isRecoverable: boolean;
}

/**
 * Bitvue error types
 */
export enum BitvueErrorType {
  // File errors
  FileNotFound = 'FILE_NOT_FOUND',
  FileReadError = 'FILE_READ_ERROR',
  InvalidFormat = 'INVALID_FORMAT',
  UnsupportedCodec = 'UNSUPPORTED_CODEC',

  // Frame errors
  FrameDecodeError = 'FRAME_DECODE_ERROR',
  FrameIndexError = 'FRAME_INDEX_ERROR',
  FrameCorrupted = 'FRAME_CORRUPTED',

  // Memory errors
  OutOfMemory = 'OUT_OF_MEMORY',
  CacheError = 'CACHE_ERROR',

  // Analysis errors
  AnalysisError = 'ANALYSIS_ERROR',
  ParseError = 'PARSE_ERROR',

  // UI errors
  RenderError = 'RENDER_ERROR',
  StateError = 'STATE_ERROR',

  // Unknown
  Unknown = 'UNKNOWN',
}

/**
 * Bitvue error class
 */
export class BitvueError extends Error {
  type: BitvueErrorType;
  context: ErrorContext;
  isRecoverable: boolean;
  originalError?: Error;

  constructor(
    type: BitvueErrorType,
    message: string,
    context: ErrorContext,
    isRecoverable = true,
    originalError?: Error
  ) {
    super(message);
    this.name = 'BitvueError';
    this.type = type;
    this.context = context;
    this.isRecoverable = isRecoverable;
    this.originalError = originalError;
  }

  /**
   * Convert to user-friendly error
   */
  toUserFriendly(): UserFriendlyError {
    return convertToUserFriendly(this);
  }

  /**
   * Get error code for logging
   */
  getErrorCode(): string {
    return `${this.type}_${this.context.operation}`;
  }
}

/**
 * Convert error to user-friendly format
 */
function convertToUserFriendly(error: BitvueError): UserFriendlyError {
  const { type, context, isRecoverable } = error;

  switch (type) {
    case BitvueErrorType.FileNotFound:
      return {
        title: 'File Not Found',
        message: `The file "${context.filePath || 'unknown'}" could not be found.`,
        suggestion: 'Please check if the file exists and you have permission to access it.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.FileReadError:
      return {
        title: 'File Read Error',
        message: 'An error occurred while reading the file.',
        suggestion: 'The file may be corrupted or in use by another application. Try closing other programs and reopening the file.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.InvalidFormat:
      return {
        title: 'Invalid File Format',
        message: 'The file format is not recognized or is corrupted.',
        suggestion: 'Please ensure the file is a valid video file (IVF, MP4, MKV, WebM, etc.).',
        isRecoverable: false,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.UnsupportedCodec:
      return {
        title: 'Unsupported Codec',
        message: 'This video codec is not currently supported.',
        suggestion: 'Bitvue supports AV1, H.264/AVC, HEVC/H.265, VP9, VVC/H.266, and AV3. Please convert your video to a supported codec.',
        isRecoverable: false,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.FrameDecodeError:
      return {
        title: 'Frame Decode Error',
        message: `Failed to decode frame ${context.frameIndex ?? 'unknown'}.`,
        suggestion: 'This frame may be corrupted. Try skipping to the next frame. The video file may be partially corrupted.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.FrameIndexError:
      return {
        title: 'Frame Index Error',
        message: 'Invalid frame index requested.',
        suggestion: `The video has ${context.additionalInfo?.totalFrames ?? 'unknown'} frames. Please select a valid frame number.`,
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.FrameCorrupted:
      return {
        title: 'Corrupted Frame',
        message: `Frame ${context.frameIndex ?? 'unknown'} appears to be corrupted.`,
        suggestion: 'You can skip this frame and continue. Consider checking the source file for corruption.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.OutOfMemory:
      return {
        title: 'Out of Memory',
        message: 'The application is running low on memory.',
        suggestion: 'Try closing other applications or reducing the number of loaded frames. Consider using a smaller video file.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.CacheError:
      return {
        title: 'Cache Error',
        message: 'An error occurred with the frame cache.',
        suggestion: 'The cache has been cleared. Try reloading the frame. This may happen with very large video files.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.AnalysisError:
      return {
        title: 'Analysis Error',
        message: 'Failed to analyze frame data.',
        suggestion: 'Some analysis features may not be available for this codec or frame type. Other features should still work.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.ParseError:
      return {
        title: 'Parse Error',
        message: 'An error occurred while parsing the bitstream.',
        suggestion: 'The video file may use unsupported encoding features. Basic playback should still work.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.RenderError:
      return {
        title: 'Render Error',
        message: 'Failed to render the frame.',
        suggestion: 'Try refreshing the view or selecting a different frame. This may be a temporary issue.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    case BitvueErrorType.StateError:
      return {
        title: 'State Error',
        message: 'An internal state error occurred.',
        suggestion: 'Try reloading the file. If the problem persists, please restart the application.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };

    default:
      return {
        title: 'Unknown Error',
        message: error.message || 'An unexpected error occurred.',
        suggestion: 'Please try again. If the problem persists, please restart the application.',
        isRecoverable: true,
        errorCode: error.getErrorCode(),
      };
  }
}

/**
 * Wrap an async function with error handling
 */
export function withErrorHandling<T extends (...args: unknown[]) => Promise<unknown>>(
  fn: T,
  context: ErrorContext
): T {
  return (async (...args: unknown[]) => {
    try {
      return await fn(...args);
    } catch (error) {
      if (error instanceof BitvueError) {
        throw error;
      }

      // Convert to BitvueError
      const bitvueError = new BitvueError(
        BitvueErrorType.Unknown,
        error instanceof Error ? error.message : String(error),
        context,
        true,
        error instanceof Error ? error : undefined
      );

      throw bitvueError;
    }
  }) as T;
}

/**
 * Create error context from operation name
 */
export function createErrorContext(
  operation: string,
  additionalInfo?: Record<string, unknown>
): ErrorContext {
  return {
    operation,
    ...additionalInfo,
  };
}

/**
 * Parse error and convert to BitvueError
 */
export function parseError(error: unknown, context: ErrorContext): BitvueError {
  if (error instanceof BitvueError) {
    return error;
  }

  if (error instanceof Error) {
    // Try to determine error type from message
    const message = error.message.toLowerCase();

    if (message.includes('not found') || message.includes('no such file')) {
      return new BitvueError(
        BitvueErrorType.FileNotFound,
        error.message,
        context,
        true,
        error
      );
    }

    if (message.includes('permission') || message.includes('access denied')) {
      return new BitvueError(
        BitvueErrorType.FileReadError,
        'Permission denied accessing the file.',
        context,
        false,
        error
      );
    }

    if (message.includes('memory') || message.includes('allocation')) {
      return new BitvueError(
        BitvueErrorType.OutOfMemory,
        error.message,
        context,
        true,
        error
      );
    }

    if (message.includes('decode') || message.includes('corrupt')) {
      return new BitvueError(
        BitvueErrorType.FrameDecodeError,
        error.message,
        context,
        true,
        error
      );
    }

    return new BitvueError(
      BitvueErrorType.Unknown,
      error.message,
      context,
      true,
      error
    );
  }

  return new BitvueError(
    BitvueErrorType.Unknown,
    String(error),
    context,
    true
  );
}

/**
 * Log error to console with context
 */
export function logError(error: BitvueError | Error | unknown): void {
  if (error instanceof BitvueError) {
    console.error(`[${error.type}] ${error.context.operation}:`, {
      message: error.message,
      context: error.context,
      recoverable: error.isRecoverable,
      original: error.originalError,
    });
  } else {
    console.error('Error:', error);
  }
}

/**
 * Recovery strategies for different error types
 */
export enum RecoveryStrategy {
  Retry = 'retry',
  Skip = 'skip',
  ClearCache = 'clear_cache',
  Reload = 'reload',
  CloseFile = 'close_file',
  Nothing = 'nothing',
}

/**
 * Get recovery strategy for error type
 */
export function getRecoveryStrategy(error: BitvueError): RecoveryStrategy {
  switch (error.type) {
    case BitvueErrorType.FileNotFound:
    case BitvueErrorType.InvalidFormat:
    case BitvueErrorType.UnsupportedCodec:
      return RecoveryStrategy.CloseFile;

    case BitvueErrorType.FileReadError:
      return RecoveryStrategy.Retry;

    case BitvueErrorType.FrameDecodeError:
    case BitvueErrorType.FrameCorrupted:
      return RecoveryStrategy.Skip;

    case BitvueErrorType.OutOfMemory:
    case BitvueErrorType.CacheError:
      return RecoveryStrategy.ClearCache;

    case BitvueErrorType.RenderError:
    case BitvueErrorType.StateError:
      return RecoveryStrategy.Reload;

    default:
      return RecoveryStrategy.Nothing;
  }
}
