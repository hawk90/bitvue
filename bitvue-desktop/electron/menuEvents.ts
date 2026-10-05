/**
 * Menu → renderer event channel names, shared by BOTH sides of the menu wiring:
 *
 * - `bitvue-desktop/electron/nativeMenu.ts` (macOS native menu) dispatches them into the renderer
 *   as `window` CustomEvents.
 * - `frontend/` (App.tsx / hooks / YuvViewerPanel listeners, and the Windows/Linux custom
 *   `TitleBar.tsx`, which dispatches the same events) imports this file directly via a relative
 *   path (`../../bitvue-desktop/electron/menuEvents`).
 *
 * Before this file existed the names were hand-typed string literals on each side and drifted:
 * the mac Help menu sent `"menu-keyboard-shortcuts"` while the renderer listened for
 * `"menu-shortcuts"`, and the mac Mode menu sent `"menu-mode-change"` with no listener at all
 * (and with values like `"coding"`/`"qp"` that aren't `VisualizationMode` keys).
 *
 * Keep this file dependency-free (no imports): it is compiled by two different TypeScript
 * configs (NodeNext for Electron main, bundler/Vite for the renderer).
 */

export const MENU_EVENTS = {
  /** detail: a `VisualizationMode` key (see `MENU_MODES`). */
  modeChange: "menu-mode-change",
  keyboardShortcuts: "menu-keyboard-shortcuts",
  /** detail: `"dark" | "light"`. */
  themeChange: "menu-theme-change",
  colorBt601: "menu-color-bt601",
  colorBt709: "menu-color-bt709",
  colorBt2020: "menu-color-bt2020",
  colorYuvAsRgb: "menu-color-yuv-rgb",
  colorYuvAsGbr: "menu-color-yuv-gbr",
} as const;

export type MenuEventName = (typeof MENU_EVENTS)[keyof typeof MENU_EVENTS];

/**
 * Mode menu entries (key = `VisualizationMode` string, as consumed by `ModeContext.setMode`).
 * Mirrors `frontend/contexts/ModeContext.tsx`'s `MODES` (which the Windows/Linux TitleBar Mode
 * menu is generated from); `frontend/tests/utils/menuEvents.test.ts` fails if they drift apart.
 */
export const MENU_MODES: readonly { key: string; label: string }[] = [
  { key: "overview", label: "Overview" },
  { key: "coding-flow", label: "Coding Flow" },
  { key: "prediction", label: "Prediction" },
  { key: "transform", label: "Transform" },
  { key: "qp-map", label: "QP Map" },
  { key: "mv-field", label: "MV Field" },
  { key: "reference", label: "Reference Frames" },
  { key: "deblocking", label: "Deblocking" },
  { key: "residuals", label: "Residuals" },
  { key: "av1-features", label: "AV1 Features" },
];

/** Options > Color Space entries, in menu order (`null` = separator). */
export const MENU_COLOR_SPACES: readonly ({
  label: string;
  event: MenuEventName;
} | null)[] = [
  { label: "ITU Rec. 601", event: MENU_EVENTS.colorBt601 },
  { label: "ITU Rec. 709", event: MENU_EVENTS.colorBt709 },
  { label: "ITU Rec. 2020", event: MENU_EVENTS.colorBt2020 },
  null,
  { label: "YUV as RGB", event: MENU_EVENTS.colorYuvAsRgb },
  { label: "YUV as GBR", event: MENU_EVENTS.colorYuvAsGbr },
];
