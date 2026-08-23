/**
 * TimelineHeader Component Tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { TimelineHeader } from "@/components/TimelineHeader";

describe("TimelineHeader", () => {
  it("should render timeline title", () => {
    render(<TimelineHeader currentFrame={42} totalFrames={100} />);

    expect(screen.getByText("Timeline")).toBeInTheDocument();
  });

  it("should render frame count", () => {
    render(<TimelineHeader currentFrame={42} totalFrames={100} />);

    expect(screen.getByText("43 / 100")).toBeInTheDocument();
  });

  it("should display current frame + 1 (0-indexed to 1-indexed)", () => {
    render(<TimelineHeader currentFrame={0} totalFrames={100} />);

    expect(screen.getByText("1 / 100")).toBeInTheDocument();
  });

  it("should handle edge cases", () => {
    const { rerender } = render(
      <TimelineHeader currentFrame={0} totalFrames={1} />,
    );

    expect(screen.getByText("1 / 1")).toBeInTheDocument();

    rerender(<TimelineHeader currentFrame={99} totalFrames={100} />);
    expect(screen.getByText("100 / 100")).toBeInTheDocument();
  });

  it("should have graph icon", () => {
    render(<TimelineHeader currentFrame={0} totalFrames={100} />);

    const icon = document.querySelector(".codicon-graph");
    expect(icon).toBeInTheDocument();
  });

  it("should have correct ARIA attributes", () => {
    render(<TimelineHeader currentFrame={42} totalFrames={100} />);

    const info = screen.getByRole("status");
    expect(info).toHaveAttribute("aria-live", "polite");
  });

  // EDGE-03: PTS quality badge
  it("renders no PTS badge when ptsQuality is omitted, null, or Ok", () => {
    const { rerender } = render(
      <TimelineHeader currentFrame={0} totalFrames={100} />,
    );
    expect(
      document.querySelector(".pts-quality-badge"),
    ).not.toBeInTheDocument();

    rerender(
      <TimelineHeader currentFrame={0} totalFrames={100} ptsQuality={null} />,
    );
    expect(
      document.querySelector(".pts-quality-badge"),
    ).not.toBeInTheDocument();

    rerender(
      <TimelineHeader currentFrame={0} totalFrames={100} ptsQuality="Ok" />,
    );
    expect(
      document.querySelector(".pts-quality-badge"),
    ).not.toBeInTheDocument();
  });

  it("renders a PTS: WARN badge when ptsQuality is Warn", () => {
    render(
      <TimelineHeader currentFrame={0} totalFrames={100} ptsQuality="Warn" />,
    );
    expect(screen.getByText("PTS: WARN")).toBeInTheDocument();
    expect(document.querySelector(".pts-quality-warn")).toBeInTheDocument();
  });

  it("renders a PTS: BAD badge when ptsQuality is Bad", () => {
    render(
      <TimelineHeader currentFrame={0} totalFrames={100} ptsQuality="Bad" />,
    );
    expect(screen.getByText("PTS: BAD")).toBeInTheDocument();
    expect(document.querySelector(".pts-quality-bad")).toBeInTheDocument();
  });
});
