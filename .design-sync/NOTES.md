# Bitvue design-sync notes

## Repo shape

- No library build (`frontend/package.json` has no `main`/`module`/`exports`) — this is an Electron
  renderer app, not a published component package. The converter runs in **synth-entry mode**
  (scans `frontend/components/**/*.tsx` directly via ts-morph; no real `dist/` component bundle).
- `cssEntry` (`.design-sync-dist.css`) is NOT hand-authored — it's the real Vite app build output
  (`npm run build`'s `dist/assets/*.css`, concatenated). `buildCmd` regenerates it. Re-sync must
  re-run `buildCmd` before the converter (hashed filenames change every build).
- Providers: `cfg.provider` nests the same 7 providers `frontend/test/test-utils.tsx`'s
  `AllTheProviders` wraps every test with (Theme→Layout→FrameData→FileState→CurrentFrame→
  Selection→Mode). Sourced via `extraEntries` pointing at `contexts/*.tsx` since they're not under
  `components/`.
- Since the design system doesn't need real decoded video, components that call
  `electronBridgeService`/`window.bitvue` get **mock data authored directly in their preview
  `.tsx`** (same pattern this repo's own vitest suite uses — see
  `frontend/tests/components/AV1FeaturesView.test.tsx` for the reference shape), not real IPC.

## Known gotchas (componentSrcMap exclusions)

- **`StatusBar`**: two unrelated components share this exact name —
  `components/StatusBar.tsx` (app-level) and `components/panels/YuvViewerPanel/StatusBar.tsx`
  (panel-local). Synth-entry does `export * from <every src file>` into one flat namespace; an
  ambiguous named-export collision resolves to `undefined` in ESM, which fails `[BUNDLE_EXPORT]`.
  Excluded via `componentSrcMap: {"StatusBar": null}` — can't be synced as-is without renaming one
  in the real source, which is out of scope for this sync.
- **`MemoizedPlaceholderPanel`**: `components/panels/PlaceholderPanel.tsx` does
  `const MemoizedPlaceholderPanel = memo(PlaceholderPanel); export { MemoizedPlaceholderPanel as
  PlaceholderPanel };` — ts-morph discovery picked up the pre-alias internal declaration name, but
  the bundle only actually exports the alias target `PlaceholderPanel`. Excluded the bogus duplicate
  via `componentSrcMap: {"MemoizedPlaceholderPanel": null}`; `PlaceholderPanel` itself syncs fine.

## Fonts — accepted substitutes (user not asked per-family; these are OS system-font fallback
stacks, not brand fonts)

`--font-family-mono`/ad-hoc component fonts reference `"SF Mono"`, `"Monaco"`, `"Inconsolata"`,
`"Fira Code"`, `"Cascadia Code"`, `"Consolas"`, `"SF Pro Text"` as a monospace/UI fallback chain
(`theme/colors.css`, `ErrorBoundary.css`, `DetailsPanel.css`, `InfoPanel.css`). None are shipped by
the app itself — they rely on whatever's installed on the OS (several are Apple-proprietary and
can't legally be redistributed as web fonts anyway). Accepted as substitutes; the DS pane renders
these with system fallback fonts. Codicon (`public/fonts/codicon.ttf`) IS shipped and IS wired via
`cfg.extraFonts` — that one's a real icon font, not a fallback stack.

## Re-sync risks (things that can silently go stale)

- `.design-sync-dist.css` is a build artifact — a re-sync that forgets to re-run `buildCmd` first
  syncs stale/missing CSS. `resync.mjs`'s driver doesn't know to run it automatically; run it by
  hand (or check `cfg.buildCmd` is being executed) before invoking the driver.
- Mock data authored into preview `.tsx` files (for anything touching `electronBridgeService`) will
  silently drift from the real wire types (`frontend/services/electronBridgeService.ts`) if those
  types change and the preview isn't updated — no compiler catches this since previews aren't part
  of the app's own `tsc` build.
- `componentSrcMap` exclusions above are current as of this sync; if `StatusBar` or
  `PlaceholderPanel` get renamed/refactored in the real app, re-check whether the collision still
  applies.

## Authored previews (31 components, first design-only pass)

31 components rendered visually blank (real DOM content present but invisible) because the
generated preview card boilerplate hardcodes `body{background:#fff}` (a white page) while Bitvue's
own near-white text/dark chrome (`rgb(255 255 255 / 90%)` etc.) assumes it's sitting inside the
app's dark shell. Fixed by wrapping every authored preview's JSX in a dark background
(`#1e1e1e` by default; see per-component overrides below), never by touching app source.
Full list: `EmptyState` + its 8 presets (`NoFileLoaded`/`NoFramesFound`/`NoFramesSelected`/
`NoReferenceFrames`/`NoResultsFound`/`NoSearchResults`/`NoSelection`/`PanelEmptyState`),
`PanelBase` + its 4 siblings (`PanelSection`/`PanelInfoRow`/`PanelEmpty`/`PanelLoading`),
`Skeleton`, `DebugPanelToggle`, `WarningBanner`, `FilmstripDropdown`, `TimelineCursor`,
`TimelineHeader`, `DetailsPanel`, `InfoPanel`, `ApsTab`/`FrameSyntaxTab`/`ProbsTab`/`QmTab`/
`RefListTab`/`ReferencesTab`, `DpbViewTab`/`FrameViewTab`, `ZoomControls`.

