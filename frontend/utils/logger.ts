/**
 * Centralized logging utility
 *
 * Provides consistent logging with levels and optional context prefixes.
 * Logs can be enabled/disabled via environment variables or runtime configuration.
 */

export type LogLevel = 'debug' | 'info' | 'warn' | 'error' | 'none';

interface ImportMetaEnv {
  VITE_LOG_LEVEL?: string;
  DEV?: boolean;
}

interface ImportMeta {
  env?: ImportMetaEnv;
}

interface ImportMetaWithEnv extends ImportMeta {
  env?: ImportMetaEnv;
  hot?: unknown;
}

// Get import.meta with proper typing
const importMetaObj = import.meta as ImportMetaWithEnv;

// Get log level from environment or default to 'info' in development, 'error' in production
const getDefaultLogLevel = (): LogLevel => {
  if (importMetaObj.env) {
    const envLogLevel = importMetaObj.env.VITE_LOG_LEVEL?.toLowerCase() as LogLevel;
    if (['debug', 'info', 'warn', 'error', 'none'].includes(envLogLevel)) {
      return envLogLevel;
    }
  }
  return importMetaObj.env?.DEV ? 'info' : 'error';
};

let currentLogLevel: LogLevel = getDefaultLogLevel();
let contextPrefix: string = '';

/**
 * Set the current log level
 */
export function setLogLevel(level: LogLevel): void {
  currentLogLevel = level;
}

/**
 * Get the current log level
 */
export function getLogLevel(): LogLevel {
  return currentLogLevel;
}

/**
 * Set a global context prefix for all log messages
 */
export function setLogContext(prefix: string): void {
  contextPrefix = prefix ? `[${prefix}] ` : '';
}

/**
 * Create a logger with a specific context prefix
 */
export function createLogger(context: string): Logger {
  return {
    debug: (msg: string, ...args: unknown[]) => logDebug(msg, context, ...args),
    info: (msg: string, ...args: unknown[]) => logInfo(msg, context, ...args),
    warn: (msg: string, ...args: unknown[]) => logWarn(msg, context, ...args),
    error: (msg: string, ...args: unknown[]) => logError(msg, context, ...args),
  };
}

export interface Logger {
  debug: (msg: string, ...args: unknown[]) => void;
  info: (msg: string, ...args: unknown[]) => void;
  warn: (msg: string, ...args: unknown[]) => void;
  error: (msg: string, ...args: unknown[]) => void;
}

function shouldLog(level: LogLevel): boolean {
  const levels: LogLevel[] = ['debug', 'info', 'warn', 'error', 'none'];
  return levels.indexOf(level) >= levels.indexOf(currentLogLevel);
}

function formatMessage(msg: string, context?: string): string {
  const parts = [];
  if (contextPrefix) parts.push(contextPrefix);
  if (context) parts.push(`[${context}]`);
  parts.push(msg);
  return parts.join(' ');
}

function logDebug(msg: string, context?: string, ...args: unknown[]): void {
  if (shouldLog('debug')) {
    console.log(formatMessage(msg, context), ...args);
  }
}

function logInfo(msg: string, context?: string, ...args: unknown[]): void {
  if (shouldLog('info')) {
    console.info(formatMessage(msg, context), ...args);
  }
}

function logWarn(msg: string, context?: string, ...args: unknown[]): void {
  if (shouldLog('warn')) {
    console.warn(formatMessage(msg, context), ...args);
  }
}

function logError(msg: string, context?: string, ...args: unknown[]): void {
  if (shouldLog('error')) {
    console.error(formatMessage(msg, context), ...args);
  }
}

/**
 * Default logger instance
 */
export const logger: Logger = {
  debug: (msg: string, ...args: unknown[]) => logDebug(msg, undefined, ...args),
  info: (msg: string, ...args: unknown[]) => logInfo(msg, undefined, ...args),
  warn: (msg: string, ...args: unknown[]) => logWarn(msg, undefined, ...args),
  error: (msg: string, ...args: unknown[]) => logError(msg, undefined, ...args),
};

/**
 * Convenience methods for direct import
 */
export const debug = logger.debug;
export const info = logger.info;
export const warn = logger.warn;
export const error = logger.error;
