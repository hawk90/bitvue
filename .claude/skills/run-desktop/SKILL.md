---
name: run-desktop
description: Launch the real Bitvue Electron desktop app and capture screenshots of a given state (open a clip, switch mode, toggle overlay, click a tab) to check a change visually. Use when a change touches anything the user sees (overlays, modes, panels, indexer/sidecar output) or when asked to run the app or verify parity visually.
allowed-tools: Read, Grep, Bash
---

# Run the desktop app and look at it

Tests do not show what is on screen. The app has a built-in screenshot mode (`bitvue-desktop/electron/main.ts`,
`runScreenshotAndExit`): it opens a fixture through the real open-file path, applies the hooks below, saves a PNG
and exits. **Read the PNG** (Read tool) -- a blank or wrong frame is a failure.

## Launch

```bash
cargo build -p bitvue-sidecar                       # sidecar the app spawns (target/debug/bitvue-sidecar)
cd bitvue-desktop && npm run build:electron         # tsc + copy; frontend/dist must exist (cd frontend && npm run build)
BITVUE_ELECTRON_OFFSCREEN=1 \                       # always: see Gotchas (a visible window gets your real mouse/keys)
BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH=$PWD/../test_data/av1_aomenc_testsrc2.ivf \
BITVUE_ELECTRON_SCREENSHOT=/tmp/shot.png \
  npx electron . --disable-gpu                       # exit 0 and "[screenshot] saved to" = captured
```

Window is 1280x800 (2560x1600 PNG on retina); `BITVUE_ELECTRON_WINDOW_SIZE=900x600` for the minimum size.

## Hooks (env vars, all optional, run in this order)

| Var | Does | Example |
|---|---|---|
| `..._SCREENSHOT_SKIP_OPEN=1` | no file; Welcome screen | |
| `..._SCREENSHOT_CLICK_TAB` | click buttons by visible label | `Unit HEX,Hex` |
| `..._SCREENSHOT_CLICK_SELECTOR` | click by CSS selector | `[data-frame-index="5"]` |
| `..._SCREENSHOT_OPEN_DEPENDENT=1` | open fixture again as stream B (compare) | |
| `..._SCREENSHOT_KEY` | real keydown on window | `F5`, `ctrl+F1` |
| `..._SCREENSHOT_DISPATCH` | menu-only actions (CustomEvent) | `menu-toggle-overlay:heat-map` |
| `..._SCREENSHOT_CONTEXT_MENU` / `_MOUSEMOVE` / `_CTRL_WHEEL` | right-click / hover / zoom on selector | |

`BITVUE_SIDECAR_BIN=<path>` runs a different sidecar build: copy the binary before and after a change and take the
same shot with each to compare.

Expected behavior per panel and mode, and what has been verified so far, is in `docs/USER_MANUAL.md`: compare the screenshot
against it, and fill in or correct the cell you checked.

## What exists per area

| Area | Where to look | Source of truth |
|---|---|---|
| Player modes (F-keys) | `Mode:` dropdown in the toolbar | `frontend/utils/codecModeRegistry.ts`, per codec |
| Overlays | `Overlay:` toolbar buttons | same file, `isInfoOverlay` entries |
| Frame data (type, QP, refs) | Syntax detail, Info, Stats panels | indexer / sidecar output |
| Filmstrip / timeline | top of the window | frame types and sizes |
| Compare A/B | `OPEN_DEPENDENT=1` | |

AV1 has F1 Coding Flow ... F5 Loop Filter, F6 CDEF, F10 YUV and overlays HM/BT/EFF/PSNR; **no QP Map overlay**
(#446). F5 is not QP. Do not assume a key's meaning: read the registry for the codec being checked.

## Gotchas

- **Use `BITVUE_ELECTRON_OFFSCREEN=1`.** In the default window mode the window really appears on screen, so your own
  mouse and keyboard land in it: hover tooltips get captured and the current frame can change by itself (#447). With
  offscreen the same shot came out byte-identical every time. Still take each shot twice and compare sizes.
- **`--disable-gpu` shows the picture.** With WebGPU active the decoded picture is drawn on a WebGPU canvas under the 2D canvas, and
  screenshot capture does not include it: the player is black while overlays still show (#456). `--disable-gpu` makes WebGPU fail
  (`Failed to create WebGPU Context Provider`) and the app draws the picture on the 2D canvas, so it appears in the capture.
- **Moving to a frame:** `KEY=ArrowRight,ArrowRight,...` works (`KEY=ArrowRight,ArrowRight,F2` = frame 2 in Predictions). Clicking a thumbnail
  works too, but the selector must be narrow: `data-frame-index` also exists on the timeline cells, and `CLICK_SELECTOR` uses the first match,
  so use `.filmstrip-frame[data-frame-index="N"]`.
- **Clicks carry no coordinates** (`CLICK_SELECTOR` sends `mousedown` and `click` only), so a position inside a canvas (selecting a block)
  cannot be clicked (#490). `MOUSEMOVE` and `CONTEXT_MENU` do use the element's center.
- **Tab labels are the DOM text, not what you see:** CSS shows `REFS` but the text is `Refs`, so use
  `CLICK_TAB=Refs` / `Stats` / `Search` / `Frame`. A wrong label aborts the capture ("tab not found").
- A clip the decoder refuses (for example a stream with its sequence header removed) opens with every frame
  failing to load; the screenshot proves nothing about data in that case.
- Native menu items are not DOM elements: use `DISPATCH`, not `CLICK_TAB`.
- Only one instance at a time (single-instance lock); close a running app first.
