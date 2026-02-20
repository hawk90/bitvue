/**
 * Unified Error Handling - Consistent error types across the application
 *
 * This module provides a unified error handling system with:
 * - Standardized error types
 * - Error conversion utilities
 * - User-friendly error messages
 * - Proper error propagation
 */

// =============================================================================
// Error Categories
// =============================================================================

/**
 * Error category for display and logging
 */
export enum ErrorCategory {
  /** Input validation failed */
  Validation = "VALIDATION",
  /** File or resource not found */
  NotFound = "NOT_FOUND",
  /** Permission denied */
  Permission = "PERMISSION",
  /** Parsing/decoding error */
  Parse = "PARSE",
  /** I/O operation failed */
  Io = "IO",
  /** Network operation failed */
  Network = "NETWORK",
  /** Codec-specific error */
  Codec = "CODEC",
  /** Internal logic error */
  Internal = "INTERNAL",
  /** Feature not implemented */
  NotImplemented = "NOT_IMPLEMENTED",
}

/**
 * Get display name for the category
 */
export function getCategoryDisplayName(category: ErrorCategory): string {
  const displayNames: Record<ErrorCategory, string> = {
    [ErrorCategory.Validation]: "Validation Error",
    [ErrorCategory.NotFound]: "Not Found",
    [ErrorCategory.Permission]: "Permission Denied",
    [ErrorCategory.Parse]: "Parse Error",
    [ErrorCategory.Io]: "I/O Error",
    [ErrorCategory.Network]: "Network Error",
    [ErrorCategory.Codec]: "Codec Error",
    [ErrorCategory.Internal]: "Internal Error",
    [ErrorCategory.NotImplemented]: "Not Implemented",
  };
  return displayNames[category];
}

/**
 * Get icon for the category (for UI)
 */
export function getCategoryIcon(category: ErrorCategory): string {
  const icons: Record<ErrorCategory, string> = {
    [ErrorCategory.Validation]: "⚠️",
    [ErrorCategory.NotFound]: "🔍",
    [ErrorCategory.Permission]: "🔒",
    [ErrorCategory.Parse]: "📄",
    [ErrorCategory.Io]: "💾",
    [ErrorCategory.Network]: "🌐",
    [ErrorCategory.Codec]: "🎬",
    [ErrorCategory.Internal]: "⚙️",
    [ErrorCategory.NotImplemented]: "🚧",
  };
  return icons[category];
}

// =============================================================================
// Error Codes
// =============================================================================

/**
 * Validation error codes
 */
export const ValidationCodes = {
  INVALID_PATH: "VAL_001",
  INVALID_RANGE: "VAL_002",
  MISSING_FIELD: "VAL_003",
  INVALID_TYPE: "VAL_004",
} as const;

/**
 * File not found error codes
 */
export const NotFoundCodes = {
  FILE_NOT_FOUND: "NF_001",
  STREAM_NOT_FOUND: "NF_002",
  FRAME_NOT_FOUND: "NF_003",
  CODEC_NOT_FOUND: "NF_004",
} as const;

/**
 * Parse error codes
 */
export const ParseCodes = {
  INVALID_HEADER: "PARSE_001",
  INVALID_SYNTAX: "PARSE_002",
  TRUNCATED_DATA: "PARSE_003",
  UNKNOWN_CODEC: "PARSE_004",
} as const;

/**
 * I/O error codes
 */
export const IoCodes = {
  READ_FAILED: "IO_001",
  WRITE_FAILED: "IO_002",
  OPEN_FAILED: "IO_003",
  CREATE_FAILED: "IO_004",
} as const;

/**
 * Codec error codes
 */
export const CodecCodes = {
  UNSUPPORTED_CODEC: "CODEC_001",
  INVALID_BITSTREAM: "CODEC_002",
  DECODE_FAILED: "CODEC_003",
  MISSING_REFERENCE: "CODEC_004",
} as const;

// =============================================================================
// Application Error
// =============================================================================

/**
 * Unified application error interface
 */
export interface AppError {
  /** Error category */
  category: ErrorCategory;
  /** Error code for programmatic handling */
  code: string;
  /** User-friendly error message */
  message: string;
  /** Detailed error for debugging (optional) */
  details?: string;
  /** Stack trace (optional) */
  stack?: string;
}

/**
 * Type guard for AppError
 */
export function isAppError(obj: unknown): obj is AppError {
  const error = obj as AppError;
  return (
    error !== null &&
    typeof error === "object" &&
    typeof error.category === "string" &&
    typeof error.code === "string" &&
    typeof error.message === "string"
  );
}

// =============================================================================
// Error Factory Functions
// =============================================================================

