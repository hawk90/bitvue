/**
 * Characterization tests for the six menu-event hooks (#560): which window event reaches which
 * handler, latest handler wins after a rerender, nothing fires after unmount, and every
 * addEventListener has a matching removeEventListener (same type + same listener).
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useFileMenuEvents } from "@/hooks/useFileMenuEvents";
import { useOptionsMenuEvents } from "@/hooks/useOptionsMenuEvents";
import { useLayoutMenuEvents } from "@/hooks/useLayoutMenuEvents";
import { useOverlayMenuEvents } from "@/hooks/useOverlayMenuEvents";
import { useModeMenuEvents } from "@/hooks/useModeMenuEvents";
import { useThemeMenuEvents } from "@/hooks/useThemeMenuEvents";

const closeWindow = vi.hoisted(() => vi.fn());
const showOpenDialog = vi.hoisted(() => vi.fn());
vi.mock("@/services/electronBridgeService", () => ({
  closeWindow,
  showOpenDialog,
}));

function fire(name: string, detail?: unknown) {
  act(() => {
    window.dispatchEvent(
      detail === undefined
        ? new CustomEvent(name)
        : new CustomEvent(name, { detail }),
    );
  });
}

type Pair = [string, unknown];

/** Record every add/remove call (still forwarding to the real implementation). */
function trackListeners() {
  const adds: Pair[] = [];
  const removes: Pair[] = [];
  const realAdd = window.addEventListener.bind(window);
  const realRemove = window.removeEventListener.bind(window);
  const addSpy = vi.spyOn(window, "addEventListener").mockImplementation(((
    t: string,
    l: EventListener,
    o?: unknown,
  ) => {
    adds.push([t, l]);
    realAdd(t, l, o as AddEventListenerOptions);
  }) as typeof window.addEventListener);
  const rmSpy = vi.spyOn(window, "removeEventListener").mockImplementation(((
    t: string,
    l: EventListener,
    o?: unknown,
  ) => {
    removes.push([t, l]);
    realRemove(t, l, o as EventListenerOptions);
  }) as typeof window.removeEventListener);
  return {
    adds,
    removes,
    restore: () => {
      addSpy.mockRestore();
      rmSpy.mockRestore();
    },
  };
}

function expectBalanced(t: ReturnType<typeof trackListeners>, types: string[]) {
  const mine = (xs: Pair[]) => xs.filter(([n]) => types.includes(n));
  expect(
    mine(t.adds)
      .map(([n]) => n)
      .sort(),
  ).toEqual([...types].sort());
  for (const [n, l] of mine(t.adds)) {
    expect(mine(t.removes).some(([rn, rl]) => rn === n && rl === l)).toBe(true);
  }
  expect(mine(t.removes).length).toBe(mine(t.adds).length);
}

afterEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
  closeWindow.mockReset();
  showOpenDialog.mockReset();
});

// ---------------------------------------------------------------- File
const FILE_OPEN_EVENTS = [
  "menu-open-bitstream",
  "menu-open-as-av1",
  "menu-open-as-hevc",
  "menu-open-as-avc",
  "menu-open-as-vp9",
  "menu-open-as-vvc",
  "menu-open-as-mpeg2",
];
const FILE_EVENTS = [
  "menu-open-recent-file",
  ...FILE_OPEN_EVENTS,
  "menu-open-dependent",
  "menu-close-bitstream",
  "menu-quit",
  "menu-keyboard-shortcuts",
  "menu-export",
  "menu-export-evidence",
  "menu-open-debug-yuv",
];

function fileParams() {
  return {
    openFileAtPath: vi.fn(),
    handleOpenFile: vi.fn(),
    handleCloseFile: vi.fn(),
    handleOpenDependentFile: vi.fn(),
    exportEvidenceBundle: vi.fn(),
    setShowExportDialog: vi.fn(),
    setShowShortcuts: vi.fn(),
    setPendingYuvPath: vi.fn(),
  };
}

