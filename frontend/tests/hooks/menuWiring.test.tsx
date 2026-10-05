/**
 * Menu → renderer wiring (UI-040 / UI-051 / DEC-008 / OVL-043 / OVL-045).
 *
 * Covers both menu front-ends:
 * - macOS native menu (bitvue-desktop/electron/nativeMenu.ts) — simulated by dispatching the same
 *   `window` CustomEvents it sends (`MENU_EVENTS.*`, shared constants).
 * - Windows/Linux custom TitleBar — real clicks on its menu items.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  act,
  fireEvent,
  render,
  renderHook,
  screen,
} from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import type { ReactNode } from "react";
import {
  MENU_COLOR_SPACES,
  MENU_EVENTS,
  MENU_MODES,
} from "../../../bitvue-desktop/electron/menuEvents";
import { MODES, ModeProvider, useMode } from "@/contexts/ModeContext";
import { ThemeProvider, useTheme } from "@/contexts/ThemeContext";
import { useModeMenuEvents } from "@/hooks/useModeMenuEvents";
import { useThemeMenuEvents } from "@/hooks/useThemeMenuEvents";
import { useFileMenuEvents } from "@/hooks/useFileMenuEvents";
import { TitleBar } from "@/components/TitleBar";

vi.mock("@/services/electronBridgeService", () => ({
  closeWindow: vi.fn(),
  showOpenDialog: vi.fn(),
  minimizeWindow: vi.fn(),
  toggleMaximizeWindow: vi.fn(),
}));

const NATIVE_MENU_SRC = readFileSync(
  path.resolve(__dirname, "../../../bitvue-desktop/electron/nativeMenu.ts"),
  "utf8",
);

function dispatch(event: string, detail?: unknown) {
  act(() => {
    window.dispatchEvent(
      detail === undefined
        ? new CustomEvent(event)
        : new CustomEvent(event, { detail }),
    );
  });
}

describe("shared menu constants", () => {
  it("MENU_MODES mirrors ModeContext MODES (keys + labels, same order)", () => {
    expect(MENU_MODES.map((m) => [m.key, m.label])).toEqual(
      MODES.map((m) => [m.key, m.label]),
    );
  });

  it("exposes AV1 Features and Residuals in the Mode menu", () => {
    const keys = MENU_MODES.map((m) => m.key);
    expect(keys).toContain("av1-features");
    expect(keys).toContain("residuals");
  });

  it("nativeMenu.ts uses the shared constants, not hand-typed channel literals", () => {
    expect(NATIVE_MENU_SRC).toContain('from "./menuEvents.js"');
    expect(NATIVE_MENU_SRC).toContain("MENU_EVENTS.modeChange");
    expect(NATIVE_MENU_SRC).toContain("MENU_EVENTS.keyboardShortcuts");
    expect(NATIVE_MENU_SRC).toContain("MENU_EVENTS.themeChange");
    expect(NATIVE_MENU_SRC).toContain("MENU_COLOR_SPACES");
    for (const name of Object.values(MENU_EVENTS)) {
      expect(NATIVE_MENU_SRC).not.toContain(`"${name}"`);
    }
    // the old mismatched values must be gone
    expect(NATIVE_MENU_SRC).not.toMatch(/"menu-shortcuts"|"coding"|"qp"|"mv"/);
  });
});

describe("Mode menu (macOS native → ModeContext.setMode)", () => {
  const wrapper = ({ children }: { children: ReactNode }) => (
    <ModeProvider>{children}</ModeProvider>
  );
  const useHarness = () => {
    const mode = useMode();
    useModeMenuEvents(mode.setMode);
    return mode;
  };

  it.each(["av1-features", "residuals", "deblocking", "overview"])(
    "switches to %s",
    (key) => {
      const { result } = renderHook(useHarness, { wrapper });
      if (key === "overview") dispatch(MENU_EVENTS.modeChange, "residuals");
      dispatch(MENU_EVENTS.modeChange, key);
      expect(result.current.currentMode).toBe(key);
    },
  );

  it("switches to codec modes once a codec is active", () => {
    const { result } = renderHook(useHarness, { wrapper });
    act(() => result.current.setActiveCodec("AV1"));
    dispatch(MENU_EVENTS.modeChange, "coding-flow");
    expect(result.current.currentMode).toBe("coding-flow");
  });

  it("ignores the old non-MODES values and malformed details", () => {
    const { result } = renderHook(useHarness, { wrapper });
    dispatch(MENU_EVENTS.modeChange, "av1-features");
    dispatch(MENU_EVENTS.modeChange, "coding");
    dispatch(MENU_EVENTS.modeChange, 42);
    dispatch(MENU_EVENTS.modeChange);
    expect(result.current.currentMode).toBe("av1-features");
  });

  it("stops listening after unmount", () => {
    const setMode = vi.fn();
    const { unmount } = renderHook(() => useModeMenuEvents(setMode));
    unmount();
    dispatch(MENU_EVENTS.modeChange, "residuals");
    expect(setMode).not.toHaveBeenCalled();
  });
});

describe("Help > Keyboard Shortcuts (macOS native)", () => {
  it("opens the shortcuts dialog on the channel the native menu sends", () => {
    const setShowShortcuts = vi.fn();
    renderHook(() =>
      useFileMenuEvents({
        openFileAtPath: vi.fn(),
        handleOpenFile: vi.fn(),
        handleCloseFile: vi.fn(),
        handleOpenDependentFile: vi.fn(),
        exportEvidenceBundle: vi.fn(),
        setShowExportDialog: vi.fn(),
        setShowShortcuts,
        setPendingYuvPath: vi.fn(),
      }),
    );
    dispatch(MENU_EVENTS.keyboardShortcuts);
    expect(MENU_EVENTS.keyboardShortcuts).toBe("menu-keyboard-shortcuts");
    expect(setShowShortcuts).toHaveBeenCalledWith(true);
  });
});

describe("Theme menu", () => {
  function ThemeHarness() {
    const { theme, setTheme } = useTheme();
    useThemeMenuEvents(setTheme);
    return <span data-testid="theme">{theme}</span>;
  }

  afterEach(() => {
    document.documentElement.removeAttribute("data-theme");
  });

  it("macOS native event switches ThemeContext and data-theme", () => {
    render(
      <ThemeProvider defaultTheme="dark">
        <ThemeHarness />
      </ThemeProvider>,
    );
    dispatch(MENU_EVENTS.themeChange, "light");
    expect(screen.getByTestId("theme")).toHaveTextContent("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    dispatch(MENU_EVENTS.themeChange, "bogus");
    expect(screen.getByTestId("theme")).toHaveTextContent("light");
  });

  it("TitleBar Dark/Light items are enabled and switch the theme", () => {
    render(
      <ThemeProvider defaultTheme="dark">
        <ThemeHarness />
        <TitleBar fileName="x" onOpenFile={vi.fn()} />
      </ThemeProvider>,
    );
    fireEvent.mouseEnter(screen.getByText("Options"));
    const light = screen.getByText("Light Theme").closest(".menu-item")!;
    expect(light).not.toHaveClass("menu-item-disabled");
    fireEvent.click(light);
    expect(screen.getByTestId("theme")).toHaveTextContent("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");

    fireEvent.mouseEnter(screen.getByText("Options"));
    fireEvent.click(screen.getByText("Dark Theme"));
    expect(screen.getByTestId("theme")).toHaveTextContent("dark");
  });
});

describe("TitleBar Options > Color Space", () => {
  let received: string[];
  const listeners = Object.values(MENU_EVENTS).map((name) => {
    const fn = (e: Event) => received.push(e.type);
    return [name, fn] as const;
  });

  beforeEach(() => {
    received = [];
    for (const [name, fn] of listeners) window.addEventListener(name, fn);
  });
  afterEach(() => {
    for (const [name, fn] of listeners) window.removeEventListener(name, fn);
  });

  const items = MENU_COLOR_SPACES.filter(
    (c): c is NonNullable<typeof c> => c !== null,
  );

  it.each(items.map((c) => [c.label, c.event]))(
    "%s is enabled and dispatches %s (same event as the mac menu)",
    (label, event) => {
      render(<TitleBar fileName="x" onOpenFile={vi.fn()} />);
      fireEvent.mouseEnter(screen.getByText("Options"));
      fireEvent.mouseEnter(screen.getByText("Color Space"));
      const el = screen.getByText(label).closest(".menu-item")!;
      expect(el).not.toHaveClass("menu-item-disabled");
      fireEvent.click(el);
      expect(received).toEqual([event]);
    },
  );
});

describe("TitleBar Mode menu", () => {
  it("offers every MENU_MODES entry (incl. AV1 Features / Residuals)", () => {
    const onModeChange = vi.fn();
    render(
      <TitleBar
        fileName="x"
        onOpenFile={vi.fn()}
        onModeChange={onModeChange}
      />,
    );
    fireEvent.mouseEnter(screen.getByText("Mode"));
    fireEvent.click(screen.getByText("AV1 Features"));
    expect(onModeChange).toHaveBeenCalledWith("av1-features");
    fireEvent.mouseEnter(screen.getByText("Mode"));
    fireEvent.click(screen.getByText("Residuals"));
    expect(onModeChange).toHaveBeenCalledWith("residuals");
  });
});
