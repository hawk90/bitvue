import { afterEach, beforeEach, vi, type MockInstance } from "vitest";

const EXPECTED = /window\.bitvue is unavailable|no bridge in tests/;

function isExpected(arg: unknown): boolean {
  if (arg instanceof Error) return EXPECTED.test(arg.message);
  return typeof arg === "string" && EXPECTED.test(arg);
}

/**
 * For test files that mount components/contexts which call the Electron bridge on mount but
 * don't care about the result: the jsdom run has no window.bitvue, so the code's own error
 * handling logs "window.bitvue is unavailable". Filter exactly that message and forward every
 * other console.error/warn, so unexpected errors still surface. Call once at file top level.
 */
export function muteBridgeUnavailableLogs(): void {
  const spies: MockInstance[] = [];
  beforeEach(() => {
    for (const level of ["error", "warn"] as const) {
      const original = console[level].bind(console);
      spies.push(
        vi.spyOn(console, level).mockImplementation((...args: unknown[]) => {
          if (args.some(isExpected)) return;
          original(...args);
        }),
      );
    }
  });
  afterEach(() => {
    for (const spy of spies.splice(0)) spy.mockRestore();
  });
}