describe("useFileMenuEvents", () => {
  it("routes each event to its handler", () => {
    const p = fileParams();
    renderHook(() => useFileMenuEvents(p));
    for (const e of FILE_OPEN_EVENTS) fire(e);
    expect(p.handleOpenFile).toHaveBeenCalledTimes(FILE_OPEN_EVENTS.length);
    fire("menu-open-dependent");
    expect(p.handleOpenDependentFile).toHaveBeenCalledTimes(1);
    fire("menu-close-bitstream");
    expect(p.handleCloseFile).toHaveBeenCalledTimes(1);
    fire("menu-quit");
    expect(closeWindow).toHaveBeenCalledTimes(1);
    fire("menu-keyboard-shortcuts");
    expect(p.setShowShortcuts).toHaveBeenCalledWith(true);
    fire("menu-export");
    expect(p.setShowExportDialog).toHaveBeenCalledWith(true);
    fire("menu-export-evidence");
    expect(p.exportEvidenceBundle).toHaveBeenCalledTimes(1);
    expect(p.openFileAtPath).not.toHaveBeenCalled();
  });

  it("open-recent passes the detail path and ignores an empty one", () => {
    const p = fileParams();
    renderHook(() => useFileMenuEvents(p));
    fire("menu-open-recent-file", "/a/b.ivf");
    expect(p.openFileAtPath).toHaveBeenCalledWith("/a/b.ivf");
    fire("menu-open-recent-file", "");
    fire("menu-open-recent-file");
    expect(p.openFileAtPath).toHaveBeenCalledTimes(1);
  });

  it("open-debug-yuv sets the pending path, or nothing when cancelled", async () => {
    const p = fileParams();
    renderHook(() => useFileMenuEvents(p));
    showOpenDialog.mockResolvedValueOnce("/x.yuv");
    fire("menu-open-debug-yuv");
    await vi.waitFor(() =>
      expect(p.setPendingYuvPath).toHaveBeenCalledWith("/x.yuv"),
    );
    expect(showOpenDialog).toHaveBeenCalledWith([
      { name: "Raw YUV", extensions: ["yuv", "raw", "y4m"] },
    ]);
    showOpenDialog.mockResolvedValueOnce(null);
    fire("menu-open-debug-yuv");
    await vi.waitFor(() => expect(showOpenDialog).toHaveBeenCalledTimes(2));
    expect(p.setPendingYuvPath).toHaveBeenCalledTimes(1);
  });

  it("calls the latest handlers after rerender, not the old ones", () => {
    const a = fileParams();
    const b = fileParams();
    const { rerender } = renderHook(({ p }) => useFileMenuEvents(p), {
      initialProps: { p: a },
    });
    rerender({ p: b });
    fire("menu-open-bitstream");
    fire("menu-close-bitstream");
    fire("menu-open-recent-file", "/p");
    expect(a.handleOpenFile).not.toHaveBeenCalled();
    expect(a.handleCloseFile).not.toHaveBeenCalled();
    expect(a.openFileAtPath).not.toHaveBeenCalled();
    expect(b.handleOpenFile).toHaveBeenCalledTimes(1);
    expect(b.handleCloseFile).toHaveBeenCalledTimes(1);
    expect(b.openFileAtPath).toHaveBeenCalledWith("/p");
  });

  it("nothing fires after unmount and every listener is removed", () => {
    const t = trackListeners();
    const p = fileParams();
    const { unmount } = renderHook(() => useFileMenuEvents(p));
    unmount();
    for (const e of FILE_EVENTS) fire(e, "/p");
    expect(p.handleOpenFile).not.toHaveBeenCalled();
    expect(p.openFileAtPath).not.toHaveBeenCalled();
    expect(closeWindow).not.toHaveBeenCalled();
    expectBalanced(t, FILE_EVENTS);
    t.restore();
  });
});

// ------------------------------------------------------------- Options
const OPTION_EVENTS = [
  "menu-cpu-avx2",
  "menu-codec-hevc-ext",
  "menu-codec-hevc-index",
  "menu-codec-hevc-mv",
  "menu-codec-vvc-dynamic",
  "menu-codec-vvc-details",
  "menu-digest-force",
  "menu-digest-none",
  "menu-digest-stream",
  "menu-auto-save-layout",
];
const FORWARDS: Record<string, string> = {
  "menu-codec-hevc-ext": "codec-hevc-extensions",
  "menu-codec-hevc-index": "codec-hevc-index",
  "menu-codec-hevc-mv": "codec-hevc-mv",
  "menu-codec-vvc-dynamic": "codec-vvc-dynamic",
  "menu-codec-vvc-details": "codec-vvc-details",
  "menu-digest-force": "codec-digest-force",
  "menu-digest-none": "codec-digest-none",
  "menu-digest-stream": "codec-digest-stream",
};

