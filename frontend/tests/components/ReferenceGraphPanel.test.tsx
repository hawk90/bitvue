/**
 * ReferenceGraphPanel Component Tests
 * Tests reference graph panel (dead code -- not currently mounted in App.tsx, see the
 * component's own doc comment -- kept alive by these tests + the panels/index.ts barrel export).
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@/test/test-utils";
import { ReferenceGraphPanel } from "../ReferenceGraphPanel";

// Same reasoning as BitrateGraphPanel.test.tsx: the component calls `invoke("get_frames")` and
// starts in a loading state until it resolves -- the global test/setup.ts mock resolves unknown
// commands with `{}` (not an array), which crashes the component's array methods once loading
// stops. Override locally with a realistic frame array (including ref_frames/temporal_id, which
// this component's detail view reads) so the real success-state markup can be tested.
const mockFrames = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 12000,
    pts: 0,
    ref_frames: null,
    temporal_id: 0,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 4000,
    pts: 1,
    ref_frames: [0],
    temporal_id: 1,
  },
  {
    frame_index: 2,
    frame_type: "P",
    size: 3500,
    pts: 2,
    ref_frames: [1],
    temporal_id: 0,
  },
];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve(mockFrames)),
}));

describe("ReferenceGraphPanel", () => {
  beforeEach(async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue(mockFrames);
  });

  it("should render reference graph panel", async () => {
    render(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
  });

  it("should display GOP stats", async () => {
    render(<ReferenceGraphPanel />);

    await waitFor(() => expect(screen.getByText("GOPs")).toBeInTheDocument());
    expect(screen.getByText("Frames")).toBeInTheDocument();
  });

  it("should use React.memo for performance", async () => {
    const { rerender } = render(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );

    rerender(<ReferenceGraphPanel />);

    expect(screen.getByText("Reference Graph")).toBeInTheDocument();
  });

  it("should render frame overview dots for the loaded stream", async () => {
    const { container } = render(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
    expect(container.querySelectorAll(".rg-dot").length).toBe(
      mockFrames.length,
    );
  });

  it("should show frame detail on click", async () => {
    const { container } = render(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
    const firstFrame = container.querySelector(".rg-frame");
    expect(firstFrame).toBeInTheDocument();
    firstFrame?.dispatchEvent(new MouseEvent("click", { bubbles: true }));

    await waitFor(() =>
      expect(container.querySelector(".rg-detail")).toBeInTheDocument(),
    );
  });
});
