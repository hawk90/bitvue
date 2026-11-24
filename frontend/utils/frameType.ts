/**
 * Single mapping from a frame's reported type to the visual "kind" every frame-type color uses
 * (Timeline bars, Filmstrip cards, Frame Sizes bars, badges, ...). Three call sites used to map
 * the type to a class on their own -- `getFrameTypeColorClass` (exact `I/KEY/INTRA`, `P/INTER`,
 * `B`), the Timeline's `getFrameBarClass` (`startsWith("b")`, no `INTRA`) and the badges
 * (`frame-type-${type.toLowerCase()}`, so `KEY`/`INTER`/`INTRA` hit classes that did not exist) --
 * so the same frame could be colored differently depending on the widget.
 */

export type FrameKind = "i" | "p" | "b" | "unknown";

/** Reported frame type (any case, e.g. "I", "KEY", "inter", "B_REF") to its color kind. */
export function frameKind(frameType: string | null | undefined): FrameKind {
  const type = (frameType ?? "").trim().toUpperCase();
  if (
    type === "I" ||
    type === "IDR" ||
    type === "IRAP" ||
    type === "KEY" ||
    type === "INTRA" ||
    type === "SI"
  ) {
    return "i";
  }
  if (type === "P" || type === "P_FRAME" || type === "INTER" || type === "SP") {
    return "p";
  }
  if (
    type === "B" ||
    type === "BFRAME" ||
    type.startsWith("B_") ||
    type.startsWith("B-")
  ) {
    return "b";
  }
  return "unknown";
}

/** CSS class for a frame type under a class family, e.g. `frameKindClass("frame", "KEY")` →
 * `"frame-i"`, `frameKindClass("timeline-bar", "P")` → `"timeline-bar-p"`. */
export function frameKindClass(
  prefix: string,
  frameType: string | null | undefined,
): string {
  return `${prefix}-${frameKind(frameType)}`;
}
