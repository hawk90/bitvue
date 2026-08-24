/**
 * usePreRenderedArrows tests.
 *
 * Covers a real bug found via live interactive testing (not a static screenshot): after clicking
 * a frame with multiple reference slots, the connecting line and its slot label ("REF0"/"REF1"/
 * etc.) didn't line up at all -- the label sat at `verticalOffset / 2` while its own line's
 * horizontal segment was drawn at the full `verticalOffset`, putting every slot's label in one
 * shared 6px-tall band regardless of which line it was supposed to label.
 */

import { describe, it, expect, vi } from "vitest";
import { render } from "@testing-library/react";
import { useRef } from "react";
import {
  usePreRenderedArrows,
  type ArrowPosition,
  type PathCalculator,
  type FrameInfoBase,
} from "@/components/usePreRenderedArrows";

// Same ㄷ-shaped path convention `ThumbnailsView.tsx`/`VirtualizedThumbnailsView.tsx` both use --
// the horizontal segment (and therefore where a label for that slot should sit) is at
// `sourceBottom + verticalOffset`.
const calculatePath: PathCalculator = (
  sourcePos: ArrowPosition,
  targetPos: ArrowPosition,
  _sourceFrame: FrameInfoBase,
  _targetFrame: FrameInfoBase,
  slotIndex: number,
) => {
  const verticalOffset = 30 + slotIndex * 12;
  const sourceBottom = sourcePos.bottom ?? 0;
  const targetBottom = targetPos.bottom ?? 0;
  return `M ${sourcePos.centerX} ${sourceBottom} L ${sourcePos.centerX} ${sourceBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom}`;
};

