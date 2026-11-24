import { describe, it, expect } from "vitest";
import { render } from "@/test/test-utils";
import { FrameTypeBadge } from "@/components/common/FrameTypeBadge";

describe("FrameTypeBadge", () => {
  it.each([
    ["I", "frame-type-i"],
    ["KEY", "frame-type-i"],
    ["IO", "frame-type-i"],
    ["P", "frame-type-p"],
    ["INTER", "frame-type-p"],
    ["B", "frame-type-b"],
    ["SW", "frame-type-switch"],
    ["?", "frame-type-unknown"],
  ])("%s gets %s", (type, cls) => {
    const { container } = render(<FrameTypeBadge frameType={type} />);
    const badge = container.querySelector(".frame-type-badge");
    expect(badge).toHaveClass(cls);
    expect(badge).toHaveTextContent(type);
  });

  // #554: the intra-only and switch labels are not self-explanatory
  it("explains the intra-only and switch labels in a tooltip", () => {
    const io = render(<FrameTypeBadge frameType="IO" />).container;
    expect(
      io.querySelector(".frame-type-badge")?.getAttribute("title"),
    ).toMatch(/not a random access point/);
    const sw = render(<FrameTypeBadge frameType="SW" />).container;
    expect(
      sw.querySelector(".frame-type-badge")?.getAttribute("title"),
    ).toMatch(/inter coded/);
  });
});
