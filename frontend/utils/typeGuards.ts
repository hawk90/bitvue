/**
 * Type Guard Utilities
 *
 * Runtime type checking utilities for error handling and type safety
 */

/**
 * Check if a value is a non-null object
 */
export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Check if a value is a string
 */
export function isString(value: unknown): value is string {
  return typeof value === "string";
}

/**
 * Check if a value is a number (excluding NaN)
 */
export function isNumber(value: unknown): value is number {
  return typeof value === "number" && !isNaN(value);
}

/**
 * Check if a value is a boolean
 */
export function isBoolean(value: unknown): value is boolean {
  return typeof value === "boolean";
}

/**
 * Check if a value is an array
 */
export function isArray<T = unknown>(value: unknown): value is T[] {
  return Array.isArray(value);
}

/**
 * Check if a value is a function
 */
export function isFunction(
  value: unknown,
): value is (...args: unknown[]) => unknown {
  return typeof value === "function";
}

/**
 * Check if error is an Error object
 */
export function isError(value: unknown): value is Error {
  return (
    value instanceof Error ||
    (isObject(value) && "name" in value && "message" in value)
  );
}

/**
 * Check if error has a code property (for Tauri/System errors)
 */
export interface ErrorWithCode extends Error {
  code?: string | number;
}

export function isErrorWithCode(value: unknown): value is ErrorWithCode {
  return isError(value) && "code" in value;
}

/**
 * Check if value is a Tauri invoke error response
 */
export interface TauriError {
  success: false;
  error: string;
}

export function isTauriError(value: unknown): value is TauriError {
  return isObject(value) && value.success === false && "error" in value;
}

/**
 * Check if value is a successful Tauri response
 */
export interface TauriSuccess<T = unknown> {
  success: true;
  data?: T;
}

export function isTauriSuccess<T = unknown>(
  value: unknown,
): value is TauriSuccess<T> {
  return isObject(value) && value.success === true;
}

/**
 * Check if value is a valid frame index
 */
export function isValidFrameIndex(
  value: unknown,
  maxFrames: number,
): value is number {
  return (
    isNumber(value) &&
    Number.isInteger(value) &&
    value >= 0 &&
    value < maxFrames
  );
}

/**
 * Check if value is a valid file path string
 */
export function isValidFilePath(value: unknown): value is string {
  if (!isString(value)) return false;
  if (value.length === 0 || value.length > 4096) return false; // Reasonable path length limit
  // Check for invalid characters (Windows/Unix); allow colon for Windows drive letters
  const invalidChars = /[<>"|?*\x00-\x1f]/;
  return !invalidChars.test(value);
}

/**
 * Type guard for objects with a specific property
 */
export function hasProperty<K extends PropertyKey>(
  value: unknown,
  prop: K,
): value is Record<K, unknown> {
  return isObject(value) && prop in value;
}

/**
 * Get error message from various error types
 */
export function getErrorMessage(error: unknown): string {
  if (isError(error)) {
    return error.message;
  }
  if (isString(error)) {
    return error;
  }
  if (isTauriError(error)) {
    return error.error;
  }
  return "Unknown error occurred";
}

/**
 * Safe JSON parse with type guard
 */
export function tryParseJSON<T = unknown>(value: string): T | null {
  try {
    return JSON.parse(value) as T;
  } catch {
    return null;
  }
}

/**
 * Assert that a value is not null/undefined (for type narrowing)
 */
export function assertNotNil<T>(
  value: T | null | undefined,
  message?: string,
): asserts value is T {
  if (value === null || value === undefined) {
    throw new Error(message || `Expected value to not be null/undefined`);
  }
}
