import React from "react";
import ReactDOM from "react-dom/client";
// Real @vscode/codicons stylesheet -- provides every `.codicon-<name>::before { content: "\eXXX" }`
// glyph mapping. App.css previously only had a bespoke `@font-face` + bare `.codicon` sizing rule
// with NO glyph mappings at all, so every codicon icon in the app (close buttons, GitHub icon,
// keyboard icon, etc.) rendered as an empty invisible box -- found 2026-08-21 via a WelcomeScreen
// screenshot showing one icon slot rendering blank next to a working inline-SVG icon. Imported
// first so App.css's `.codicon[class*='codicon-']` font-size override (kept at 14px, this
// package's own default is 16px) still wins via source order.
import "@vscode/codicons/dist/codicon.css";
import App from "./App";

// Extend Window to allow the guard that installs the global error handlers only once
interface BitvueWindow extends Window {
  __bitvue_global_error_handlers_installed__?: boolean;
}
import "./index.css";
import "./theme/colors.css";
import "./theme/utilities.css";
import "./theme/utils.css";
import "./theme/buttons.css";
import "./theme/dropdowns.css";
import "./theme/forms.css";
import "./theme/tabs.css";
import { ThemeProvider } from "./contexts/ThemeContext";
import { LayoutProvider } from "./contexts/LayoutContext";
import { createLogger } from "./utils/logger";

const logger = createLogger("main");

// Guard to prevent installing the global error handlers twice (HMR re-runs this module).
//
// This block used to replace console.log/info/warn/error/debug so logs could also be forwarded to
// the terminal through Tauri's frontend_log command. That forwarding path never runs under
// Electron (the main process already forwards renderer console output, see createWindow's
// `console-message` listener), so the override only made every console call print twice (#609).
const GLOBAL_HANDLERS_KEY = "__bitvue_global_error_handlers_installed__";

const bitvueWindow = window as BitvueWindow;
if (!bitvueWindow[GLOBAL_HANDLERS_KEY]) {
  bitvueWindow[GLOBAL_HANDLERS_KEY] = true;

  // Global error handlers to catch all unhandled errors
  window.addEventListener("error", (event) => {
    console.error(
      "[GLOBAL ERROR]",
      event.message,
      event.filename,
      event.lineno,
      event.colno,
      event.error,
    );
  });

  window.addEventListener("unhandledrejection", (event) => {
    console.error("[UNHANDLED REJECTION]", event.reason);
  });
} else {
  // Handlers already installed (HMR re-run) — skip silently
}

logger.info("Starting app...");

// macOS's native menu bar is now built in the main process (bitvue-desktop/electron/main.ts's
// installNativeMacMenu) via real Electron APIs. The renderer-side initializeSystemMenu() used to
// live here, but it called a renderer-side menu API that no longer exists after the Electron
// migration (2026-08-08) -- so it threw an unhandled rejection on every
// single launch and never actually built a working menu. See installNativeMacMenu's doc for the
// full story (found because macOS had *no* working menu at all as a result -- File, Export, Mode
// switching, all silently unreachable).

logger.info("Rendering App...");

const rootElement = document.getElementById("root");
if (!rootElement) throw new Error("Root element #root not found");

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <ThemeProvider defaultTheme="dark">
      <LayoutProvider>
        <App />
      </LayoutProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