function TestHarness({
  frames,
  onResult,
}: {
  frames: FrameInfoBase[];
  onResult: (data: ReturnType<typeof usePreRenderedArrows>) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const result = usePreRenderedArrows({
    containerRef,
    frames,
    getFrameTypeColor: () => "#39f",
    calculatePath,
    enabled: true,
  });
  onResult(result);

  return (
    <div ref={containerRef}>
      {frames.map((f) => (
        <div key={f.frame_index} data-frame-index={f.frame_index} />
      ))}
    </div>
  );
}

/** jsdom's getBoundingClientRect is always all-zero -- stub each frame element to a distinct,
 *  known horizontal slot so `sourcePos`/`targetPos` (and therefore `sourceX`/labelY) are real,
 *  predictable numbers instead of NaN/0. Stubs `offsetLeft`/`offsetTop`/`offsetWidth`/
 *  `offsetHeight` (what the real implementation actually reads -- deliberately NOT
 *  `getBoundingClientRect()`, which is scroll-position-relative and was the real bug: a target
 *  scrolled out of the visible area got a wildly wrong position even though it was correctly
 *  mounted, confirmed via real debug instrumentation against the live app, not a hunch). */
function stubFrameRects(container: HTMLElement, frameWidth = 100) {
  const els = container.querySelectorAll("[data-frame-index]");
  els.forEach((el) => {
    const idx = Number(el.getAttribute("data-frame-index"));
    Object.defineProperty(el, "offsetLeft", {
      value: idx * frameWidth,
      configurable: true,
    });
    Object.defineProperty(el, "offsetTop", { value: 0, configurable: true });
    Object.defineProperty(el, "offsetWidth", {
      value: frameWidth,
      configurable: true,
    });
    Object.defineProperty(el, "offsetHeight", {
      value: 60,
      configurable: true,
    });
  });
}

describe("usePreRenderedArrows", () => {
  it("aligns each slot's label with that same slot's own horizontal line segment", async () => {
    vi.useFakeTimers();
    const frames: FrameInfoBase[] = [
      { frame_index: 0, frame_type: "I", size: 100 },
      { frame_index: 1, frame_type: "P", size: 100 },
      { frame_index: 2, frame_type: "P", size: 100, ref_frames: [0, 1] },
    ];

    let latest: ReturnType<typeof usePreRenderedArrows> | null = null;
    const { container } = render(
      <TestHarness frames={frames} onResult={(r) => (latest = r)} />,
    );
    stubFrameRects(container);

    // The hook's internal calculation runs after a 100ms settle timer.
    await vi.advanceTimersByTimeAsync(150);

    expect(latest).not.toBeNull();
    const arrows = latest!.allArrowData;
    expect(arrows.length).toBe(2);

    const slot0 = arrows.find((a) => a.slotIndex === 0)!;
    const slot1 = arrows.find((a) => a.slotIndex === 1)!;
    expect(slot0).toBeDefined();
    expect(slot1).toBeDefined();

    // sourceBottom is 60 for every stubbed frame; slot 0's line turns horizontal at 60+30=90,
    // slot 1's at 60+42=102 -- the label's Y must match its own slot's turn point exactly, not
    // some shared midpoint between the two.
    expect(slot0.labelY).toBe(90);
    expect(slot1.labelY).toBe(102);

    // The two labels must therefore be the full 12px slot spacing apart, not verticalOffset/2's
    // old 6px.
    expect(slot1.labelY - slot0.labelY).toBe(12);

    vi.useRealTimers();
  });

  it("positions a label exactly on its arrow's own horizontal path segment", async () => {
    vi.useFakeTimers();
    const frames: FrameInfoBase[] = [
      { frame_index: 0, frame_type: "I", size: 100 },
      { frame_index: 1, frame_type: "P", size: 100, ref_frames: [0] },
    ];

    let latest: ReturnType<typeof usePreRenderedArrows> | null = null;
    const { container } = render(
      <TestHarness frames={frames} onResult={(r) => (latest = r)} />,
    );
    stubFrameRects(container);
    await vi.advanceTimersByTimeAsync(150);

    const arrow = latest!.allArrowData[0];
    // pathData = "M x1 y1 L x1 y2 L x2 y2 L x2 y3" (tokens 0-11) -- token [5] is y2, the y where
    // the path turns from vertical to horizontal (the segment a label should sit on). Parse it
    // out and assert the label sits at that same y, confirming line and label genuinely align
    // (not just that both independently equal a hand-computed constant).
    const horizontalSegmentY = Number(arrow.pathData.split(" ")[5]);
    expect(arrow.labelY).toBe(horizontalSegmentY);

    vi.useRealTimers();
  });

  it("draws one merged arrow (not overlapping duplicates) when multiple AV1 reference slots point at the same real frame", async () => {
    vi.useFakeTimers();
    // Real data shape found via live testing: frame 69's 7 real AV1 reference slots were
    // [68,67,66,60,0,0,61] -- slots 4 and 5 both point at frame 0, which is normal AV1 behavior
    // (e.g. GOLDEN and ALTREF reusing the same long-term DPB slot), not a data bug.
    const frames: FrameInfoBase[] = [
      { frame_index: 0, frame_type: "I", size: 100 },
      { frame_index: 60, frame_type: "P", size: 100 },
      {
        frame_index: 69,
        frame_type: "P",
        size: 100,
        ref_frames: [0, 60, 0],
      },
    ];

    let latest: ReturnType<typeof usePreRenderedArrows> | null = null;
    const { container } = render(
      <TestHarness frames={frames} onResult={(r) => (latest = r)} />,
    );
    stubFrameRects(container);
    await vi.advanceTimersByTimeAsync(150);

    const arrows = latest!.allArrowData;
    // 2 unique targets (0 and 60), not 3 raw slots -- exactly one arrow per real target.
    expect(arrows.length).toBe(2);
    const arrowsToZero = arrows.filter((a) => a.targetFrameIndex === 0);
    expect(arrowsToZero.length).toBe(1);

    // The merged arrow's label names both slots that share this target, not just the first one.
    expect(arrowsToZero[0].label).toBe("REF0,REF2");
  });

  it("stacks backward and forward references as two independent depths, not one shared tower", async () => {
    vi.useFakeTimers();
    // Frame 5 has 2 backward refs (0, 1 -- both < 5) and 1 forward ref (9 -- a B-frame-style
    // reference to a not-yet-displayed future frame, > 5). Sharing one ordinal counter across
    // both directions would give these slotIndex 0, 1, 2 and a maxStackDepth of 3; since the two
    // directions' lines diverge in opposite screen directions the moment they leave the source,
    // they don't compete for vertical room and should stack independently instead.
    const frames: FrameInfoBase[] = [
      { frame_index: 0, frame_type: "I", size: 100 },
      { frame_index: 1, frame_type: "P", size: 100 },
      { frame_index: 5, frame_type: "B", size: 100, ref_frames: [0, 1, 9] },
      { frame_index: 9, frame_type: "P", size: 100 },
    ];

    let latest: ReturnType<typeof usePreRenderedArrows> | null = null;
    const { container } = render(
      <TestHarness frames={frames} onResult={(r) => (latest = r)} />,
    );
    stubFrameRects(container);
    await vi.advanceTimersByTimeAsync(150);

    const arrows = latest!.allArrowData;
    expect(arrows.length).toBe(3);

    const toZero = arrows.find((a) => a.targetFrameIndex === 0)!;
    const toOne = arrows.find((a) => a.targetFrameIndex === 1)!;
    const toNine = arrows.find((a) => a.targetFrameIndex === 9)!;

    // The two backward refs still stack against each other (ordinals 0, 1).
    expect(toZero.slotIndex).toBe(0);
    expect(toOne.slotIndex).toBe(1);
    // The lone forward ref starts its own stack at 0, not 2 -- it isn't piled on top of the
    // backward group.
    expect(toNine.slotIndex).toBe(0);

    // Deepest group is the 2-deep backward stack, so maxStackDepth is 2, not 3.
    expect(latest!.maxStackDepth).toBe(2);

    vi.useRealTimers();
  });
});