- **`ApsTab`/`ProbsTab`/`QmTab`/`FrameSyntaxTab`/`RefListTab`**: these don't take data via props —
  they fetch internally via `electronBridgeService`/`window.bitvue`. Previews mock
  `window.bitvue.<method>` at module scope with realistic wire-shaped responses (matching the real
  `*WireResult` TS interfaces), same approach as this repo's own vitest suite. **`RefListTab`'s
  `getCodecExtendedInfo()` helper is a plain (non-async) function** — a missing `window.bitvue`
  throws *synchronously* inside its effect rather than rejecting a promise, so mocking the bridge
  is required, not optional (a `.catch()`-only approach won't render anything).
- **`RefListTab` mock must branch on `frameIndex`**: the first draft returned one static
  `getCodecExtendedInfo()` response regardless of which frame was requested, so the
  `PFrameSingleList` story (meant to show only an L0 list, no L1) rendered identically to
  `BFrameBothLists` — caught by the validator's `[RENDER_THIN]` "variants render identically"
  check, not by eyeballing alone. Fixed by branching the mock's `l1_refs` on the requested
  `frameIndex`. **Lesson**: when mocking a per-argument bridge call across multiple stories, vary
  the mock's response by the actual argument passed, don't return one shared fixture.
- **`DebugPanelToggle` contrast**: its real CSS (`bg: rgb(30 30 30 / 95%)`, border
  `rgb(255 255 255 / 10%)`) is intentionally a subtle, small (24×80px) edge-docked tab in the real
  app — that's by design, not a bug. Two things made it look broken in isolation: (1) a wrapper
  background too close to the button's own near-black background (first attempt used `#050505`,
  which if anything made it worse), and (2) the button (`position: fixed`) getting lost in a large
  empty wrapper. Fixed with a lighter `#3a3a3a` wrapper close to the button's own footprint —
  `cfg.overrides.DebugPanelToggle: {"cardMode": "single", "primaryStory": "Open"}` in
  `config.json` (flagged by `[GRID_OVERFLOW]`; the `Open` story additionally needs a *wider*
  wrapper than `Closed` since `.debug-toggle.open` shifts to `right: 320px`).
- **`TimelineCursor`**: also needed `cfg.overrides.TimelineCursor: {"cardMode": "column"}`
  (`[GRID_OVERFLOW]` — stories render wider than a grid cell).

## Regrouping (fixing the "everything dumped in general" problem)

The default group is derived from each component's folder path (`components/<subdir>/<Name>.tsx` →
group `<subdir>`; top-level files with no subdir → `general`). 61 of 128 components live directly
under `frontend/components/` with no subdir, so they all landed in one `general` bucket — a real
review-difficulty problem the user flagged. Fixed via `cfg.docsMap` stub files: each
`.design-sync/category-stubs/<Name>.md` is a 2-line frontmatter-only stub (`---\ncategory:
<Group>\n---`), and `cfg.docsMap.<Name>` points at it — the converter reads `category` from the
matched doc and uses it as the component's group.

**Guard discovered**: `package-build.mjs` only lets a doc's `category` override the group when the
component's auto-derived group is *already* `general`/`misc`/empty (see the `categoryApplied`
logic around the `ingestDoc` loop) — it will NOT relabel a component that already has a real,
non-generic folder-derived group. So `common` (`EmptyState`/`FrameTypeBadge`/`TabContainer`,
3 items) and the already-organized `panels`/`renderers`/`syntaxdetailpanel`/`unithexpanel`/
`yuvviewerpanel`/`views`/`charts` groups keep their original (lowercase, lightly cryptic) labels
and membership no matter what `docsMap` says — this is intentional tool behavior, not a bug to
route around. Final scheme: the 61 `general` components split into 7 new groups (`Timeline &
Filmstrip`, `Codec Overlays`, `Dialogs`, `Toolbars`, `Feedback & Empty States`, `App Shell`, `Tabs &
Badges` — folder-slugified on disk, e.g. `timeline-filmstrip`); everything else unchanged.
`docsMap` now covers all 128 components (previously 0 had a matched doc — synthesized `.prompt.md`
from `.d.ts`/JSDoc only), so every `.prompt.md` was re-generated in this pass too.

**Re-sync note**: `.design-sync/category-stubs/*.md` (128 files) is a durable, committed input —
treat it like `docsMap` itself; don't delete without also removing the corresponding `docsMap`
entries in `config.json`.

## Remaining 67 floor cards authored (second design-only pass)

Split across two parallel passes: 26 canvas-overlay/renderer components (`frontend/components/panels/
OverlayRenderer/renderers/*.tsx` — imperative `(props) => void` canvas-draw functions, not JSX
components; each preview wraps them in a small local `CanvasStage` that grabs a 2D context via
`useLayoutEffect` and invokes the function directly) + 41 remaining UI components (dialogs, timeline/
filmstrip, app shell, panel sub-tabs), all following the same `"bitvue"`-import / dark-background-
wrapper / `window.bitvue`-mock conventions as the first 31.

