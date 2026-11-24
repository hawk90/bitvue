/**
 * Timeline bars keep one size whatever their frame type or state: the selected bar used to be
 * scaled up (and glow), so the selected red I-frame bar looked taller than the green ones.
 * Also guards that the frame colors come only from theme/colors.css (a `:root` copy in
 * Timeline.css overrode the theme's colors, including the light theme's).
 */
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const css = readFileSync(
  resolve(__dirname, "../../components/Timeline.css"),
  "utf8",
);

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function rule(selector: string): string {
  const m = new RegExp(`${escapeRegExp(selector)}\\s*\\{([^}]*)\\}`).exec(css);
  return m ? m[1] : "";
}

describe("Timeline bar styles", () => {
  it.each([".timeline-thumb:hover", ".timeline-thumb.selected"])(
    "%s does not resize or glow the bar",
    (selector) => {
      const body = rule(selector);
      expect(body).not.toBe("");
      expect(body).not.toMatch(/transform|box-shadow|filter/);
    },
  );

  it("does not redefine the frame colors", () => {
    expect(css).not.toMatch(/--frame-(i|p|b|unknown)\s*:/);
  });
});
