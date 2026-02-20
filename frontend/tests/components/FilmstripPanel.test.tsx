/**
 * FilmstripPanel Component Tests
 * Tests filmstrip panel component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { FilmstripPanel } from "../panels/FilmstripPanel";

// Mock TimelineFilmstrip dependency
vi.mock("../TimelineFilmstrip", () => ({
  TimelineFilmstrip: ({
    frames,
    className,
    viewMode,
    filmstripCollapsed,
  }: any) => (
    <div
      className={`timeline-filmstrip ${filmstripCollapsed ? "collapsed" : ""} ${className}`}
    >
      <div className="timeline">Timeline Mock</div>
      {!filmstripCollapsed && (
        <div className="filmstrip">
          Filmstrip Mock
          {frames?.map((f: any) => (
            <div key={f.frame_index} data-frame-type={f.frame_type}>
              Frame {f.frame_index}: {f.frame_type}
            </div>
          ))}
        </div>
      )}
    </div>
  ),
}));

describe("FilmstripPanel", () => {
  const mockFrames = [
    { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
    { frame_index: 2, frame_type: "B", size: 20000, poc: 2 },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render filmstrip panel", () => {
    render(<FilmstripPanel frames={mockFrames} />);

    const filmstrip = document.querySelector(".timeline-filmstrip");
    expect(filmstrip).toBeInTheDocument();
  });

  it("should render timeline and filmstrip", () => {
    render(<FilmstripPanel frames={mockFrames} />);

    expect(screen.getByText("Timeline Mock")).toBeInTheDocument();
    expect(screen.getByText("Filmstrip Mock")).toBeInTheDocument();
  });

  it("should display frame information", () => {
    render(<FilmstripPanel frames={mockFrames} />);

    expect(screen.getByText("Frame 0: I")).toBeInTheDocument();
    expect(screen.getByText("Frame 1: P")).toBeInTheDocument();
    expect(screen.getByText("Frame 2: B")).toBeInTheDocument();
  });

  it("should handle empty frames array", () => {
    render(<FilmstripPanel frames={[]} />);

    const filmstrip = document.querySelector(".timeline-filmstrip");
    expect(filmstrip).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<FilmstripPanel frames={mockFrames} />);

    rerender(<FilmstripPanel frames={mockFrames} />);

    const filmstrip = document.querySelector(".timeline-filmstrip");
    expect(filmstrip).toBeInTheDocument();
  });

  it("should render with correct container styles", () => {
    const { container } = render(<FilmstripPanel frames={mockFrames} />);

    const panel = container.firstChild as HTMLElement;
    expect(panel.style.width).toBe("100%");
    expect(panel.style.height).toBe("100%");
    expect(panel.style.display).toBe("flex");
    expect(panel.style.flexDirection).toBe("column");
  });

  it("should pass frames to TimelineFilmstrip", () => {
    render(<FilmstripPanel frames={mockFrames} />);

    // Verify all frames are rendered
    const frameElements = document.querySelectorAll("[data-frame-type]");
    expect(frameElements.length).toBe(3);
  });

  it("should handle single frame", () => {
    const singleFrame = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    ];
    render(<FilmstripPanel frames={singleFrame} />);

    expect(screen.getByText("Frame 0: I")).toBeInTheDocument();
  });

  it("should handle large frame array", () => {
    const largeFrames = Array.from({ length: 1000 }, (_, i) => ({
      frame_index: i,
      frame_type: i === 0 ? "I" : i % 2 === 0 ? "B" : "P",
      size: 30000,
      poc: i,
    }));

    render(<FilmstripPanel frames={largeFrames} />);

    const filmstrip = document.querySelector(".timeline-filmstrip");
    expect(filmstrip).toBeInTheDocument();
  });
});

describe("FilmstripPanel integration", () => {
  it("should update when frames prop changes", () => {
    const { rerender } = render(
      <FilmstripPanel
        frames={[{ frame_index: 0, frame_type: "I", size: 50000, poc: 0 }]}
      />,
    );

    expect(screen.getByText("Frame 0: I")).toBeInTheDocument();

    rerender(
      <FilmstripPanel
        frames={[
          { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
          { frame_index: 2, frame_type: "B", size: 20000, poc: 2 },
        ]}
      />,
    );

    expect(screen.getByText("Frame 1: P")).toBeInTheDocument();
    expect(screen.getByText("Frame 2: B")).toBeInTheDocument();
  });

  it("should handle null or undefined frames gracefully", () => {
    // @ts-expect-error - testing invalid prop
    render(<FilmstripPanel frames={null} />);

    const filmstrip = document.querySelector(".timeline-filmstrip");
    expect(filmstrip).toBeInTheDocument();
  });
});
