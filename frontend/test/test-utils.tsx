import { render as rtlRender, RenderOptions } from "@testing-library/react";
import { ReactElement } from "react";
import { ModeProvider } from "../contexts/ModeContext";
import { SelectionProvider } from "../contexts/SelectionContext";
import {
  FrameDataProvider,
  FileStateProvider,
  CurrentFrameProvider,
} from "../contexts/StreamDataContext";
import { LayoutProvider } from "../contexts/LayoutContext";
import { ThemeProvider } from "../contexts/ThemeContext";

/**
 * All-in-one provider wrapper for tests
 */
function AllTheProviders({ children }: { children: React.ReactNode }) {
  return (
    <ThemeProvider>
      <LayoutProvider>
        <FrameDataProvider>
          <FileStateProvider>
            <CurrentFrameProvider>
              <SelectionProvider>
                <ModeProvider>{children}</ModeProvider>
              </SelectionProvider>
            </CurrentFrameProvider>
          </FileStateProvider>
        </FrameDataProvider>
      </LayoutProvider>
    </ThemeProvider>
  );
}

/**
 * Custom render function with all providers
 * Overrides default render to include providers automatically
 */
export function render(
  ui: ReactElement,
  options?: Omit<RenderOptions, "wrapper">,
) {
  return rtlRender(ui, { wrapper: AllTheProviders, ...options });
}

/**
 * Render without providers (use only if you need to test provider errors)
 */
export function renderWithoutProviders(
  ui: ReactElement,
  options?: RenderOptions,
) {
  return rtlRender(ui, options);
}

/**
 * Re-export testing library utilities
 */
export * from "@testing-library/react";
export { default as userEvent } from "@testing-library/user-event";

/**
 * Common test data generators
 */
export const mockFrames = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    pts: 0,
    poc: 0,
    key_frame: true,
    display_order: 0,
    coding_order: 0,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    pts: 1,
    poc: 1,
    key_frame: false,
    ref_frames: [0],
    display_order: 1,
    coding_order: 1,
  },
  {
    frame_index: 2,
    frame_type: "B",
    size: 20000,
    pts: 2,
    poc: 2,
    key_frame: false,
    ref_frames: [0, 1],
    display_order: 2,
    coding_order: 2,
  },
];

export const mockPanelConfig = {
  id: "test-panel",
  title: "Test Panel",
  component: () => <div>Test Panel Content</div>,
  icon: "test-icon",
};
