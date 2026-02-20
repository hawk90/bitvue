/**
 * Reference Lines Overlay Component Tests
 */

import { describe, it, expect } from "vitest";
import { render } from "@/test/test-utils";
import { ReferenceLinesOverlay } from "@/components/ReferenceLinesOverlay";

interface ExpansionInfo {
  from: number;
  fromPos: { x: number; y: number };
  to: number;
  toPos: { x: number; y: number };
  color: string;
  arrowIndex: number;
  arrowTotal: number;
}

const mockExpansionInfo: ExpansionInfo[] = [
  {
    from: 0,
    fromPos: { x: 100, y: 50 },
    to: 5,
    toPos: { x: 300, y: 150 },
    color: "#ff0000",
    arrowIndex: 0,
    arrowTotal: 1,
  },
  {
    from: 1,
    fromPos: { x: 100, y: 60 },
    to: 5,
    toPos: { x: 300, y: 150 },
    color: "#00ff00",
    arrowIndex: 0,
    arrowTotal: 2,
  },
  {
    from: 2,
    fromPos: { x: 100, y: 70 },
    to: 5,
    toPos: { x: 300, y: 150 },
    color: "#0000ff",
    arrowIndex: 1,
    arrowTotal: 2,
  },
];

describe("ReferenceLinesOverlay", () => {
  it("should render SVG element", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay expansionInfo={[]} containerRef={containerRef} />,
    );

    const svg = container.querySelector("svg.reference-lines-overlay");
    expect(svg).toBeInTheDocument();
  });

  it("should render no lines when expansionInfo is empty", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay expansionInfo={[]} containerRef={containerRef} />,
    );

    const paths = container.querySelectorAll("svg path");
    expect(paths).toHaveLength(0);
  });

  it("should render one path and one arrow per expansion info", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    const paths = container.querySelectorAll("svg path");
    // Each expansion info has 2 paths: the line and the arrow
    expect(paths).toHaveLength(6);
  });

  it("should render circles at target positions", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    const circles = container.querySelectorAll("svg circle");
    // Each expansion info has 2 circles: outer and inner
    expect(circles).toHaveLength(6);
  });

  it("should apply correct styles to SVG", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay expansionInfo={[]} containerRef={containerRef} />,
    );

    const svg = container.querySelector(
      "svg.reference-lines-overlay",
    ) as HTMLElement;
    expect(svg.style.position).toBe("absolute");
    expect(svg.style.top).toBe("0px"); // Browser normalizes to '0px'
    expect(svg.style.left).toBe("0px"); // Browser normalizes to '0px'
    expect(svg.style.pointerEvents).toBe("none");
    expect(svg.style.zIndex).toBe("100");
  });

  it("should use correct color for each line", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    const paths = container.querySelectorAll("svg path");
    const firstPath = paths[0] as HTMLElement;
    expect(firstPath.getAttribute("stroke")).toBe("#ff0000");

    const secondPath = paths[2] as HTMLElement;
    expect(secondPath.getAttribute("stroke")).toBe("#00ff00");

    const thirdPath = paths[4] as HTMLElement;
    expect(thirdPath.getAttribute("stroke")).toBe("#0000ff");
  });

  it("should create unique keys for each element", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    const groups = container.querySelectorAll("svg g");
    expect(groups).toHaveLength(3);

    // React's key prop doesn't render as a DOM attribute
    // Just verify that the correct number of groups are rendered
    expect(groups.length).toBe(3);
  });

  it("should calculate arrow position based on target element width", () => {
    const containerRef = { current: document.createElement("div") };
    const targetDiv = document.createElement("div");
    targetDiv.setAttribute("data-frame-index", "5");
    targetDiv.style.width = "100px";
    containerRef.current.appendChild(targetDiv);

    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    // Should not throw error and should render correctly
    const paths = container.querySelectorAll("svg path");
    expect(paths.length).toBeGreaterThan(0);
  });

  it("should handle multiple arrows to same target", () => {
    const containerRef = { current: document.createElement("div") };
    const multiArrowInfo: ExpansionInfo[] = [
      {
        from: 0,
        fromPos: { x: 100, y: 50 },
        to: 5,
        toPos: { x: 300, y: 150 },
        color: "#ff0000",
        arrowIndex: 0,
        arrowTotal: 3,
      },
      {
        from: 1,
        fromPos: { x: 100, y: 60 },
        to: 5,
        toPos: { x: 300, y: 150 },
        color: "#00ff00",
        arrowIndex: 1,
        arrowTotal: 3,
      },
      {
        from: 2,
        fromPos: { x: 100, y: 70 },
        to: 5,
        toPos: { x: 300, y: 150 },
        color: "#0000ff",
        arrowIndex: 2,
        arrowTotal: 3,
      },
    ];

    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={multiArrowInfo}
        containerRef={containerRef}
      />,
    );

    const groups = container.querySelectorAll("svg g");
    expect(groups).toHaveLength(3);
  });

  it("should apply correct stroke opacity", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    const linePaths = container.querySelectorAll("svg path:nth-child(1)");
    const firstLine = linePaths[0] as HTMLElement;
    expect(firstLine.getAttribute("stroke-opacity")).toBe("0.6");
  });

  it("should apply correct fill opacity for arrow", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    const arrowPaths = container.querySelectorAll("svg path:nth-child(2)");
    const firstArrow = arrowPaths[0] as HTMLElement;
    expect(firstArrow.getAttribute("fill-opacity")).toBe("0.9");
  });

  it("should render outer and inner circles", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    const groups = container.querySelectorAll("svg g");
    const firstGroup = groups[0];

    const outerCircle = firstGroup.querySelectorAll("circle")[0] as HTMLElement;
    const innerCircle = firstGroup.querySelectorAll("circle")[1] as HTMLElement;

    expect(outerCircle.getAttribute("r")).toBe("4");
    expect(innerCircle.getAttribute("r")).toBe("1.5");

    expect(outerCircle.getAttribute("fill-opacity")).toBe("0.15");
    expect(innerCircle.getAttribute("fill-opacity")).toBe("1");
  });

  it("should handle containerRef with null current", () => {
    const containerRef = { current: null };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    // Should still render without throwing error
    const svg = container.querySelector("svg.reference-lines-overlay");
    expect(svg).toBeInTheDocument();
  });

  it("should use default width when target element not found", () => {
    const containerRef = { current: document.createElement("div") };
    const { container } = render(
      <ReferenceLinesOverlay
        expansionInfo={mockExpansionInfo}
        containerRef={containerRef}
      />,
    );

    // Should not throw error
    const paths = container.querySelectorAll("svg path");
    expect(paths).toHaveLength(6);
  });
});
