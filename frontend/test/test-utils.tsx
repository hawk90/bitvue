import { render as rtlRender, RenderOptions } from "@testing-library/react";
import { ReactElement } from "react";
import { ModeProvider } from "../contexts/ModeContext";
import { SelectionProvider } from "../contexts/SelectionContext";
import {
  FrameDataProvider,
  FileStateProvider,
} from "../contexts/StreamDataContext";
import { LayoutProvider } from "../contexts/LayoutContext";
import { ThemeProvider } from "../contexts/ThemeContext";

/**
 * Re-export testing library utilities. MUST come before this file's own `render` declaration
 * below -- found 2026-08-21 (axis-2 Tri-Sync completion) that with it declared *after* (as it
 * used to be), the star re-export's own `render` silently won over this file's wrapped one for
 * every consumer of `@/test/test-utils`'s `render` (verified: `utilsRender === rtlRenderRaw`
 * was `true`), meaning `AllTheProviders` was NEVER actually applied by any test -- it happened
 * not to matter until now because every test exercising a real context mocked that context
 * module directly instead of relying on this wrapper. Local named exports are supposed to take
 * precedence over a same-named star re-export per the ES module spec regardless of declaration
 * order, but this project's actual bundler pipeline evidently didn't honor that -- declaring this
 * first sidesteps the question rather than depending on it.
 */
export * from "@testing-library/react";

/**
 * All-in-one provider wrapper for tests
 */
function AllTheProviders({ children }: { children: React.ReactNode }) {
  return (
    <ThemeProvider>
      <LayoutProvider>
        <FrameDataProvider>
          <FileStateProvider>
            <SelectionProvider>
              <ModeProvider>{children}</ModeProvider>
            </SelectionProvider>
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
