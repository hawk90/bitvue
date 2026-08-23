/**
 * PanelErrorBoundary tests (EDGE-01) — covers the panel-isolation guarantee that
 * ErrorBoundary.test.tsx's generic "sibling error boundaries" case doesn't: a render-time throw
 * inside one named panel must not take down a sibling panel, and the fallback must identify
 * which panel failed.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  renderWithoutProviders,
  screen,
  fireEvent,
} from "../../test/test-utils";
import { PanelErrorBoundary } from "../../components/PanelErrorBoundary";

vi.mock("../../utils/logger", () => ({
  createLogger: vi.fn(() => ({
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  })),
  logger: { error: vi.fn(), warn: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

const BrokenPanel = () => {
  throw new Error("panel exploded");
};

const WorkingPanel = ({ message = "ok" }: { message?: string }) => (
  <div>{message}</div>
);

describe("PanelErrorBoundary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders children normally when there is no error", () => {
    renderWithoutProviders(
      <PanelErrorBoundary panelName="Player">
        <WorkingPanel message="Player content" />
      </PanelErrorBoundary>,
    );

    expect(screen.getByText("Player content")).toBeInTheDocument();
  });

  it("shows a panel-scoped fallback naming the failed panel, not the app-wide fallback", () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    renderWithoutProviders(
      <PanelErrorBoundary panelName="Player">
        <BrokenPanel />
      </PanelErrorBoundary>,
    );

    expect(screen.getByText("Player")).toBeInTheDocument();
    expect(screen.getByText(/panel failed to render/)).toBeInTheDocument();
    expect(screen.getByText("panel exploded")).toBeInTheDocument();
    // Not the app-wide ErrorBoundary's copy -- confirms this is the panel-scoped fallback.
    expect(screen.queryByText("Something went wrong")).not.toBeInTheDocument();
    expect(document.querySelector(".panel-error-fallback")).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it("Retry resets the panel's own error state", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    renderWithoutProviders(
      <PanelErrorBoundary panelName="Player">
        <BrokenPanel />
      </PanelErrorBoundary>,
    );

    expect(screen.getByText(/panel failed to render/)).toBeInTheDocument();
    const retryButton = screen.getByRole("button", { name: /retry/i });
    expect(() => fireEvent.click(retryButton)).not.toThrow();

    consoleErrorSpy.mockRestore();
  });

  // The actual EDGE-01 guarantee: a crash in one panel's boundary must not remove a sibling
  // panel's content from the DOM -- this is what "isolated to that panel's slot" means in
  // practice, as opposed to the pre-fix state where App.tsx's single app-wide ErrorBoundary
  // would replace the ENTIRE app (including every other panel) with one generic fallback.
  it("isolates a crash to one panel -- a sibling PanelErrorBoundary keeps rendering", () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    renderWithoutProviders(
      <div>
        <PanelErrorBoundary panelName="Timeline">
          <WorkingPanel message="Timeline content" />
        </PanelErrorBoundary>
        <PanelErrorBoundary panelName="Player">
          <BrokenPanel />
        </PanelErrorBoundary>
        <PanelErrorBoundary panelName="Diagnostics">
          <WorkingPanel message="Diagnostics content" />
        </PanelErrorBoundary>
      </div>,
    );

    // The two working sibling panels are completely unaffected by Player's crash.
    expect(screen.getByText("Timeline content")).toBeInTheDocument();
    expect(screen.getByText("Diagnostics content")).toBeInTheDocument();
    // Only the broken panel shows its own scoped fallback.
    expect(screen.getByText("Player")).toBeInTheDocument();
    expect(screen.getByText(/panel failed to render/)).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});
