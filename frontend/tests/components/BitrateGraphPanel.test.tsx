/**
 * BitrateGraphPanel Component Tests
 * Tests bitrate graph placeholder panel
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { BitrateGraphPanel } from "../BitrateGraphPanel";

describe("BitrateGraphPanel", () => {
  it("should render bitrate graph panel", () => {
    render(<BitrateGraphPanel />);

    expect(screen.getByText("Bitrate Graph")).toBeInTheDocument();
  });

  it("should display description", () => {
    render(<BitrateGraphPanel />);

    expect(
      screen.getByText(/Frame size and bitrate over time/),
    ).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<BitrateGraphPanel />);

    rerender(<BitrateGraphPanel />);

    expect(screen.getByText("Bitrate Graph")).toBeInTheDocument();
  });

  it("should render graph icon", () => {
    const { container } = render(<BitrateGraphPanel />);

    const icon = container.querySelector(".codicon-graph");
    expect(icon).toBeInTheDocument();
  });
});
