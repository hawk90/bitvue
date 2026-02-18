/**
 * Tauri Logger - Sends frontend logs to the terminal
 */

import { invoke } from "@tauri-apps/api/core";

let isTauri = false;

// Check if we're running in Tauri
try {
  isTauri = !!window.__TAURI__;
} catch {
  isTauri = false;
}

// Flag to prevent infinite logging loops
let isInsideLog = false;

/**
 * Send a log message to the terminal via Tauri
 */
function sendLog(level: string, message: string, ...args: unknown[]): void {
  // Prevent infinite loops from recursive logging
  if (isInsideLog) {
    return; // Silently drop recursive log calls
  }

  // Format args into message
  let formattedMessage = message;
  if (args.length > 0) {
    try {
      formattedMessage +=
        " " +
        args
          .map((arg) => {
            if (typeof arg === "string") return arg;
            try {
              return JSON.stringify(arg);
            } catch {
              return String(arg);
            }
          })
          .join(" ");
    } catch {
      formattedMessage += " " + args.map(String).join(" ");
    }
  }

  // Send to Tauri if available
  if (isTauri) {
    isInsideLog = true;
    invoke("frontend_log", { level, message: formattedMessage })
      .catch(() => {
        // Fall back to native console (not the overridden one) if invoke fails
        const nativeConsole = window.console as any;
        const originalLog = nativeConsole._originalLog || nativeConsole.log;
        originalLog(`[TAURI LOG FAILED] ${level}:`, message, ...args);
      })
      .finally(() => {
        isInsideLog = false;
      });
  }

  // Always log to console as well (use native console to avoid infinite loop)
  const nativeConsole = window.console as any;
  const originalError = nativeConsole._originalError || nativeConsole.error;
  const originalWarn = nativeConsole._originalWarn || nativeConsole.warn;
  const originalInfo = nativeConsole._originalInfo || nativeConsole.info;
  const originalDebug = nativeConsole._originalDebug || nativeConsole.debug;
  const originalLog = nativeConsole._originalLog || nativeConsole.log;

  switch (level) {
    case "error":
      originalError.call(nativeConsole, message, ...args);
      break;
    case "warn":
      originalWarn.call(nativeConsole, message, ...args);
      break;
    case "info":
      originalInfo.call(nativeConsole, message, ...args);
      break;
    case "debug":
      originalDebug.call(nativeConsole, message, ...args);
      break;
    default:
      originalLog.call(nativeConsole, message, ...args);
  }
}

/**
 * Tauri logger with the same interface as console
 */
export const tauriLog = {
  log: (message: string, ...args: unknown[]) =>
    sendLog("log", message, ...args),
  info: (message: string, ...args: unknown[]) =>
    sendLog("info", message, ...args),
  warn: (message: string, ...args: unknown[]) =>
    sendLog("warn", message, ...args),
  error: (message: string, ...args: unknown[]) =>
    sendLog("error", message, ...args),
  debug: (message: string, ...args: unknown[]) =>
    sendLog("debug", message, ...args),
};
