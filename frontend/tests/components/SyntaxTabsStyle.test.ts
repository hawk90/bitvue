/**
 * The Syntax Detail tab row must scroll, not squeeze: at the minimum window width the tabs shrank
 * below their labels and overlapped ("FRAMI", "SEARC", #550).
 */
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const css = readFileSync(
  resolve(__dirname, "../../components/panels/SyntaxDetailPanel.css"),
  "utf8",
);

function block(selector: string): string {
  const start = css.indexOf(`\n${selector} {`);
  if (start === -1) return "";
  return css.slice(start, css.indexOf("}", start));
}

describe("Syntax Detail tab row", () => {
  it("scrolls horizontally", () => {
    expect(block(".syntax-tabs")).toMatch(/overflow-x:\s*auto/);
  });

  it("keeps each tab at its label width", () => {
    expect(block(".syntax-tab")).toMatch(/flex-shrink:\s*0/);
  });
});
