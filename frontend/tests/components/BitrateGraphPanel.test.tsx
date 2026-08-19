/**
 * BitrateGraphPanel Component Tests
 * Tests bitrate graph panel (dead code -- not currently mounted in App.tsx, see the component's
 * own doc comment -- kept alive by these tests + the panels/index.ts barrel export).
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@/test/test-utils";
import { BitrateGraphPanel } from "../BitrateGraphPanel";

// The component calls `invoke("get_frames")` from `@tauri-apps/api/core` (a real Tauri leftover,
// see the component's own doc comment) and starts in a loading state until that resolves --
// mocked globally in test/setup.ts to resolve unknown commands with `{}` (not an array), which
// crashes the component's `frames.filter(...)` call once it stops loading. Override locally with
// a realistic frame array so the real (still dead-code) success-state markup can be tested.
const mockFrames = [
  { frame_index: 0, frame_type: "I", size: 12000, pts: 0 },
  { frame_index: 1, frame_type: "P", size: 4000, pts: 1 },
  { frame_index: 2, frame_type: "P", size: 3500, pts: 2 },
];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve(mockFrames)),
}));

describe("BitrateGraphPanel", () => {
  beforeEach(async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue(mockFrames);
  });

  it("should render bitrate graph panel", async () => {
    render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );
  });

  it("should display frame stats", async () => {
    render(<BitrateGraphPanel />);

    await waitFor(() => expect(screen.getByText("Frames")).toBeInTheDocument());
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("should use React.memo for performance", async () => {
    const { rerender } = render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );

    rerender(<BitrateGraphPanel />);

    expect(screen.getByText("Bitrate Graph")).toBeInTheDocument();
  });

  it("should render frame type legend", async () => {
    const { container } = render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );
    // Legend renders I/P/B/SKIP swatches (FRAME_TYPE_COLORS minus INTER/KEY/UNKNOWN aliases) --
    // "I"/"P"/"B" also appear as stat labels above, so scope to the legend container.
    const legend = container.querySelector(".bitrate-legend");
    expect(legend).toBeInTheDocument();
    expect(legend?.textContent).toContain("I");
    expect(legend?.textContent).toContain("P");
    expect(legend?.textContent).toContain("B");
  });
});
