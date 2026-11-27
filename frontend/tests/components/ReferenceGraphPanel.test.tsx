/**
 * ReferenceGraphPanel Component Tests
 * Tests reference graph panel (not mounted in App.tsx yet, #567 -- kept alive by these tests +
 * the panels/index.ts barrel export).
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, renderAct, screen, waitFor } from "@/test/test-utils";
import { ReferenceGraphPanel } from "../ReferenceGraphPanel";
import type { BridgeUnitNode } from "@/services/electronBridgeService";

// The data path is the real one: the panel's hook walks the bridge's `getFramesChunk`; only that
// bridge call is mocked. Units include ref_frames/temporal_id, which this component's detail view
// reads.
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
  ref_frames: number[] | null,
  temporal_id: number | null,
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
  temporal_id,
  ref_frames,
  ref_slots: null,
});

const mockFrames = [
  unit(0, "I", 12000, 0, null, 0),
  unit(1, "P", 4000, 1, [0], 1),
  unit(2, "P", 3500, 2, [1], 0),
];

const chunkOf = (units: BridgeUnitNode[], total = units.length) => ({
  indexed: true,
  units,
  total_count: total,
});

describe("ReferenceGraphPanel", () => {
  beforeEach(() => {
    getFramesChunk.mockReset();
    getFramesChunk.mockResolvedValue(chunkOf(mockFrames));
  });

  it("should render reference graph panel", async () => {
    await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
  });

  it("should display GOP stats", async () => {
    await renderAct(<ReferenceGraphPanel />);

    await waitFor(() => expect(screen.getByText("GOPs")).toBeInTheDocument());
    expect(screen.getByText("Frames")).toBeInTheDocument();
  });

  it("should use React.memo for performance", async () => {
    const { rerender } = await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );

    rerender(<ReferenceGraphPanel />);

    expect(screen.getByText("Reference Graph")).toBeInTheDocument();
  });

  it("should render frame overview dots for the loaded stream", async () => {
    const { container } = await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
    expect(container.querySelectorAll(".rg-dot").length).toBe(
      mockFrames.length,
    );
  });

  it("should show frame detail on click", async () => {
    const { container } = await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
    const firstFrame = container.querySelector(".rg-frame");
    expect(firstFrame).toBeInTheDocument();
    act(() => {
      firstFrame?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    await waitFor(() =>
      expect(container.querySelector(".rg-detail")).toBeInTheDocument(),
    );
  });

  it("shows the clicked frame's type, size, PTS and references", async () => {
    const { container } = await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(container.querySelectorAll(".rg-frame")).toHaveLength(3),
    );
    act(() => {
      container
        .querySelectorAll(".rg-frame")[1]
        .dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    const detail = await waitFor(() => {
      const el = container.querySelector(".rg-detail");
      expect(el).toBeInTheDocument();
      return el as Element;
    });
    expect(detail.textContent).toContain("Index1");
    expect(detail.textContent).toContain("TypeP");
    expect(detail.textContent).toContain("3.91 KB");
    expect(detail.textContent).toContain("PTS1");
    expect(detail.textContent).toContain("Refs0");
  });

  it("splits the stream into GOPs at intra frames and navigates between them", async () => {
    getFramesChunk.mockResolvedValue(
      chunkOf([
        unit(0, "I", 100, 0, null, 0),
        unit(1, "P", 100, 1, [0], 0),
        unit(2, "I", 100, 2, null, 0),
        unit(3, "P", 100, 3, [2], 0),
        unit(4, "P", 100, 4, [3], 0),
      ]),
    );
    const { container } = await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText(/GOP 1 \/ 2/)).toBeInTheDocument(),
    );
    expect(container.querySelectorAll(".rg-frame")).toHaveLength(2);

    act(() => {
      screen.getByText("›").click();
    });

    await waitFor(() =>
      expect(screen.getByText(/GOP 2 \/ 2/)).toBeInTheDocument(),
    );
    expect(container.querySelectorAll(".rg-frame")).toHaveLength(3);
  });

  it("asks the bridge for stream A starting at the first frame", async () => {
    await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
    expect(getFramesChunk).toHaveBeenCalledTimes(1);
    expect(getFramesChunk).toHaveBeenCalledWith("A", 0, expect.any(Number));
  });

  it("joins every chunk of a stream longer than one chunk", async () => {
    const all = Array.from({ length: 1200 }, (_, i) =>
      unit(i, i === 0 ? "I" : "P", 100, i, i === 0 ? null : [i - 1], 0),
    );
    getFramesChunk.mockImplementation(
      async (_stream: string, offset: number, limit: number) =>
        chunkOf(all.slice(offset, offset + limit), all.length),
    );

    const { container } = await renderAct(<ReferenceGraphPanel />);

    // "1200" is both the Frames count and the Avg GOP (one GOP)
    await waitFor(() => expect(screen.getAllByText("1200")).toHaveLength(2));
    expect(getFramesChunk.mock.calls.length).toBeGreaterThan(1);
    // the overview shows at most 300 dots, then "+N"
    expect(container.querySelectorAll(".rg-dot")).toHaveLength(300);
    expect(screen.getByText("+900")).toBeInTheDocument();
  });

  it("leaves out units that are not frames (no frame_index)", async () => {
    getFramesChunk.mockResolvedValue(
      chunkOf([unit(null, null, 20, null, null, null), ...mockFrames]),
    );
    const { container } = await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(container.querySelectorAll(".rg-dot")).toHaveLength(3),
    );
  });

  it("shows the empty state when the stream is not indexed yet", async () => {
    getFramesChunk.mockResolvedValue({
      indexed: false,
      units: [],
      total_count: 0,
    });
    await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("No stream loaded")).toBeInTheDocument(),
    );
    expect(screen.queryByText("Reference Graph")).not.toBeInTheDocument();
  });

  it("shows the error and a Retry button that loads again", async () => {
    getFramesChunk.mockRejectedValueOnce(new Error("sidecar not running"));
    await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("sidecar not running")).toBeInTheDocument(),
    );
    expect(screen.queryByText("Reference Graph")).not.toBeInTheDocument();

    act(() => {
      screen.getByText("Retry").click();
    });

    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
    expect(screen.queryByText("sidecar not running")).not.toBeInTheDocument();
    expect(getFramesChunk).toHaveBeenCalledTimes(2);
  });

  it("shows the loading state until the frames arrive", async () => {
    let resolve: (v: ReturnType<typeof chunkOf>) => void = () => {};
    getFramesChunk.mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    await renderAct(<ReferenceGraphPanel />);

    await waitFor(() =>
      expect(screen.getByText("Loading...")).toBeInTheDocument(),
    );
    await act(async () => {
      resolve(chunkOf(mockFrames));
    });
    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
  });

  it("Reload fetches the frames again", async () => {
    await renderAct(<ReferenceGraphPanel />);
    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );

    act(() => {
      screen.getByText("Reload").click();
    });

    await waitFor(() => expect(getFramesChunk).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(screen.getByText("Reference Graph")).toBeInTheDocument(),
    );
  });
});