describe("useOptionsMenuEvents", () => {
  it("forwards codec/digest events under their codec-* names", () => {
    renderHook(() => useOptionsMenuEvents());
    for (const [menu, fwd] of Object.entries(FORWARDS)) {
      const spy = vi.fn();
      window.addEventListener(fwd, spy);
      fire(menu);
      window.removeEventListener(fwd, spy);
      expect(spy, menu).toHaveBeenCalledTimes(1);
    }
  });

  it("toggles the localStorage flags for avx2 and auto-save-layout", () => {
    renderHook(() => useOptionsMenuEvents());
    fire("menu-cpu-avx2");
    expect(localStorage.getItem("bitvue:cpu-avx2")).toBe("true");
    fire("menu-cpu-avx2");
    expect(localStorage.getItem("bitvue:cpu-avx2")).toBe("false");
    fire("menu-auto-save-layout");
    expect(localStorage.getItem("bitvue:auto-save-layout")).toBe("true");
    expect(localStorage.getItem("bitvue:cpu-avx2")).toBe("false");
  });

  it("nothing fires after unmount and every listener is removed", () => {
    const t = trackListeners();
    const { unmount } = renderHook(() => useOptionsMenuEvents());
    unmount();
    fire("menu-cpu-avx2");
    fire("menu-auto-save-layout");
    expect(localStorage.getItem("bitvue:cpu-avx2")).toBeNull();
    expect(localStorage.getItem("bitvue:auto-save-layout")).toBeNull();
    expectBalanced(t, OPTION_EVENTS);
    t.restore();
  });
});

// -------------------------------------------------------------- Layout
describe("useLayoutMenuEvents", () => {
  const LAYOUT_EVENTS = [
    "menu-save-layout",
    "menu-load-layout",
    "menu-reset-layout",
  ];
  it("routes save/load/reset", () => {
    const [s, l, r] = [vi.fn(), vi.fn(), vi.fn()];
    renderHook(() => useLayoutMenuEvents(s, l, r));
    fire("menu-save-layout");
    expect([s, l, r].map((f) => f.mock.calls.length)).toEqual([1, 0, 0]);
    fire("menu-load-layout");
    fire("menu-reset-layout");
    expect([s, l, r].map((f) => f.mock.calls.length)).toEqual([1, 1, 1]);
  });
  it("calls the latest handlers after rerender", () => {
    const a = [vi.fn(), vi.fn(), vi.fn()] as const;
    const b = [vi.fn(), vi.fn(), vi.fn()] as const;
    const { rerender } = renderHook(
      ({ h }) => useLayoutMenuEvents(h[0], h[1], h[2]),
      { initialProps: { h: a } },
    );
    rerender({ h: b });
    fire("menu-save-layout");
    fire("menu-load-layout");
    fire("menu-reset-layout");
    for (const f of a) expect(f).not.toHaveBeenCalled();
    for (const f of b) expect(f).toHaveBeenCalledTimes(1);
  });
  it("nothing fires after unmount and every listener is removed", () => {
    const t = trackListeners();
    const s = vi.fn();
    const { unmount } = renderHook(() => useLayoutMenuEvents(s, s, s));
    unmount();
    for (const e of LAYOUT_EVENTS) fire(e);
    expect(s).not.toHaveBeenCalled();
    expectBalanced(t, LAYOUT_EVENTS);
    t.restore();
  });
});

