import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./test/setup.ts"],
    include: ["tests/**/*.{test,spec}.{ts,tsx}"],
    exclude: ["node_modules", "dist", "src-tauri"],
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      exclude: [
        "node_modules/",
        "dist/",
        "src-tauri/",
        "**/*.test.{ts,tsx}",
        "**/*.spec.{ts,tsx}",
        "test/",
        "tests/",
      ],
    },
  },
  resolve: {
    alias: [
      // Force React to use the local frontend copy (React 18) to avoid conflicts
      // with workspace-level React 19 in bitvue/node_modules
      {
        find: "react/jsx-dev-runtime",
        replacement: path.resolve(
          __dirname,
          "node_modules/react/jsx-dev-runtime.js",
        ),
      },
      {
        find: "react/jsx-runtime",
        replacement: path.resolve(
          __dirname,
          "node_modules/react/jsx-runtime.js",
        ),
      },
      {
        find: "react-dom/client",
        replacement: path.resolve(
          __dirname,
          "node_modules/react-dom/client.js",
        ),
      },
      {
        find: "react-dom",
        replacement: path.resolve(__dirname, "node_modules/react-dom"),
      },
      {
        find: "react",
        replacement: path.resolve(__dirname, "node_modules/react"),
      },
      // Fix @ alias to point to frontend root (not ./src which doesn't exist)
      { find: "@", replacement: path.resolve(__dirname, ".") },

      // Player/views components (tests import as "../Foo" but components are in Player/views/)
      {
        find: /^\.\.\/AV1FeaturesView$/,
        replacement: path.resolve(
          __dirname,
          "components/Player/views/AV1FeaturesView",
        ),
      },
      {
        find: /^\.\.\/CodingFlowView$/,
        replacement: path.resolve(
          __dirname,
          "components/Player/views/CodingFlowView",
        ),
      },
      {
        find: /^\.\.\/DualVideoView$/,
        replacement: path.resolve(
          __dirname,
          "components/Player/views/DualVideoView",
        ),
      },
      {
        find: /^\.\.\/DeblockingView$/,
        replacement: path.resolve(
          __dirname,
          "components/Player/views/DeblockingView",
        ),
      },
      {
        find: /^\.\.\/ResidualsView$/,
        replacement: path.resolve(
          __dirname,
          "components/Player/views/ResidualsView",
        ),
      },

      // Filmstrip/views components
      {
        find: /^\.\.\/BPyramidTimeline$/,
        replacement: path.resolve(
          __dirname,
          "components/Filmstrip/views/BPyramidTimeline",
        ),
      },
      {
        find: /^\.\.\/BPyramidView$/,
        replacement: path.resolve(
          __dirname,
          "components/Filmstrip/views/BPyramidView",
        ),
      },
      {
        find: /^\.\.\/ThumbnailsView$/,
        replacement: path.resolve(
          __dirname,
          "components/Filmstrip/views/ThumbnailsView",
        ),
      },
      {
        find: /^\.\.\/FrameSizesView$/,
        replacement: path.resolve(
          __dirname,
          "components/Filmstrip/views/FrameSizesView",
        ),
      },

      // panels/UnitHexPanel components
      {
        find: /^\.\.\/DpbViewTab$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/UnitHexPanel/DpbViewTab",
        ),
      },
      {
        find: /^\.\.\/FrameViewTab$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/UnitHexPanel/FrameViewTab",
        ),
      },
      {
        find: /^\.\.\/HexViewTab$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/UnitHexPanel/HexViewTab",
        ),
      },
      {
        find: /^\.\.\/UnitHexPanel$/,
        replacement: path.resolve(__dirname, "components/panels/UnitHexPanel"),
      },

      // panels/SyntaxDetailPanel components (with subpath capture)
      {
        find: /^\.\.\/SyntaxDetailPanel\/(.+)$/,
        replacement:
          path.resolve(__dirname, "components/panels/SyntaxDetailPanel") +
          "/$1",
      },
      {
        find: /^\.\.\/SyntaxDetailPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/SyntaxDetailPanel",
        ),
      },

      // panels/YuvViewerPanel components (with subpath capture)
      {
        find: /^\.\.\/YuvViewerPanel\/(.+)$/,
        replacement:
          path.resolve(__dirname, "components/panels/YuvViewerPanel") + "/$1",
      },
      {
        find: /^\.\.\/YuvViewerPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/YuvViewerPanel",
        ),
      },

      // charts components
      {
        find: /^\.\.\/BarChart$/,
        replacement: path.resolve(__dirname, "components/charts/BarChart"),
      },
      {
        find: /^\.\.\/LineChart$/,
        replacement: path.resolve(__dirname, "components/charts/LineChart"),
      },

      // common components
      {
        find: /^\.\.\/TabContainer$/,
        replacement: path.resolve(__dirname, "components/common/TabContainer"),
      },

      // components directly in panels/
      {
        find: /^\.\.\/BitrateGraphPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/BitrateGraphPanel",
        ),
      },
      {
        find: /^\.\.\/BitViewPanel$/,
        replacement: path.resolve(__dirname, "components/panels/BitViewPanel"),
      },
      {
        find: /^\.\.\/DetailsPanel$/,
        replacement: path.resolve(__dirname, "components/panels/DetailsPanel"),
      },
      {
        find: /^\.\.\/DiagnosticsPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/DiagnosticsPanel",
        ),
      },
      {
        find: /^\.\.\/DockableLayout$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/DockableLayout",
        ),
      },
      {
        find: /^\.\.\/InfoPanel$/,
        replacement: path.resolve(__dirname, "components/panels/InfoPanel"),
      },
      {
        find: /^\.\.\/QualityComparisonPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/QualityComparisonPanel",
        ),
      },
      {
        find: /^\.\.\/QualityMetricsPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/QualityMetricsPanel",
        ),
      },
      {
        find: /^\.\.\/RDCurvesPanel$/,
        replacement: path.resolve(__dirname, "components/panels/RDCurvesPanel"),
      },
      {
        find: /^\.\.\/ReferenceGraphPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/ReferenceGraphPanel",
        ),
      },
      {
        find: /^\.\.\/SelectionInfoPanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/SelectionInfoPanel",
        ),
      },
      {
        find: /^\.\.\/StreamTreePanel$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/StreamTreePanel",
        ),
      },

      // Subpath imports like ../panels/X → components/panels/X
      {
        find: /^\.\.\/panels\/(.+)$/,
        replacement: path.resolve(__dirname, "components/panels") + "/$1",
      },

      // panels/ components (direct children of panels/)
      {
        find: /^\.\.\/OverlayRenderer$/,
        replacement: path.resolve(
          __dirname,
          "components/panels/OverlayRenderer",
        ),
      },
      {
        find: /^\.\.\/PanelBase$/,
        replacement: path.resolve(__dirname, "components/panels/PanelBase"),
      },

      // Flat components directly in components/
      {
        find: /^\.\.\/BookmarksPanel$/,
        replacement: path.resolve(__dirname, "components/BookmarksPanel"),
      },
      {
        find: /^\.\.\/DebugPanel$/,
        replacement: path.resolve(__dirname, "components/DebugPanel"),
      },
      {
        find: /^\.\.\/EnhancedView$/,
        replacement: path.resolve(__dirname, "components/EnhancedView"),
      },
      {
        find: /^\.\.\/ErrorBoundary$/,
        replacement: path.resolve(__dirname, "components/ErrorBoundary"),
      },
      {
        find: /^\.\.\/ErrorDialog$/,
        replacement: path.resolve(__dirname, "components/ErrorDialog"),
      },
      {
        find: /^\.\.\/ExportDialog$/,
        replacement: path.resolve(__dirname, "components/ExportDialog"),
      },
      {
        find: /^\.\.\/Filmstrip$/,
        replacement: path.resolve(__dirname, "components/Filmstrip"),
      },
      {
        find: /^\.\.\/Filmstrip\/(.+)$/,
        replacement: path.resolve(__dirname, "components/Filmstrip") + "/$1",
      },
      {
        find: /^\.\.\/FilmstripDropdown$/,
        replacement: path.resolve(__dirname, "components/FilmstripDropdown"),
      },
      {
        find: /^\.\.\/FilmstripTooltip$/,
        replacement: path.resolve(__dirname, "components/FilmstripTooltip"),
      },
      {
        find: /^\.\.\/FrameNavigationToolbar$/,
        replacement: path.resolve(
          __dirname,
          "components/FrameNavigationToolbar",
        ),
      },
      {
        find: /^\.\.\/FrameSizesLegend$/,
        replacement: path.resolve(__dirname, "components/FrameSizesLegend"),
      },
      {
        find: /^\.\.\/GraphUtils$/,
        replacement: path.resolve(__dirname, "components/GraphUtils"),
      },
      {
        find: /^\.\.\/KeyboardShortcutsDialog$/,
        replacement: path.resolve(
          __dirname,
          "components/KeyboardShortcutsDialog",
        ),
      },
      {
        find: /^\.\.\/LayoutToolbar$/,
        replacement: path.resolve(__dirname, "components/LayoutToolbar"),
      },
      {
        find: /^\.\.\/Loading$/,
        replacement: path.resolve(__dirname, "components/Loading"),
      },
      {
        find: /^\.\.\/MinimapView$/,
        replacement: path.resolve(__dirname, "components/MinimapView"),
      },
      {
        find: /^\.\.\/ReferenceLinesOverlay$/,
        replacement: path.resolve(
          __dirname,
          "components/ReferenceLinesOverlay",
        ),
      },
      {
        find: /^\.\.\/StatusBar$/,
        replacement: path.resolve(__dirname, "components/StatusBar"),
      },
      {
        find: /^\.\.\/Timeline$/,
        replacement: path.resolve(__dirname, "components/Timeline"),
      },
      {
        find: /^\.\.\/TimelineCursor$/,
        replacement: path.resolve(__dirname, "components/TimelineCursor"),
      },
      {
        find: /^\.\.\/TimelineFilmstrip$/,
        replacement: path.resolve(__dirname, "components/TimelineFilmstrip"),
      },
      {
        find: /^\.\.\/TimelineHeader$/,
        replacement: path.resolve(__dirname, "components/TimelineHeader"),
      },
      {
        find: /^\.\.\/TimelineThumbnails$/,
        replacement: path.resolve(__dirname, "components/TimelineThumbnails"),
      },
      {
        find: /^\.\.\/TimelineTooltip$/,
        replacement: path.resolve(__dirname, "components/TimelineTooltip"),
      },
      {
        find: /^\.\.\/TitleBar$/,
        replacement: path.resolve(__dirname, "components/TitleBar"),
      },
      {
        find: /^\.\.\/Tooltip$/,
        replacement: path.resolve(__dirname, "components/Tooltip"),
      },
      {
        find: /^\.\.\/VirtualizedFilmstrip$/,
        replacement: path.resolve(__dirname, "components/VirtualizedFilmstrip"),
      },
      {
        find: /^\.\.\/WelcomeScreen$/,
        replacement: path.resolve(__dirname, "components/WelcomeScreen"),
      },
    ],
  },
});
