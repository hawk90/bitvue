/**
 * Console logger - routes a log call to the native (pre-override) console method.
 *
 * `main.tsx` replaces `console.*` and stores the originals on `window.console._original*`; this
 * helper reaches those originals so it can't recurse into the override.
 */

interface NativeConsole extends Console {
  _originalLog?: (...args: unknown[]) => void;
  _originalError?: (...args: unknown[]) => void;
  _originalWarn?: (...args: unknown[]) => void;
  _originalInfo?: (...args: unknown[]) => void;
  _originalDebug?: (...args: unknown[]) => void;
}

function sendLog(level: string, message: string, ...args: unknown[]): void {
  // Use the native console (not the overridden one) to avoid an infinite loop
  const nativeConsole = window.console as NativeConsole;
  const originalError = nativeConsole._originalError ?? nativeConsole.error;
  const originalWarn = nativeConsole._originalWarn ?? nativeConsole.warn;
  const originalInfo = nativeConsole._originalInfo ?? nativeConsole.info;
  const originalDebug = nativeConsole._originalDebug ?? nativeConsole.debug;
  const originalLog = nativeConsole._originalLog ?? nativeConsole.log;

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
 * Logger with the same interface as console
 */
export const consoleLog = {
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
