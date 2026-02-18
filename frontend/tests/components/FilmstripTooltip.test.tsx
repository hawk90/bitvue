/**
 * Filmstrip Tooltip Component Tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { FilmstripTooltip } from "@/components/FilmstripTooltip";
import type { FrameInfo } from "@/types/video";

const mockFrame: FrameInfo = {
  frame_index: 42,
  frame_type: "I",
  size: 102400,
  pts: 1000,
  poc: 0,
  temporal_id: 0,
  ref_frames: [10, 20],
};

describe("FilmstripTooltip", () => {
  it("should render frame number and type", () => {
    const { container } = render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    expect(screen.getByText("#42")).toBeInTheDocument();
    expect(screen.getByText("I")).toBeInTheDocument();
  });

  it("should render frame size in KB", () => {
    render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    expect(screen.getByText("100.0 KB")).toBeInTheDocument();
  });

  it("should render PTS when available", () => {
    render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    expect(screen.getByText("1000")).toBeInTheDocument(); // PTS value
  });

  it("should not render PTS when not available", () => {
    const frameWithoutPts = { ...mockFrame, pts: undefined };
    const { container } = render(
      <FilmstripTooltip
        frame={frameWithoutPts}
        x={100}
        y={100}
        placement="right"
      />,
    );

    const ptsLabels = container.querySelectorAll(".filmstrip-tooltip-label");
    const hasPtsLabel = Array.from(ptsLabels).some(
      (el) => el.textContent === "PTS",
    );
    expect(hasPtsLabel).toBe(false);
  });

  it("should render POC when available", () => {
    render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    expect(screen.getByText("0")).toBeInTheDocument(); // POC value
  });

  it("should not render POC when not available", () => {
    const frameWithoutPoc = { ...mockFrame, poc: undefined };
    const { container } = render(
      <FilmstripTooltip
        frame={frameWithoutPoc}
        x={100}
        y={100}
        placement="right"
      />,
    );

    const pocLabels = container.querySelectorAll(".filmstrip-tooltip-label");
    const hasPocLabel = Array.from(pocLabels).some(
      (el) => el.textContent === "POC",
    );
    expect(hasPocLabel).toBe(false);
  });

  it("should render temporal ID when available", () => {
    render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    expect(screen.getByText("T0")).toBeInTheDocument();
  });

  it("should not render temporal ID when not available", () => {
    const frameWithoutTemporal = { ...mockFrame, temporal_id: undefined };
    const { container } = render(
      <FilmstripTooltip
        frame={frameWithoutTemporal}
        x={100}
        y={100}
        placement="right"
      />,
    );

    const temporalLabels = container.querySelectorAll(
      ".filmstrip-tooltip-label",
    );
    const hasTemporalLabel = Array.from(temporalLabels).some(
      (el) => el.textContent === "Temporal",
    );
    expect(hasTemporalLabel).toBe(false);
  });

  it("should render reference frames when available", () => {
    render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    expect(screen.getByText("[10, 20]")).toBeInTheDocument();
  });

  it("should not render reference frames when empty", () => {
    const frameWithoutRefs = { ...mockFrame, ref_frames: [] };
    const { container } = render(
      <FilmstripTooltip
        frame={frameWithoutRefs}
        x={100}
        y={100}
        placement="right"
      />,
    );

    const refLabels = container.querySelectorAll(".filmstrip-tooltip-label");
    const hasRefLabel = Array.from(refLabels).some(
      (el) => el.textContent === "References",
    );
    expect(hasRefLabel).toBe(false);
  });

  it("should not render reference frames when undefined", () => {
    const frameWithoutRefs = { ...mockFrame, ref_frames: undefined };
    const { container } = render(
      <FilmstripTooltip
        frame={frameWithoutRefs}
        x={100}
        y={100}
        placement="right"
      />,
    );

    const refLabels = container.querySelectorAll(".filmstrip-tooltip-label");
    const hasRefLabel = Array.from(refLabels).some(
      (el) => el.textContent === "References",
    );
    expect(hasRefLabel).toBe(false);
  });

  it("should apply correct class for left placement", () => {
    const { container } = render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="left" />,
    );

    const tooltip = container.querySelector(".filmstrip-tooltip");
    expect(tooltip).toHaveClass("tooltip-left");
  });

  it("should apply correct class for right placement", () => {
    const { container } = render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    const tooltip = container.querySelector(".filmstrip-tooltip");
    expect(tooltip).toHaveClass("tooltip-right");
  });

  it("should position tooltip correctly", () => {
    const { container } = render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    const tooltip = container.querySelector(
      ".filmstrip-tooltip",
    ) as HTMLElement;
    expect(tooltip.style.left).toBe("140px"); // x + offset for right placement
    expect(tooltip.style.top).toBe("40px"); // y - 60 offset
  });

  it("should position tooltip left correctly", () => {
    const { container } = render(
      <FilmstripTooltip frame={mockFrame} x={200} y={100} placement="left" />,
    );

    const tooltip = container.querySelector(
      ".filmstrip-tooltip",
    ) as HTMLElement;
    expect(tooltip.style.left).toBe("160px"); // x - 40 offset for left placement
    expect(tooltip.style.top).toBe("40px"); // y - 60 offset
  });

  it("should handle different frame types", () => {
    const frameTypes: Array<"I" | "P" | "B"> = ["I", "P", "B"];

    frameTypes.forEach((frameType) => {
      const frame = { ...mockFrame, frame_type: frameType };
      const { container, unmount } = render(
        <FilmstripTooltip frame={frame} x={100} y={100} placement="right" />,
      );

      const typeBadge = container.querySelector(".frame-type");
      expect(typeBadge).toHaveClass(`type-${frameType.toLowerCase()}`);
      unmount();
    });
  });

  it("should render highlight class for reference frames", () => {
    render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    const highlight = document.querySelector(".highlight");
    expect(highlight).toBeInTheDocument();
  });

  it("should display all labels correctly", () => {
    const { container } = render(
      <FilmstripTooltip frame={mockFrame} x={100} y={100} placement="right" />,
    );

    const labels = container.querySelectorAll(".filmstrip-tooltip-label");
    const labelTexts = Array.from(labels).map((el) => el.textContent);

    expect(labelTexts).toContain("Size");
    expect(labelTexts).toContain("PTS");
    expect(labelTexts).toContain("POC");
    expect(labelTexts).toContain("Temporal");
    expect(labelTexts).toContain("References");
  });
});
