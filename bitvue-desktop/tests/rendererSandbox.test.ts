import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

// #566: the renderer runs with `sandbox: true`. A sandboxed preload may only require "electron"
// (and a few polyfilled built-ins), so a stray `require("node:fs")` would break the preload at
// runtime. Guard both halves statically.
const electronDir = path.join(__dirname, "..", "electron");

describe("renderer sandbox", () => {
  it("main.ts enables sandbox and does not re-enable nodeIntegration", () => {
    const main = readFileSync(path.join(electronDir, "main.ts"), "utf8");
    expect(main).toMatch(/sandbox:\s*true/);
    expect(main).not.toMatch(/sandbox:\s*false/);
    expect(main).toMatch(/nodeIntegration:\s*false/);
    expect(main).toMatch(/contextIsolation:\s*true/);
  });

  it("preload.cjs requires only electron", () => {
    const preload = readFileSync(path.join(electronDir, "preload.cjs"), "utf8");
    const required = [...preload.matchAll(/require\(\s*["']([^"']+)["']\s*\)/g)].map((m) => m[1]);
    expect(required).toEqual(["electron"]);
  });
});
