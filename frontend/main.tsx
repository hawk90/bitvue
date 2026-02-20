import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import "./theme/colors.css";
import "./theme/utilities.css";
import "./theme/utils.css";
import "./theme/buttons.css";
import "./theme/dropdowns.css";
import "./theme/forms.css";
import "./theme/tabs.css";
import { ThemeProvider } from "./contexts/ThemeContext";
import { initializeSystemMenu } from "./utils/menu";
import { createLogger } from "./utils/logger";
import { tauriLog } from "./utils/tauriLogger";

const logger = createLogger("main");

// Guard to prevent multiple console overrides due to HMR
const CONSOLE_OVERRIDE_KEY = "__bitvue_console_override_installed__";

if (!(window as any)[CONSOLE_OVERRIDE_KEY]) {
  (window as any)[CONSOLE_OVERRIDE_KEY] = true;

  // Override console methods to also send logs to terminal
  const originalLog = console.log;
  const originalInfo = console.info;
  const originalWarn = console.warn;
  const originalError = console.error;
  const originalDebug = console.debug;

  // Store original methods on window.console for tauriLogger to access
  (window.console as any)._originalLog = originalLog;
  (window.console as any)._originalInfo = originalInfo;
  (window.console as any)._originalWarn = originalWarn;
  (window.console as any)._originalError = originalError;
  (window.console as any)._originalDebug = originalDebug;

  console.log = (...args) => {
    tauriLog.log(String(args[0]), ...args.slice(1));
    originalLog(...args);
  };
  console.info = (...args) => {
    tauriLog.info(String(args[0]), ...args.slice(1));
    originalInfo(...args);
  };
  console.warn = (...args) => {
    tauriLog.warn(String(args[0]), ...args.slice(1));
    originalWarn(...args);
  };
  console.error = (...args) => {
    tauriLog.error(String(args[0]), ...args.slice(1));
    originalError(...args);
  };
  console.debug = (...args) => {
    tauriLog.debug(String(args[0]), ...args.slice(1));
    originalDebug(...args);
  };

  originalLog("[main] Console override installed (once)");

  // Global error handlers to catch all unhandled errors
  window.addEventListener("error", (event) => {
    originalError(
      "[GLOBAL ERROR]",
      event.message,
      event.filename,
      event.lineno,
      event.colno,
      event.error,
    );
  });

  window.addEventListener("unhandledrejection", (event) => {
    originalError("[UNHANDLED REJECTION]", event.reason);
  });

  originalLog("[main] Global error handlers installed");
} else {
  console.log("[main] Console override already installed, skipping");
}

logger.info("Starting app...");
console.log("[main] Script starting, document ready:", !!document);
console.log("[main] Root element:", document.getElementById("root"));

// Initialize system menu (macOS only)
initializeSystemMenu()
  .then(() => {
    logger.info("System menu initialized");
    console.log("[main] System menu initialized successfully");
  })
  .catch((err) => {
    logger.error("System menu init failed:", err);
    console.error("[main] System menu init failed:", err);
  });

logger.info("Rendering App...");
console.log("[main] Creating React root and rendering App");

const rootElement = document.getElementById("root");
if (!rootElement) {
  console.error("[main] CRITICAL: Root element not found!");
} else {
  console.log("[main] Root element found, creating ReactDOM root");
}

ReactDOM.createRoot(rootElement as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider defaultTheme="dark">
      <App />
    </ThemeProvider>
  </React.StrictMode>,
);

console.log("[main] ReactDOM.render called");
