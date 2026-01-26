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

const logger = createLogger('main');

logger.info('Starting app...');

// Initialize system menu (macOS only)
initializeSystemMenu().then(() => {
  logger.info('System menu initialized');
}).catch((err) => {
  logger.error('System menu init failed:', err);
});

logger.info('Rendering App...');

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider defaultTheme="dark">
      <App />
    </ThemeProvider>
  </React.StrictMode>,
);