/**
 * Create a new error
 */
export function createError(
  category: ErrorCategory,
  code: string,
  message: string,
  details?: string,
): AppError {
  const error: AppError = {
    category,
    code,
    message,
  };

  if (details) {
    error.details = details;
  }

  return error;
}

/**
 * Create a validation error
 */
export function validationError(
  code: string,
  message: string,
  details?: string,
): AppError {
  return createError(ErrorCategory.Validation, code, message, details);
}

/**
 * Create a not found error
 */
export function notFoundError(
  code: string,
  message: string,
  details?: string,
): AppError {
  return createError(ErrorCategory.NotFound, code, message, details);
}

/**
 * Create a permission error
 */
export function permissionError(
  code: string,
  message: string,
  details?: string,
): AppError {
  return createError(ErrorCategory.Permission, code, message, details);
}

/**
 * Create a parse error
 */
export function parseError(
  code: string,
  message: string,
  details?: string,
): AppError {
  return createError(ErrorCategory.Parse, code, message, details);
}

/**
 * Create an I/O error
 */
export function ioError(
  code: string,
  message: string,
  details?: string,
): AppError {
  return createError(ErrorCategory.Io, code, message, details);
}

/**
 * Create a codec error
 */
export function codecError(
  code: string,
  message: string,
  details?: string,
): AppError {
  return createError(ErrorCategory.Codec, code, message, details);
}

/**
 * Create an internal error
 */
export function internalError(
  code: string,
  message: string,
  details?: string,
): AppError {
  return createError(ErrorCategory.Internal, code, message, details);
}

// =============================================================================
// Convenience Error Creators
// =============================================================================

/**
 * Create an error for invalid file path
 */
export function invalidPathError(path: string, reason: string): AppError {
  return validationError(
    ValidationCodes.INVALID_PATH,
    `Invalid file path: ${reason}`,
    `Path: ${path}`,
  );
}

/**
 * Create an error for file not found
 */
export function fileNotFoundError(path: string): AppError {
  return notFoundError(NotFoundCodes.FILE_NOT_FOUND, `File not found: ${path}`);
}

/**
 * Create an error for invalid frame index
 */
export function invalidFrameIndexError(
  frame: number,
  maxFrame: number,
): AppError {
  return validationError(
    ValidationCodes.INVALID_RANGE,
    `Frame index ${frame} is out of range (max: ${maxFrame})`,
  );
}

/**
 * Create an error for invalid byte range
 */
export function invalidByteRangeError(
  start: number,
  end: number,
  fileSize: number,
): AppError {
  return validationError(
    ValidationCodes.INVALID_RANGE,
    `Invalid byte range: ${start}-${end} (file size: ${fileSize})`,
  );
}

/**
 * Create an error for parse failure
 */
export function parseFailureError(codec: string, context: string): AppError {
  return parseError(
    ParseCodes.INVALID_SYNTAX,
    `Failed to parse ${codec} bitstream`,
    context,
  );
}

/**
 * Create an error for unsupported codec
 */
export function unsupportedCodecError(codec: string): AppError {
  return codecError(
    CodecCodes.UNSUPPORTED_CODEC,
    `Unsupported codec: ${codec}`,
  );
}

/**
 * Create an error for decode failure
 */
export function decodeFailureError(frame: number, reason: string): AppError {
  return codecError(
    CodecCodes.DECODE_FAILED,
    `Failed to decode frame ${frame}`,
    reason,
  );
}

// =============================================================================
// Error Display for UI
// =============================================================================

/**
 * Format error for UI display
 */
export function formatErrorForUI(error: AppError): string {
  const icon = getCategoryIcon(error.category);
  const categoryName = getCategoryDisplayName(error.category);

  const baseMessage = `${icon} ${categoryName}\n\n${error.message}`;

  let helpText = "";
  switch (error.category) {
    case ErrorCategory.Validation:
      helpText = "\n\n💡 Please check your input and try again.";
      break;
    case ErrorCategory.NotFound:
      helpText = "\n\n💡 The requested resource could not be found.";
      break;
    case ErrorCategory.Permission:
      helpText = "\n\n💡 You don't have permission to access this resource.";
      break;
    case ErrorCategory.Parse:
      helpText =
        "\n\n💡 The file could not be parsed. It may be corrupted or in an unsupported format.";
      break;
    case ErrorCategory.Io:
      helpText =
        "\n\n💡 Please check if the file exists and you have permission to access it.";
      break;
    case ErrorCategory.Codec:
      helpText =
        "\n\n💡 This appears to be a codec-related issue. Try opening a different file.";
      break;
    case ErrorCategory.Internal:
      helpText =
        "\n\n💡 An unexpected error occurred. Please try again or report this issue.";
      break;
    case ErrorCategory.NotImplemented:
      helpText = "\n\n💡 This feature is not yet implemented.";
      break;
    case ErrorCategory.Network:
      helpText = "\n\n💡 Please check your internet connection.";
      break;
  }

  return baseMessage + helpText;
}

