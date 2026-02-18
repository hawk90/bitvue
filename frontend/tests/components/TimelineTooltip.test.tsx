/**
 * TimelineTooltip Component Tests
 */

import { describe, it, expect } from "vitest";
import { render } from "@/test/test-utils";
import { TimelineTooltip } from "@/components/TimelineTooltip";

describe("TimelineTooltip", () => {
  const mockFrame = {
    frame_index: 42,
    frame_type: "I",
    size: 50000,
    pts: 42,
    poc: 21,
  };

  it("should render frame information", () => {
    const { container } = render(
      <TimelineTooltip frame={mockFrame} positionPercent={50} />,
    );

    expect(container.querySelector(".tooltip-frame")).toHaveTextContent("#42");
    expect(container.querySelector(".tooltip-type")).toHaveTextContent("I");
    expect(container.querySelector(".tooltip-size")).toHaveTextContent(
      "48.8 KB",
    );
  });

  it("should position correctly based on percentage", () => {
    const { container } = render(
      <TimelineTooltip frame={mockFrame} positionPercent={75} />,
    );

    const tooltip = container.querySelector(".timeline-tooltip") as HTMLElement;
    expect(tooltip.style.left).toBe("75%");
  });

  it("should calculate size correctly", () => {
    const { container } = render(
      <TimelineTooltip frame={mockFrame} positionPercent={0} />,
    );

    expect(container.querySelector(".tooltip-size")).toHaveTextContent(
      "48.8 KB",
    );
  });

  it("should render different frame types", () => {
    const pFrame = { ...mockFrame, frame_type: "P" as const };
    const bFrame = { ...mockFrame, frame_type: "B" as const };

    const { container: pContainer } = render(
      <TimelineTooltip frame={pFrame} positionPercent={0} />,
    );
    const { container: bContainer } = render(
      <TimelineTooltip frame={bFrame} positionPercent={0} />,
    );

    expect(pContainer.querySelector(".tooltip-type")).toHaveTextContent("P");
    expect(bContainer.querySelector(".tooltip-type")).toHaveTextContent("B");
  });
});
