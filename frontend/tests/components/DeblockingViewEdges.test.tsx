/**
 * DeblockingView draws the sidecar's loop filter edges: colored by the largest filter an edge
 * allows (green 16, yellow 8, orange 4), red for an edge that is considered but not filtered.
 */

import { describe, it, expect, vi } from "vitest";
import { renderAct, screen } from "@/test/test-utils";
import type { FrameInfo } from "@/types/video";
import type { DeblockingAnalysisWireResult } from "@/services/electronBridgeService";

const wire: DeblockingAnalysisWireResult = {
  frame_index: 3,
  width: 64,
  height: 32,
  edges: [
    {
      x: 16,
      y: 0,
      length: 16,
      orientation: "vertical",
      filter_size: 16,
      filtered: true,
      level: 20,
    },
    {
      x: 32,
      y: 0,
      length: 8,
      orientation: "vertical",
      filter_size: 8,
      filtered: true,
      level: 20,
    },
    {
      x: 0,
      y: 16,
      length: 12,
      orientation: "horizontal",
      filter_size: 4,
      filtered: true,
      level: 9,
    },
    {
      x: 48,
      y: 0,
      length: 32,
      orientation: "vertical",
      filter_size: 16,
      filtered: false,
      level: 0,
    },
  ],
  params: {
    level: [20, 9, 0, 0],
    sharpness: 0,
    delta_enabled: false,
    ref_deltas: [],
    mode_deltas: [],
  },
  stats: {
    edge_segments: 18,
    filtered_segments: 10,
    unfiltered_segments: 8,
    filtered_16: 4,
    filtered_8: 2,
    filtered_4: 3,
  },
};

vi.mock("@/services/electronBridgeService", async (importOriginal) => ({
  ...(await importOriginal<
    typeof import("@/services/electronBridgeService")
  >()),
  getDeblockingAnalysis: () => Promise.resolve(wire),
}));

const { DeblockingView, FILTER_SIZE_COLORS, UNFILTERED_EDGE_COLOR } =
  await import("@/components/Player/views/DeblockingView");

const frame: FrameInfo = { frame_index: 3, frame_type: "P", size: 1000 };

describe("DeblockingView with loop filter edges", () => {
  it("colors each edge by its filter size, and an unfiltered edge red", async () => {
    const { container } = await renderAct(
      <DeblockingView frame={frame} width={64} height={32} codec="av1" />,
    );
    const lines = Array.from(container.querySelectorAll("svg line"));
    expect(lines).toHaveLength(4);
    expect(lines.map((l) => l.getAttribute("stroke"))).toEqual([
      FILTER_SIZE_COLORS[16],
      FILTER_SIZE_COLORS[8],
      FILTER_SIZE_COLORS[4],
      UNFILTERED_EDGE_COLOR,
    ]);
    // a vertical edge runs down from (x, y), a horizontal one across
    expect(lines[0].getAttribute("y2")).toBe("16");
    expect(lines[2].getAttribute("x2")).toBe("12");
    // an unfiltered edge is fainter than a filtered one
    expect(Number(lines[3].getAttribute("stroke-opacity"))).toBeLessThan(
      Number(lines[0].getAttribute("stroke-opacity")),
    );
  });

  it("explains each edge in its tooltip", async () => {
    const { container } = await renderAct(
      <DeblockingView frame={frame} width={64} height={32} codec="av1" />,
    );
    const title = container.querySelector("svg line title")?.textContent ?? "";
    expect(title).toContain("Largest filter: 16 samples");
    expect(title).toContain("Level: 20");
    expect(title).toContain("Filtered: Yes");
  });

  it("shows the counts", async () => {
    await renderAct(
      <DeblockingView frame={frame} width={64} height={32} codec="av1" />,
    );
    expect(screen.getByText(/10 \(55\.6%\)/)).toBeInTheDocument();
  });
});
