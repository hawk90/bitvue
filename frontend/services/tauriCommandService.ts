/**
 * Tauri Command Service
 *
 * Abstraction layer for Tauri command invocations.
 * Provides centralized error handling, type safety, and logging.
 */

import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "../utils/logger";

const logger = createLogger("TauriCommandService");

/**
 * Service error for Tauri command failures
 */
export class TauriCommandError extends Error {
  constructor(
    public command: string,
    message: string,
    public originalError?: unknown,
  ) {
    super(`[Tauri:${command}] ${message}`);
    this.name = "TauriCommandError";
  }
}

/**
 * Result type for command responses
 */
export type CommandResult<T> =
  | { success: true; data: T }
  | { success: false; error: TauriCommandError };

/**
 * Options for command invocation
 */
export interface CommandInvokeOptions {
  /** Retry the command on failure */
  retry?: number;
  /** Timeout in milliseconds */
  timeout?: number;
  /** Log the request/response */
  log?: boolean;
}

/**
 * Default options for command invocation
 */
const DEFAULT_OPTIONS: CommandInvokeOptions = {
  retry: 0,
  timeout: 30000,
  log: true,
};

/**
 * Tauri Command Service
 *
 * Provides a centralized interface for invoking Tauri commands
 * with consistent error handling, logging, and optional retry logic.
 */
class TauriCommandServiceClass {
  private commandLatency = new Map<string, number[]>();

  /**
   * Invoke a Tauri command with error handling and logging
   *
   * @param command - Command name to invoke
   * @param args - Command arguments
   * @param options - Invocation options
   * @returns Promise that resolves with command result
   */
  async invoke<T>(
    command: string,
    args: Record<string, unknown> = {},
    options: CommandInvokeOptions = {},
  ): Promise<T> {
    const opts = { ...DEFAULT_OPTIONS, ...options };
    const startTime = performance.now();

    // Log request
    if (opts.log) {
      logger.debug(`→ ${command}`, args);
    }

    let lastError: unknown;

    // Retry logic
    for (let attempt = 0; attempt <= (opts.retry ?? 0); attempt++) {
      try {
        const result = await invoke<T>(command, args);
        const latency = performance.now() - startTime;

        // Log success
        if (opts.log) {
          logger.debug(`← ${command}`, { latency: `${latency.toFixed(2)}ms` });
        }

        // Track latency
        this.trackLatency(command, latency);

        return result;
      } catch (error) {
        lastError = error;

        // Don't retry on certain errors
        if (this.isNonRetriableError(error)) {
          break;
        }

        if (attempt < (opts.retry ?? 0)) {
          logger.warn(
            `↻ ${command} failed, retrying (${attempt + 1}/${opts.retry})...`,
          );
          // Exponential backoff
          await this.delay(Math.pow(2, attempt) * 1000);
        }
      }
    }

    // All retries exhausted
    const errorMessage = this.extractErrorMessage(lastError);
    const commandError = new TauriCommandError(
      command,
      errorMessage,
      lastError,
    );

    logger.error(`✗ ${command} failed:`, errorMessage);
    throw commandError;
  }

  /**
   * Invoke a command and return a CommandResult type
   * Never throws, always returns a result object
   */
  async safeInvoke<T>(
    command: string,
    args: Record<string, unknown> = {},
    options: CommandInvokeOptions = {},
  ): Promise<CommandResult<T>> {
    try {
      const data = await this.invoke<T>(command, args, options);
      return { success: true, data };
    } catch (error) {
      if (error instanceof TauriCommandError) {
        return { success: false, error };
      }
      return {
        success: false,
        error: new TauriCommandError(command, String(error), error),
      };
    }
  }

  /**
   * Check if an error is non-retriable
   */
  private isNonRetriableError(error: unknown): boolean {
    if (error instanceof Error) {
      // Validation errors, authentication errors, etc.
      const nonRetriablePatterns = [
        "validation",
        "invalid",
        "unauthorized",
        "forbidden",
        "not found",
        "permission denied",
      ];

      const message = error.message.toLowerCase();
      return nonRetriablePatterns.some((pattern) => message.includes(pattern));
    }
    return false;
  }

  /**
   * Extract error message from error object
   */
  private extractErrorMessage(error: unknown): string {
    if (error instanceof Error) {
      return error.message;
    }
    if (typeof error === "string") {
      return error;
    }
    return "Unknown error";
  }

  /**
   * Track command latency for metrics
   */
  private trackLatency(command: string, latency: number): void {
    if (!this.commandLatency.has(command)) {
      this.commandLatency.set(command, []);
    }

    const latencies = this.commandLatency.get(command)!;
    latencies.push(latency);

    // Keep only last 100 measurements
    if (latencies.length > 100) {
      latencies.shift();
    }
  }

  /**
   * Get latency statistics for a command
   */
  getLatencyStats(
    command: string,
  ): { avg: number; min: number; max: number; count: number } | null {
    const latencies = this.commandLatency.get(command);
    if (!latencies || latencies.length === 0) {
      return null;
    }

    return {
      avg: latencies.reduce((a, b) => a + b, 0) / latencies.length,
      min: Math.min(...latencies),
      max: Math.max(...latencies),
      count: latencies.length,
    };
  }

  /**
   * Get latency statistics for all commands
   */
  getAllLatencyStats(): Map<
    string,
    { avg: number; min: number; max: number; count: number }
  > {
    const stats = new Map<
      string,
      { avg: number; min: number; max: number; count: number }
    >();

    for (const [command, latencies] of this.commandLatency.entries()) {
      if (latencies.length > 0) {
        stats.set(command, {
          avg: latencies.reduce((a, b) => a + b, 0) / latencies.length,
          min: Math.min(...latencies),
          max: Math.max(...latencies),
          count: latencies.length,
        });
      }
    }

    return stats;
  }

  /**
   * Clear latency statistics
   */
  clearLatencyStats(): void {
    this.commandLatency.clear();
  }

  /**
   * Delay utility for retry backoff
   */
  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

/**
 * Singleton instance of the Tauri Command Service
 */
export const TauriCommandService = new TauriCommandServiceClass();

/**
 * Convenience function to invoke a command
 * Same as TauriCommandService.invoke but shorter
 */
export function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
  options?: CommandInvokeOptions,
): Promise<T> {
  return TauriCommandService.invoke<T>(command, args, options);
}

/**
 * Convenience function to safely invoke a command
 * Same as TauriCommandService.safeInvoke but shorter
 */
export function safeInvokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
  options?: CommandInvokeOptions,
): Promise<CommandResult<T>> {
  return TauriCommandService.safeInvoke<T>(command, args, options);
}