/**
 * Format error for notification (short version)
 */
export function formatErrorForNotification(error: AppError): string {
  const icon = getCategoryIcon(error.category);
  return `${icon} ${error.message}`;
}

/**
 * Format error for console/logging
 */
export function formatErrorForLogging(error: AppError): string {
  const parts = [`[${error.category}:${error.code}]`, error.message];

  if (error.details) {
    parts.push(`Details: ${error.details}`);
  }

  if (error.stack) {
    parts.push(`Stack: ${error.stack}`);
  }

  return parts.join(" | ");
}

// =============================================================================
// Error Conversion Utilities
// =============================================================================

/**
 * Convert JavaScript Error to AppError
 */
export function jsErrorToAppError(
  jsError: Error,
  category: ErrorCategory,
  code: string,
): AppError {
  return createError(
    category,
    code,
    jsError.message || "Unknown error",
    jsError.stack,
  );
}

/**
 * Convert fetch Response error to AppError
 */
export async function fetchErrorToAppError(
  response: Response,
): Promise<AppError> {
  const message = `HTTP ${response.status}: ${response.statusText}`;
  let details: string | undefined;

  try {
    const body = await response.text();
    if (body) {
      details = body.substring(0, 200); // Limit details length
    }
  } catch {
    // Ignore JSON parse errors
  }

  return createError(
    ErrorCategory.Network,
    `HTTP_${response.status}`,
    message,
    details,
  );
}

/**
 * Convert unknown error to AppError
 */
export function unknownToAppError(error: unknown): AppError {
  if (isAppError(error)) {
    return error;
  }

  if (error instanceof Error) {
    return jsErrorToAppError(error, ErrorCategory.Internal, "UNKNOWN_ERROR");
  }

  if (typeof error === "string") {
    return createError(ErrorCategory.Internal, "UNKNOWN_ERROR", error);
  }

  return createError(
    ErrorCategory.Internal,
    "UNKNOWN_ERROR",
    "An unknown error occurred",
  );
}

// =============================================================================
// Error Result Type
// =============================================================================

/**
 * Result type with AppError
 */
export type AppResult<T> = Result<T, AppError>;

/**
 * Create a success result
 */
export function ok<T>(value: T): AppResult<T> {
  return { ok: true, value };
}

/**
 * Create an error result
 */
export function err<T>(error: AppError): AppResult<T> {
  return { ok: false, error };
}

/**
 * Wrap a function that might throw in a try-catch and convert to AppResult
 */
export function safe<T>(fn: () => T): AppResult<T> {
  try {
    return ok(fn());
  } catch (error) {
    return err(unknownToAppError(error));
  }
}

/**
 * Wrap an async function that might throw in a try-catch and convert to AppResult
 */
export async function safeAsync<T>(
  fn: () => Promise<T>,
): Promise<AppResult<T>> {
  try {
    return ok(await fn());
  } catch (error) {
    return err(unknownToAppError(error));
  }
}

// =============================================================================
// Error Boundary Helpers
// =============================================================================

/**
 * Get error info from error boundary fallback
 */
export function getErrorInfo(error: unknown): AppError {
  return unknownToAppError(error);
}

/**
 * Log error to console with proper formatting
 */
export function logError(error: AppError, context?: string): void {
  const contextStr = context ? `[${context}] ` : "";
  console.error(`${contextStr}${formatErrorForLogging(error)}`);
}

/**
 * Log error and display to user
 */
export function handleAndDisplayError(error: AppError, context?: string): void {
  logError(error, context);
  // In a real app, you might also show a toast notification here
}

// =============================================================================
// Exports
// =============================================================================

export default {
  ErrorCategory,
  createError,
  validationError,
  notFoundError,
  parseError,
  ioError,
  codecError,
  invalidPathError,
  fileNotFoundError,
  invalidFrameIndexError,
  parseFailureError,
  unsupportedCodecError,
  decodeFailureError,
  formatErrorForUI,
  formatErrorForNotification,
  formatErrorForLogging,
  jsErrorToAppError,
  fetchErrorToAppError,
  unknownToAppError,
  ok,
  err,
  safe,
  safeAsync,
  getErrorInfo,
  logError,
  handleAndDisplayError,
};
