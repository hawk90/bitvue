/**
 * WelcomeScreen Component Tests
 *
 * WelcomeScreen is pure presentation now (see its own module doc) -- these tests only exercise
 * render + callback wiring. Recent-file validation/pruning lives in
 * useValidatedRecentFiles.test.ts; open-in-flight guarding lives in useWelcomeActions.test.ts.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { WelcomeScreen } from "@/components/WelcomeScreen";

function renderWelcome(
  overrides: Partial<Parameters<typeof WelcomeScreen>[0]> = {},
) {
  return render(
    <WelcomeScreen
      onOpenFile={vi.fn()}
      loading={false}
      error={null}
      onShowShortcuts={vi.fn()}
      recentFiles={[]}
      onOpenRecent={vi.fn()}
      sampleResolvedPaths={{}}
      onOpenSample={vi.fn()}
      {...overrides}
    />,
  );
}

describe("WelcomeScreen", () => {
  let mockOnOpenFile: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockOnOpenFile = vi.fn();
  });

  it("renders the header", () => {
    renderWelcome();
    expect(screen.getByText("Bitvue")).toBeInTheDocument();
    expect(screen.getByText("Video Bitstream Analyzer")).toBeInTheDocument();
  });

  it("renders the Open Bitstream File action with its shortcut badge", () => {
    renderWelcome({ onOpenFile: mockOnOpenFile });
    const button = screen.getByRole("button", {
      name: /open bitstream file/i,
    });
    expect(button).toBeInTheDocument();
    expect(button.querySelectorAll("kbd").length).toBeGreaterThan(0);
  });

  it("calls onOpenFile when the Open row is clicked", () => {
    renderWelcome({ onOpenFile: mockOnOpenFile });
    fireEvent.click(
      screen.getByRole("button", { name: /open bitstream file/i }),
    );
    expect(mockOnOpenFile).toHaveBeenCalledTimes(1);
  });

  it("shows the Opening state and hides the shortcut badge while loading", () => {
    renderWelcome({ loading: true });
    const button = screen.getByRole("button", { name: /opening/i });
    expect(button).toBeDisabled();
    expect(button.querySelectorAll("kbd").length).toBe(0);
  });

  it("shows the error message with role=alert", () => {
    renderWelcome({ error: "Failed to open file" });
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Failed to open file");
  });

  it("renders Keyboard Shortcuts and calls onShowShortcuts when clicked", () => {
    const onShowShortcuts = vi.fn();
    renderWelcome({ onShowShortcuts });
    fireEvent.click(
      screen.getByRole("button", { name: /keyboard shortcuts/i }),
    );
    expect(onShowShortcuts).toHaveBeenCalledTimes(1);
  });

  it("does not render a Recent section when there are no recent files", () => {
    renderWelcome({ recentFiles: [] });
    expect(screen.queryByText("Recent")).not.toBeInTheDocument();
  });

  it("renders recent files with name and directory split out", () => {
    // Deliberately not foreman_av1.ivf -- that name also appears in the Samples section (see
    // sampleCatalog.ts), which would make the getByText queries below ambiguous.
    renderWelcome({
      recentFiles: ["/Users/hawk/Workspaces/projects/my_capture.ivf"],
    });
    expect(screen.getByText("Recent")).toBeInTheDocument();
    expect(screen.getByText("my_capture.ivf")).toBeInTheDocument();
    expect(
      screen.getByText("/Users/hawk/Workspaces/projects"),
    ).toBeInTheDocument();
  });

  it("calls onOpenRecent with the full path when a recent file is clicked", () => {
    const onOpenRecent = vi.fn();
    renderWelcome({ recentFiles: ["/a/b/clip.ivf"], onOpenRecent });
    fireEvent.click(screen.getByText("clip.ivf"));
    expect(onOpenRecent).toHaveBeenCalledWith("/a/b/clip.ivf");
  });

  it("calls onRemoveRecent (not onOpenRecent) when a recent file's remove button is clicked", () => {
    const onOpenRecent = vi.fn();
    const onRemoveRecent = vi.fn();
    renderWelcome({
      recentFiles: ["/a/b/clip.ivf"],
      onOpenRecent,
      onRemoveRecent,
    });
    fireEvent.click(
      screen.getByRole("button", { name: /remove clip.ivf from recent/i }),
    );
    expect(onRemoveRecent).toHaveBeenCalledWith("/a/b/clip.ivf");
    expect(onOpenRecent).not.toHaveBeenCalled();
  });

  it("does not render a remove button when onRemoveRecent is absent", () => {
    renderWelcome({
      recentFiles: ["/a/b/clip.ivf"],
      onRemoveRecent: undefined,
    });
    expect(
      screen.queryByRole("button", { name: /remove/i }),
    ).not.toBeInTheDocument();
  });

  it("disables the recent-file open and remove buttons while loading", () => {
    renderWelcome({
      recentFiles: ["/a/b/clip.ivf"],
      onRemoveRecent: vi.fn(),
      loading: true,
    });
    expect(screen.getByText("clip.ivf").closest("button")).toBeDisabled();
    expect(
      screen.getByRole("button", { name: /remove clip.ivf from recent/i }),
    ).toBeDisabled();
  });

  it("renders the GitHub footer link", () => {
    renderWelcome();
    expect(screen.getByText("GitHub")).toBeInTheDocument();
  });

  describe("Samples", () => {
    it("renders all codec groups collapsed by default, expanding AV1 on click", () => {
      const onOpenSample = vi.fn();
      renderWelcome({
        sampleResolvedPaths: { "foreman_av1.ivf": "/samples/foreman_av1.ivf" },
        onOpenSample,
      });

      expect(screen.getByText("Samples")).toBeInTheDocument();
      expect(
        screen.queryByRole("button", { name: /ivf.*foreman_av1\.ivf/i }),
      ).not.toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: /^av1/i }));
      const av1Row = screen.getByRole("button", {
        name: /ivf.*foreman_av1\.ivf/i,
      });
      expect(av1Row).not.toBeDisabled();

      fireEvent.click(av1Row);
      expect(onOpenSample).toHaveBeenCalledWith("/samples/foreman_av1.ivf");
    });

    it("disables the AV1/IVF row until its path has resolved", () => {
      renderWelcome({ sampleResolvedPaths: {} });
      fireEvent.click(screen.getByRole("button", { name: /^av1/i }));
      const av1Row = screen.getByRole("button", {
        name: /ivf.*foreman_av1\.ivf/i,
      });
      expect(av1Row).toBeDisabled();
    });

    it("collapses other codec groups by default, expanding on click", () => {
      renderWelcome({
        sampleResolvedPaths: { "foreman_av1.ivf": "/samples/foreman_av1.ivf" },
      });

      expect(
        screen.queryByRole("button", { name: /foreman_hevc\.mp4/i }),
      ).not.toBeInTheDocument();

      fireEvent.click(screen.getByRole("button", { name: /^hevc/i }));

      const hevcMp4Row = screen.getByRole("button", {
        name: /mp4.*foreman_hevc\.mp4.*coming soon/i,
      });
      expect(hevcMp4Row).toBeInTheDocument();
      expect(hevcMp4Row).toBeDisabled();
    });
  });
});
