import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig({
  // Relative asset paths -- required for Electron's `file://`-loaded renderer (win.loadFile()).
  // Vite's default `base: "/"` produces absolute paths like "/assets/index-*.js" that resolve
  // fine on a web server but 404 under file:// (Electron looks for them at the filesystem root).
  // Doesn't affect `npm run dev`'s dev server (still served over http://, base is a no-op there).
  base: "./",
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Vitest configuration
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
    css: true,
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
});