- **`YuvDiffPanel`/`YuvViewerPanel` needed `YuvDiffProvider`**: both call `useYuvDiff()`
  unconditionally; `YuvDiffContext.tsx` wasn't in `cfg.extraEntries` or the global `cfg.provider`
  chain, so every render crashed with "must be used within provider". Fixed by adding
  `./contexts/YuvDiffContext.tsx` to `extraEntries` and nesting `YuvDiffProvider` innermost in the
  provider chain (after `ModeProvider`) — it takes no props, so this is a safe global addition.
- **`FrameNavigationToolbar` renders `null` by design, left as-is**: it reads
  `useFrameData()`/`useCurrentFrame()` (zero props) and returns `null` when `frames.length === 0`.
  The DS harness's auto-wrapped providers always seed empty defaults with no per-story way to
  inject data (`FrameDataProvider`/`CurrentFrameProvider` take only `{children}`, no seed-data
  prop) — this is the correct empty-state behavior for a frame-navigation toolbar with no
  frames, not a bug. Not chased further.
- **`ErrorBoundary`'s `CaughtErrorFallback` story deliberately triggers `[RENDER_ERRORS]`**: the
  story intentionally throws (to demonstrate the real error-boundary fallback UI — "Something went
  wrong" / Try Again / Reload Page). React still logs the caught error to console even though
  `componentDidCatch` handles it, which `package-validate.mjs` counts as a page error (`errs: 2`,
  `bad: true`). Confirmed via screenshot the fallback UI renders exactly as intended. Accepted as
  benign — this is the one component intentionally exercising the "show a real error" path.
- **24 canvas-drawing components trip `[RENDER_THIN]` "variants render identically"**: all in the
  `codec-overlays`/`renderers` groups (`Av1BlockTypeOverlay`, `Av1CdefRenderer`,
  `Av1EfficiencyMapOverlay`, `Av1FilmGrainRenderer`, `Av1LoopRestorationRenderer`,
  `Av1SuperResRenderer`, `AvcMbTypeOverlay`, `AvcRefIdxOverlay`, `Avs3CcsaoRenderer`,
  `Avs3EsaoRenderer`, `CodingFlowOverlay`, `JpegXsDequantRenderer`, `JpegXsMctRenderer`,
  `JpegXsNltRenderer`, `JpegXsPrecinctRenderer`, `JpegXsTransformRenderer`, `MVFieldOverlay`,
  `PredictionOverlay`, `QPMapOverlay`, `ReferenceOverlay`, `TransformOverlay`, `Vc3SegmentRenderer`,
  `VvcAdaptiveFilterRenderer`, `VvcDualTreeRenderer`, `VvcInverseMapRenderer`, plus `YuvDiffPanel`).
  The check measures DOM text/height, which can't see canvas pixel differences — spot-checked
  `Av1CdefRenderer` and `QPMapOverlay` screenshots directly and confirmed the two stories paint
  visibly different content (different arrow directions/colors, different QP heatmaps). False
  positive, confirmed benign, not reworked.
- **48 components needed `cfg.overrides.<Name>.cardMode`** (`[GRID_OVERFLOW]`): `"column"` (full
  card width, all stories kept) for components whose stories render wider than a grid cell —
  mostly the canvas renderers/overlays (fixed pixel width) plus `Tooltip`/`HRDBufferPanel`/
  `YuvDiffPanel`/`YuvViewerPanel`/`Graph`/`TimelineTooltip`/`VirtualizedFilmstrip`/`HexViewTab`/
  `VideoCanvas`. `"single"` + `primaryStory` (only the first story shown, others still authored but
  not on the card) for `position: fixed`/portal-style components whose content escapes any grid
  cell entirely — `ContextMenu`, `ErrorDialog`, `ExportDialog`, `GoToFrameDialog`,
  `KeyboardShortcutsDialog`, `ErrorBoundary`, `CropDialog`, `LoadDebugYuvDialog`,
  `FilmstripTooltip`, `FrameSizesLegend`, `DebugPanel`.

**Result of this pass**: all 128 components authored (98 with real authored previews + 30 that
render correctly with zero authoring), 127/128 clean per `package-validate.mjs`, 1 (`ErrorBoundary`)
intentionally/correctly flagged and accepted as benign above.

## NOT fixed as part of this sync (found, out of scope)

- **Real bug, unrelated to design-sync**: `components/BookmarksPanel.css` and
  `components/panels/YuvViewerPanel/YuvViewerPanel.css` reference `var(--vscode-sidebar-background)`,
  `var(--vscode-foreground)`, `var(--vscode-button-secondaryBackground)`, and similar
  `--vscode-*` custom properties. Bitvue is a plain Electron app, not a VS Code webview — nothing in
  `theme/*.css` ever defines these, so in the real running app these rules currently resolve to
  unset/inherited values (likely a visible background/color bug). Flagged to the user during this
  sync; not fixed here (out of scope for a design-catalog sync). Worth a follow-up pass.
