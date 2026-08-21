/**
 * WelcomeContainer Tests
 *
 * Verifies this component correctly wires useValidatedRecentFiles + useWelcomeActions into
 * WelcomeScreen -- not re-testing either hook's own internal behavior (see their dedicated test
 * files), just that the container assembles them correctly.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@/test/test-utils";
import { WelcomeContainer } from "@/components/WelcomeContainer";
import { pathExists, getSamplePath } from "@/services/electronBridgeService";

vi.mock("@/services/electronBridgeService", () => ({
  pathExists: vi.fn().mockResolvedValue(true),
  getSamplePath: vi.fn().mockResolvedValue("/samples/foreman_av1.ivf"),
}));

describe("WelcomeContainer", () => {
  beforeEach(() => {
    vi.mocked(pathExists).mockReset().mockResolvedValue(true);
    vi.mocked(getSamplePath)
      .mockReset()
      .mockResolvedValue("/samples/foreman_av1.ivf");
  });

  it("passes error and recent files through to WelcomeScreen", async () => {
    render(
      <WelcomeContainer
        onOpenFile={vi.fn(async () => {})}
        openFileAtPath={vi.fn(async () => {})}
        error="boom"
        onShowShortcuts={vi.fn()}
        recentFiles={["/a/b/clip.ivf"]}
        removeRecentFile={vi.fn()}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent("boom");
    await waitFor(() => {
      expect(screen.getByText("clip.ivf")).toBeInTheDocument();
    });
  });

  it("disables the open action while handleOpenFile's promise is pending", async () => {
    let resolveOpen!: () => void;
    const onOpenFile = vi.fn(
      () => new Promise<void>((resolve) => (resolveOpen = resolve)),
    );

    render(
      <WelcomeContainer
        onOpenFile={onOpenFile}
        openFileAtPath={vi.fn(async () => {})}
        error={null}
        onShowShortcuts={vi.fn()}
        recentFiles={[]}
        removeRecentFile={vi.fn()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: /open bitstream file/i }),
    );

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /opening/i })).toBeDisabled();
    });

    resolveOpen();
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /open bitstream file/i }),
      ).not.toBeDisabled();
    });
  });

  it("routes a recent-file click through openFileAtPath", async () => {
    const openFileAtPath = vi.fn(async () => {});

    render(
      <WelcomeContainer
        onOpenFile={vi.fn(async () => {})}
        openFileAtPath={openFileAtPath}
        error={null}
        onShowShortcuts={vi.fn()}
        recentFiles={["/a/b/clip.ivf"]}
        removeRecentFile={vi.fn()}
      />,
    );

    await waitFor(() => screen.getByText("clip.ivf"));
    fireEvent.click(screen.getByText("clip.ivf"));

    await waitFor(() => {
      expect(openFileAtPath).toHaveBeenCalledWith("/a/b/clip.ivf");
    });
  });

  it("prunes a stale recent file via removeRecentFile", async () => {
    vi.mocked(pathExists).mockResolvedValue(false);
    const removeRecentFile = vi.fn();

    render(
      <WelcomeContainer
        onOpenFile={vi.fn(async () => {})}
        openFileAtPath={vi.fn(async () => {})}
        error={null}
        onShowShortcuts={vi.fn()}
        recentFiles={["/a/gone.ivf"]}
        removeRecentFile={removeRecentFile}
      />,
    );

    await waitFor(() => {
      expect(removeRecentFile).toHaveBeenCalledWith("/a/gone.ivf");
    });
    expect(screen.queryByText("gone.ivf")).not.toBeInTheDocument();
  });
});
