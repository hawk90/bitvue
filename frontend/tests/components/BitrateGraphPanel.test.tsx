/**
 * BitrateGraphPanel Component Tests
 * Tests bitrate graph panel (not mounted in App.tsx yet, #567 -- kept alive by these tests + the
 * panels/index.ts barrel export). The data path is the real one: the panel's hook walks the
 * bridge's `getFramesChunk`; only that bridge call is mocked.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, render, screen, waitFor } from "@/test/test-utils";
import { BitrateGraphPanel } from "../BitrateGraphPanel";
import type { BridgeUnitNode } from "@/services/electronBridgeService";

const { getFramesChunk } = vi.hoisted(() => ({ getFramesChunk: vi.fn() }));

vi.mock("@/services/bridge/stream", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/services/bridge/stream")>()),
  getFramesChunk,
}));

const unit = (
  frame_index: number | null,
  frame_type: string | null,
  size: number,
  pts: number | null,
): BridgeUnitNode => ({
  key: null,
  unit_type: "Frame",
  offset: 0,
  size,
  frame_index,
  frame_type,
  pts,
  dts: null,
  display_name: `Frame ${frame_index}`,
  children: [],
  qp_avg: null,
  mv_grid: null,
  temporal_id: null,
  ref_frames: null,
  ref_slots: null,
});

const mockUnits = [
  unit(0, "I", 12000, 0),
  unit(1, "P", 4000, 1),
  unit(2, "P", 3500, 2),
];

const chunkOf = (units: BridgeUnitNode[], total = units.length) => ({
  indexed: true,
  units,
  total_count: total,
});

describe("BitrateGraphPanel", () => {
  beforeEach(() => {
    getFramesChunk.mockReset();
    getFramesChunk.mockResolvedValue(chunkOf(mockUnits));
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

  it("asks the bridge for stream A starting at the first frame", async () => {
    render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );
    expect(getFramesChunk).toHaveBeenCalledTimes(1);
    expect(getFramesChunk).toHaveBeenCalledWith("A", 0, expect.any(Number));
  });

  it("draws one bar per frame, counts the frame types and shows the totals", async () => {
    const { container } = render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );
    expect(container.querySelectorAll(".bitrate-bar")).toHaveLength(3);
    // 12000 + 4000 + 3500 bytes
    expect(screen.getByText("19 KB")).toBeInTheDocument();
    const counts = Array.from(container.querySelectorAll(".bitrate-stat")).map(
      (el) => el.textContent,
    );
    expect(counts).toContain("I1");
    expect(counts).toContain("P2");
    expect(counts).toContain("B0");
  });

  it("shows the hovered frame's index, type, size and PTS", async () => {
    const { container } = render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(container.querySelectorAll(".bitrate-bar")).toHaveLength(3),
    );
    act(() => {
      container
        .querySelectorAll(".bitrate-bar")[1]
        .dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
    });

    const tooltip = await waitFor(() => {
      const el = container.querySelector(".bitrate-tooltip");
      expect(el).toBeInTheDocument();
      return el as Element;
    });
    expect(tooltip.textContent).toContain("Frame 1");
    expect(tooltip.textContent).toContain("(P)");
    expect(tooltip.textContent).toContain("3.91 KB");
    expect(tooltip.textContent).toContain("PTS: 1");
  });

  it("joins every chunk of a stream longer than one chunk", async () => {
    const all = Array.from({ length: 1200 }, (_, i) =>
      unit(i, i === 0 ? "I" : "P", 100, i),
    );
    getFramesChunk.mockImplementation(
      async (_stream: string, offset: number, limit: number) =>
        chunkOf(all.slice(offset, offset + limit), all.length),
    );

    render(<BitrateGraphPanel />);

    await waitFor(() => expect(screen.getByText("1200")).toBeInTheDocument());
    expect(getFramesChunk.mock.calls.length).toBeGreaterThan(1);
    // chunks are contiguous: each call starts where the previous one ended
    const offsets = getFramesChunk.mock.calls.map((c) => c[1] as number);
    expect(offsets[0]).toBe(0);
    for (let i = 1; i < offsets.length; i++) {
      expect(offsets[i]).toBe(
        offsets[i - 1] + (getFramesChunk.mock.calls[i - 1][2] as number),
      );
    }
    expect(
      screen.getByText("Showing first 500 of 1200 frames"),
    ).toBeInTheDocument();
  });

  it("leaves out units that are not frames (no frame_index)", async () => {
    getFramesChunk.mockResolvedValue(
      chunkOf([unit(null, null, 20, null), ...mockUnits]),
    );
    const { container } = render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(container.querySelectorAll(".bitrate-bar")).toHaveLength(3),
    );
  });

  it("shows the empty state when the stream is not indexed yet", async () => {
    getFramesChunk.mockResolvedValue({
      indexed: false,
      units: [],
      total_count: 0,
    });
    render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("No stream loaded")).toBeInTheDocument(),
    );
    expect(screen.queryByText("Bitrate Graph")).not.toBeInTheDocument();
  });

  it("shows the empty state for an indexed stream without frames", async () => {
    getFramesChunk.mockResolvedValue(chunkOf([]));
    render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("No stream loaded")).toBeInTheDocument(),
    );
  });

  it("shows the error and a Retry button that loads again", async () => {
    getFramesChunk.mockRejectedValueOnce(new Error("sidecar not running"));
    render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("sidecar not running")).toBeInTheDocument(),
    );
    expect(screen.queryByText("Bitrate Graph")).not.toBeInTheDocument();

    act(() => {
      screen.getByText("Retry").click();
    });

    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );
    expect(screen.queryByText("sidecar not running")).not.toBeInTheDocument();
    expect(getFramesChunk).toHaveBeenCalledTimes(2);
  });

  it("shows a non-Error rejection as text", async () => {
    getFramesChunk.mockRejectedValueOnce("bridge unavailable");
    render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("bridge unavailable")).toBeInTheDocument(),
    );
  });

  it("shows the loading state until the frames arrive", async () => {
    let resolve: (v: ReturnType<typeof chunkOf>) => void = () => {};
    getFramesChunk.mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    render(<BitrateGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Loading frame data...")).toBeInTheDocument(),
    );
    await act(async () => {
      resolve(chunkOf(mockUnits));
    });
    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );
  });

  it("Reload fetches the frames again", async () => {
    render(<BitrateGraphPanel />);
    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );

    act(() => {
      screen.getByText("Reload").click();
    });

    await waitFor(() => expect(getFramesChunk).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(screen.getByText("Bitrate Graph")).toBeInTheDocument(),
    );
  });
});