// ------------------------------------------------------------- Overlay
describe("useOverlayMenuEvents", () => {
  const OVERLAY_EVENTS = [
    "menu-toggle-overlay",
    "menu-clear-overlays",
    "viewer-toggle-overlay-fkey",
  ];
  const overlays = [
    { mode: "heat-map" },
    { mode: "block-type" },
  ] as unknown as Parameters<typeof useOverlayMenuEvents>[2];

  it("toggle passes detail mode; empty detail ignored; clear calls clear", () => {
    const toggle = vi.fn();
    const clear = vi.fn();
    renderHook(() => useOverlayMenuEvents(toggle, clear, overlays));
    fire("menu-toggle-overlay", "qp-map");
    expect(toggle).toHaveBeenCalledWith("qp-map");
    fire("menu-toggle-overlay", "");
    fire("menu-toggle-overlay");
    expect(toggle).toHaveBeenCalledTimes(1);
    fire("menu-clear-overlays");
    expect(clear).toHaveBeenCalledTimes(1);
  });

  it("F-key maps 1-based index into availableOverlays, out of range is a no-op", () => {
    const toggle = vi.fn();
    renderHook(() => useOverlayMenuEvents(toggle, vi.fn(), overlays));
    fire("viewer-toggle-overlay-fkey", 2);
    expect(toggle).toHaveBeenLastCalledWith("block-type");
    fire("viewer-toggle-overlay-fkey", 1);
    expect(toggle).toHaveBeenLastCalledWith("heat-map");
    fire("viewer-toggle-overlay-fkey", 5);
    fire("viewer-toggle-overlay-fkey", 0);
    expect(toggle).toHaveBeenCalledTimes(2);
  });

  it("uses the latest toggle and overlay list after rerender", () => {
    const t1 = vi.fn();
    const t2 = vi.fn();
    const o2 = [{ mode: "psnr-overlay" }] as unknown as typeof overlays;
    const { rerender } = renderHook(
      ({ t, o }) => useOverlayMenuEvents(t, vi.fn(), o),
      { initialProps: { t: t1, o: overlays } },
    );
    rerender({ t: t2, o: o2 });
    fire("viewer-toggle-overlay-fkey", 1);
    fire("menu-toggle-overlay", "m");
    expect(t1).not.toHaveBeenCalled();
    expect(t2.mock.calls).toEqual([["psnr-overlay"], ["m"]]);
  });

  it("nothing fires after unmount and every listener is removed", () => {
    const t = trackListeners();
    const toggle = vi.fn();
    const clear = vi.fn();
    const { unmount } = renderHook(() =>
      useOverlayMenuEvents(toggle, clear, overlays),
    );
    unmount();
    fire("menu-toggle-overlay", "m");
    fire("menu-clear-overlays");
    fire("viewer-toggle-overlay-fkey", 1);
    expect(toggle).not.toHaveBeenCalled();
    expect(clear).not.toHaveBeenCalled();
    expectBalanced(t, OVERLAY_EVENTS);
    t.restore();
  });
});

// ---------------------------------------------------------------- Mode
describe("useModeMenuEvents", () => {
  it("passes string detail, ignores non-string/empty; latest setMode wins", () => {
    const a = vi.fn();
    const b = vi.fn();
    const { rerender } = renderHook(({ f }) => useModeMenuEvents(f), {
      initialProps: { f: a },
    });
    rerender({ f: b });
    fire("menu-mode-change", "residuals");
    fire("menu-mode-change", 3);
    fire("menu-mode-change", "");
    fire("menu-mode-change");
    expect(a).not.toHaveBeenCalled();
    expect(b.mock.calls).toEqual([["residuals"]]);
  });
  it("nothing fires after unmount and every listener is removed", () => {
    const t = trackListeners();
    const f = vi.fn();
    const { unmount } = renderHook(() => useModeMenuEvents(f));
    unmount();
    fire("menu-mode-change", "overview");
    expect(f).not.toHaveBeenCalled();
    expectBalanced(t, ["menu-mode-change"]);
    t.restore();
  });
});

// --------------------------------------------------------------- Theme
describe("useThemeMenuEvents", () => {
  it("accepts only dark/light; latest setTheme wins", () => {
    const a = vi.fn();
    const b = vi.fn();
    const { rerender } = renderHook(({ f }) => useThemeMenuEvents(f), {
      initialProps: { f: a },
    });
    rerender({ f: b });
    fire("menu-theme-change", "dark");
    fire("menu-theme-change", "light");
    fire("menu-theme-change", "blue");
    fire("menu-theme-change");
    expect(a).not.toHaveBeenCalled();
    expect(b.mock.calls).toEqual([["dark"], ["light"]]);
  });
  it("nothing fires after unmount and every listener is removed", () => {
    const t = trackListeners();
    const f = vi.fn();
    const { unmount } = renderHook(() => useThemeMenuEvents(f));
    unmount();
    fire("menu-theme-change", "dark");
    expect(f).not.toHaveBeenCalled();
    expectBalanced(t, ["menu-theme-change"]);
    t.restore();
  });
});
